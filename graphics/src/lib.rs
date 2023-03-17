
use crate::{
    resources::gpu_image::GpuImageArrayManager,
    pipelines::{
        clear::{
            ClearPipelineCreator,
            ClearPipeline,
        },
        clear_depth::ClearDepthPipeline,
        clip::{
            ClipPipeline,
            PreppedClipEdit,
            CLIP_FORMAT,
        },
        solid::SolidPipeline,
        line::LinePipeline,
        image::{
            ImagePipeline,
            PreppedDrawImage,
        },
        text::{
            TextPipeline,
            PreppedDrawText,
        },
        mesh::MeshPipeline,
    },
    std140::{
        Std140,
        std140_struct,
    },
    frame_content::{
        FrameContent,
        TextBlock,
        LayedOutTextBlock,
        FontId,
        DrawMesh,
        GpuImageArray,
        GpuVec,
        GpuVecElem,
    },
    render_instrs::{
        frame_render_compiler,
        RenderInstr,
        DrawObjNorm,
    },
    uniform_buffer::UniformBuffer,
};
use std::{
    sync::Arc,
    borrow::Borrow,
    fmt::{self, Debug, Formatter},
};
use anyhow::Result;
use tracing::*;
use winit::window::Window;
use wgpu::*;
use vek::*;
use image::DynamicImage;
use glyph_brush::ab_glyph::FontArc;
use opentype437::Font437;


mod resources;
mod pipelines;
mod std140;
mod shader;
mod vertex;
pub mod modifier;
pub mod view_proj;
pub mod frame_content;
mod render_instrs;
mod uniform_buffer;


//const SWAPCHAIN_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm; TODO why can't it be this?
const SWAPCHAIN_FORMAT: TextureFormat = TextureFormat::Bgra8Unorm;
const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;


/// Top-level resource for drawing frames onto a window.
pub struct Renderer {
    surface: Surface,
    device: Arc<Device>,
    queue: Arc<Queue>,
    depth_texture: Texture,
    config: SurfaceConfiguration,
    uniform_buffer: UniformBuffer,
    modifier_uniform_bind_group_layout: BindGroupLayout,
    gpu_image_manager: GpuImageArrayManager,
    clear_color_pipeline: ClearPipeline,
    clear_clip_pipeline: ClearPipeline,
    clear_depth_pipeline: ClearDepthPipeline,
    clip_pipeline: ClipPipeline,
    solid_pipeline: SolidPipeline,
    line_pipeline: LinePipeline,
    image_pipeline: ImagePipeline,    
    text_pipeline: TextPipeline,
    mesh_pipeline: MeshPipeline,

    // safety: surface must be dropped before window
    _window: Arc<Window>,
}

impl Debug for Renderer {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Renderer")
    }
}

#[derive(Debug, Copy, Clone)]
struct ModifierUniformData {
    transform: Mat4<f32>,
    color: Rgba<f32>,
}

std140_struct!(ModifierUniformData {
    transform: Mat4<f32>,
    color: Rgba<f32>,
});

fn create_depth_texture_like(
    device: &Device,
    size: Extent2<u32>,
    label: &'static str,
    format: TextureFormat,
) -> Texture {
    device
        .create_texture(&TextureDescriptor {
            label: Some(label),
            size: Extent3d {
                width: size.w,
                height: size.h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
        })
}

impl Renderer {
    /// Create a new renderer on a given window.
    pub async fn new(window: Arc<Window>) -> Result<Self> {
        trace!("beginning initializing renderer");

        // create the instance, surface, and adapter
        trace!("creating instance");
        let size = window.inner_size();
        let size = Extent2::new(size.width, size.height);
        let instance = Instance::new(Backends::PRIMARY);
        trace!("creating surface");
        // safety: surface must be dropped before window
        let surface = unsafe { instance.create_surface(&*window) };

        trace!("creating adapter");
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| anyhow::Error::msg(
                "failed to find an appropriate adapter"
            ))?;
        info!("using backend {:?}", adapter.get_info().backend);

        // create the device and queue
        trace!("creating device and queue");
        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: None,
                    features: Features::empty(),
                    limits: Limits {
                        max_bind_groups: 5, // TODO don't do that
                        ..Default::default()
                    },
                },
                None,
            )
            .await?;
        let device = Arc::new(device);
        let queue = Arc::new(queue);

        // create the uniform buffer
        // TODO rename to uniform manager or something?
        let uniform_buffer = UniformBuffer::new(&device);
        
        // create the layout for the standard uniform bind group all object
        // drawing pipelines use for (some) modifiers
        let modifier_uniform_bind_group_layout = device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("modifier uniform bind group layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: Some(
                                // TODO factor out
                                (ModifierUniformData::SIZE as u64).try_into().unwrap()
                            ),
                        },
                        count: None,
                    },
                ],
            });

        // create the depth texture
        let depth_texture = create_depth_texture_like(
            &device,
            size,
            "depth texture",
            DEPTH_FORMAT,
        );

        // create the gpu image manager
        trace!("creating gpu image manager");
        let gpu_image_manager = GpuImageArrayManager::new(&device);

        // create the clear pipelines
        trace!("creating clear pipelines");
        let clear_pipeline_creator = ClearPipelineCreator::new(&device)?;
        let clear_color_pipeline = clear_pipeline_creator
            .create(&device, SWAPCHAIN_FORMAT);
        let clear_clip_pipeline = clear_pipeline_creator
            .create(&device, CLIP_FORMAT);

        // create the clear depth pipeline
        trace!("creating clear depth pipeline");
        let clear_depth_pipeline = ClearDepthPipeline::new(&device)?;

        // create the clip pipeline
        trace!("creating clip pipeline");
        let clip_pipeline = ClipPipeline::new(&device, size)?;

        // create the solid pipeline
        trace!("creating solid pipeline");
        let solid_pipeline = SolidPipeline::new(
            &device,
            &modifier_uniform_bind_group_layout,
            &clip_pipeline.clip_texture_bind_group_layout,
        )?;

        trace!("creating line pipeline");
        let line_pipeline = LinePipeline::new(
            &device,
            &modifier_uniform_bind_group_layout,
            &clip_pipeline.clip_texture_bind_group_layout,
        )?;
        
        // create the image pipeline
        trace!("creating image pipeline");
        let image_pipeline = ImagePipeline::new(
            &device,
            &modifier_uniform_bind_group_layout,
            &clip_pipeline.clip_texture_bind_group_layout,
            &gpu_image_manager,
        )?;
        
        // create the text pipeline
        trace!("creating text pipeline");
        let text_pipeline = TextPipeline::new(
            &device,
            &modifier_uniform_bind_group_layout,
            &clip_pipeline.clip_texture_bind_group_layout,
        )?;
        
        // create the mesh pipeline
        trace!("creating mesh pipeline");
        let mesh_pipeline = MeshPipeline::new(
            &device,
            &modifier_uniform_bind_group_layout,
            &clip_pipeline.clip_texture_bind_group_layout,
            &gpu_image_manager,
        )?;

        // set up the swapchain
        trace!("configuring swapchain");
        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: SWAPCHAIN_FORMAT,
            width: size.w,
            height: size.h,
            present_mode: PresentMode::AutoVsync,
            alpha_mode: CompositeAlphaMode::Auto,
        };
        surface.configure(&device, &config);

        // done
        trace!("done initializing renderer");
        Ok(Renderer {
            surface,
            device,
            queue,
            depth_texture,
            config,
            uniform_buffer,
            modifier_uniform_bind_group_layout,
            gpu_image_manager,
            clear_color_pipeline,
            clear_clip_pipeline,
            clear_depth_pipeline,
            clip_pipeline,
            solid_pipeline,
            line_pipeline,
            image_pipeline,
            text_pipeline,
            mesh_pipeline,
            _window: window,
        })
    }

    /// Get the underlying winit window.
    pub fn window(&self) -> &Arc<Window> {
        &self._window
    }

    /// Get the current surface physical size.
    pub fn size(&self) -> Extent2<u32> {
        Extent2::new(self.config.width, self.config.height)
    }

    pub fn aspect(&self) -> f32 {
        self.config.width as f32 / self.config.height as f32
    }

    /// Resize the surface, in reponse to a change in window size.
    pub fn resize(&mut self, size: Extent2<u32>) {
        // resize surface
        trace!("resizing surface");
        self.config.width = size.w;
        self.config.height = size.h;
        self.surface.configure(&self.device, &self.config);

        // resize depth texture
        trace!("resizing depth texture");
        self.depth_texture = create_depth_texture_like(
            &self.device,
            size,
            "depth texture",
            DEPTH_FORMAT,
        );// TODO factor out

        // resize clip pipeline
        trace!("resizing clip pipeline");
        self.clip_pipeline.resize(&self.device, size);
    }

    /// Draw a frame with the given frame content.
    ///
    /// It will be displayed on the window from <0,0> (top left corner) to
    /// <1,1> (bottom right corner). 
    pub fn draw_frame(&mut self, content: &FrameContent) -> Result<()> {
        let surface_size = self.size();

        // acquire frame to draw onto
        trace!("acquiring frame");
        let mut attempts = 0;
        let frame = loop {
            match self.surface.get_current_texture() {
                Ok(frame) => break frame,
                Err(e) => {
                    if attempts < 10 {
                        trace!(error=%e, "get_current_texture error, retrying");
                        attempts += 1;
                        self.surface.configure(&self.device, &self.config);
                    } else {
                        return Err(e.into());
                    }
                }
            }
        };
        if attempts > 0 {
            trace!("successfully recreated swap chain surface");
        }
        let color_texture = frame
            .texture
            .create_view(&TextureViewDescriptor::default());

        // compile and pre-render
        trace!("beginning pre-render");
        let mut uniform_packer = self.uniform_buffer.create_packer();
        let mut text_pre_renderer = self.text_pipeline.begin_pre_render();

        trace!("compiling and pre-rendering");

        #[derive(Debug)]
        enum PreppedRenderInstr<'a> {
            Draw {
                obj: PreppedRenderObj<'a>,
                muo: u32,
                depth: bool,
                dbg_transform: Mat4<f32>,
            },
            ClearClip,
            EditClip(PreppedClipEdit),
            ClearDepth,
        }

        #[derive(Debug)]
        enum PreppedRenderObj<'a> {
            Solid,
            Line,
            Image(PreppedDrawImage<'a>),
            Text(PreppedDrawText),
            Mesh(&'a DrawMesh<'a>),
        }

        let instrs = frame_render_compiler(&content, surface_size)
            .map(|instr| match instr {
                RenderInstr::Draw {
                    obj,
                    transform,
                    color,
                    depth,
                } => {
                    let muo = uniform_packer
                        .pack(&ModifierUniformData {
                            transform,
                            color: color.map(|n| n as f32 / 255.0),
                        });
                    let obj = match obj {
                        DrawObjNorm::Solid => PreppedRenderObj::Solid,
                        DrawObjNorm::Line => PreppedRenderObj::Line,
                        DrawObjNorm::Image(image) => PreppedRenderObj::Image(
                            ImagePipeline::pre_render(
                                image,
                                &mut uniform_packer,
                            )
                        ),
                        DrawObjNorm::Text(text) => PreppedRenderObj::Text(
                            text_pre_renderer.pre_render(text)
                        ),
                        DrawObjNorm::Mesh(mesh) => PreppedRenderObj::Mesh(
                            mesh
                        ),
                    };
                    PreppedRenderInstr::Draw {
                        obj,
                        muo,
                        depth,
                        dbg_transform: transform,
                    }
                },
                RenderInstr::ClearClip => PreppedRenderInstr::ClearClip,
                RenderInstr::EditClip(clip_edit) => {
                    let prepped_clip_edit = self.clip_pipeline
                        .pre_render(clip_edit, &mut uniform_packer);
                    PreppedRenderInstr::EditClip(prepped_clip_edit)
                }
                RenderInstr::ClearDepth => PreppedRenderInstr::ClearDepth,
            })
            .collect::<Vec<PreppedRenderInstr>>();

        trace!("finalizing pre-render");
        text_pre_renderer.finalize_pre_render(&self.device, &self.queue);

        // write uniform data to uniform buffer
        trace!("writing uniform data");
        self.uniform_buffer
            .upload(
                &uniform_packer,
                &self.device,
                &self.queue,
                &self.modifier_uniform_bind_group_layout,
                &self.clip_pipeline,
                &self.image_pipeline,
            );

        // begin encoder
        trace!("creating encoder");
        let mut encoder = self.device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: None,
            });

        // create views
        // TODO: just cache these or?
        let depth_texture = self
            .depth_texture
            .create_view(&TextureViewDescriptor::default());

        // clear the color buffer
        trace!("clearing color buffer");
        self.clear_color_pipeline
            .render(
                &mut encoder,
                &color_texture,
                Color::WHITE,
            );

        // execute pre-rendered render instructions
        for instr in instrs {
            match instr {
                PreppedRenderInstr::Draw {
                    obj,
                    muo, // TODO
                    depth,
                    dbg_transform,
                } => {
                    let depth_load_op =
                        if depth { LoadOp::Load }
                        else { LoadOp::Clear(1.0) };
                    // TODO: the possibility of pass not being used and
                    //       questions about how that works and how that
                    //       interacts with depth buffer is concerning and
                    //       complicating
                    let mut pass = encoder
                        .begin_render_pass(&RenderPassDescriptor {
                            label: Some("draw render pass"),
                            color_attachments: &[
                                Some(RenderPassColorAttachment {
                                    view: &color_texture,
                                    resolve_target: None,
                                    ops: Operations {
                                        load: LoadOp::Load,
                                        store: true,
                                    },
                                }),
                            ],
                            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                                view: &depth_texture,
                                depth_ops: Some(Operations {
                                    load: depth_load_op,
                                    store: true,
                                }),
                                stencil_ops: None,
                            }),
                        });
                    pass
                        .set_bind_group(
                            0,
                            self.uniform_buffer.unwrap_modifier_uniform_bind_group(),
                            //&self.uniform_buffer_state.as_ref().unwrap().modifier_uniform_bind_group,
                            &[muo as u32], // TODO
                        );
                    pass
                        .set_bind_group(
                            1,
                            &self.clip_pipeline.clip_min_texture.bind_group,
                            &[],
                        );
                    pass.set_bind_group(
                            2,
                            &self.clip_pipeline.clip_max_texture.bind_group,
                            &[], // TODO min/max order consistency/
                        );
                    match obj {
                        PreppedRenderObj::Solid => {
                            self.solid_pipeline
                                .render(&mut pass);
                        }
                        PreppedRenderObj::Line => {
                            self.line_pipeline
                                .render(&mut pass);
                        }
                        PreppedRenderObj::Image(image) => {
                            self.image_pipeline
                                .render(
                                    image,
                                    &mut pass,
                                    self.uniform_buffer.unwrap_image_uniform_bind_group(),
                                );
                        }
                        PreppedRenderObj::Text(text) => {
                            self.text_pipeline
                                .render(
                                    text,
                                    &mut pass,
                                );
                        }
                        PreppedRenderObj::Mesh(mesh) => {
                            self.mesh_pipeline
                                .render(
                                    mesh,
                                    &mut pass,
                                    dbg_transform,
                                );
                        }
                    }
                }
                PreppedRenderInstr::ClearClip => {
                    self.clear_clip_pipeline
                        .render(
                            &mut encoder,
                            &self.clip_pipeline.clip_min_texture.view,
                            Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 1.0,
                            },
                        );
                    self.clear_clip_pipeline
                        .render(
                            &mut encoder,
                            &self.clip_pipeline.clip_max_texture.view,
                            Color {
                                r: 1.0,
                                g: 0.0,
                                b: 0.0,
                                a: 1.0,
                            },
                        );
                }
                PreppedRenderInstr::EditClip(clip_edit) => {
                    self.clip_pipeline
                        .render(
                            clip_edit,
                            &mut encoder,
                            self.uniform_buffer.unwrap_clip_edit_uniform_bind_group(),
                        );
                }
                PreppedRenderInstr::ClearDepth => {
                    self.clear_depth_pipeline
                        .render(
                            &mut encoder,
                            &depth_texture,
                            1.0,
                        );
                }
            }
        }

        // submit, present, return
        trace!("finishing frame");
        self.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }

    /// Load a font onto the renderer from OTF / TTF / etc file data.
    ///
    /// Be mindful that there is currently no way to un-load a font from the
    /// renderer.
    pub fn load_font(&mut self, file_data: impl AsRef<[u8]>) -> Result<FontId> {
        trace!("loading font");
        let font = FontArc::try_from_vec(file_data.as_ref().into())?;
        Ok(self.text_pipeline.load_font(font))
    }

    /// Load a font onto the renderer from a code point 437 glyph atlas PNG /
    /// JPG / etc file data.
    ///
    /// Be mindful that there is currently no way to un-load a font from the
    /// renderer.
    pub fn load_font_437(&mut self, file_data: impl AsRef<[u8]>) -> Result<FontId> {
        trace!("decompressing font 437 image file data");
        let image = image::load_from_memory(file_data.as_ref())?;
        self.load_font_437_raw(image)
    }

    /// Load a font onto the renderer from a code point 437 glyph atlas which
    /// has already been decompressed.
    ///
    /// Be mindful that there is currently no way to un-load a font from the
    /// renderer.
    pub fn load_font_437_raw(&mut self, image: impl Borrow<DynamicImage>) -> Result<FontId> {
        trace!("loading font 437");
        let font = FontArc::new(Font437::new(image)?);
        Ok(self.text_pipeline.load_font(font))
    }

    /// Pre-compute the layout for a text block.
    pub fn lay_out_text(&self, text_block: &TextBlock) -> LayedOutTextBlock {
        trace!("laying out text block"); // TODO spans?
        self.text_pipeline.lay_out_text(text_block)
    }

    /// Load an array of images onto the GPU from PNG / JPG / etc file data.
    ///
    /// If `size` is None, automatically chooses dimensions as the maximum
    /// between all images, or 1 if no images are given. If any images are a
    /// different size than chosen, automatically resizes them with
    /// nearest-neighbor filtering.
    ///
    /// Panics if `size` has 0 components.
    pub fn load_image_array<I>(
        &self,
        size: Option<Extent2<u32>>,
        images: I,
    ) -> Result<GpuImageArray>
    where
        I: IntoIterator,
        <I as IntoIterator>::Item: AsRef<[u8]>,
    {
        trace!("decompressing image array file data");
        let images = images
            .into_iter()
            .map(|file_data| image::load_from_memory(file_data.as_ref())
                .map_err(anyhow::Error::from))
            .collect::<Result<Vec<DynamicImage>>>()?;
        let size = size
            .unwrap_or_else(|| Extent2 {
                w: images.iter().map(|image| image.width()).max().unwrap_or(1),
                h: images.iter().map(|image| image.height()).max().unwrap_or(1),
            });
        Ok(self
            .load_image_array_raw(
                size,
                images,
            ))
    }

    /// Load an array of images onto the GPU which have already been
    /// decompressed.
    ///
    /// If any images are a different size than `size`, automatically resizes
    /// them with nearest-neighbor filtering.
    ///
    /// Panics if `size` has 0 components.
    pub fn load_image_array_raw<I>(
        &self,
        size: Extent2<u32>,
        images: I,
    ) -> GpuImageArray
    where
        I: IntoIterator,
        <I as IntoIterator>::Item: Borrow<DynamicImage>,
    {
        trace!("loading image array");
        self
            .gpu_image_manager
            .load_image_array(
                &self.device,
                &self.queue,
                size,
                images,
            )
    }

    pub fn create_async_gpu_vec_context(&self) -> AsyncGpuVecContext {
        AsyncGpuVecContext {
            device: Arc::clone(&self.device),
            queue: Arc::clone(&self.queue),
        }
    }
}

pub trait GpuVecContext: Sized {
    /// Create an empty `GpuVec`.
    fn create_gpu_vec<T: GpuVecElem>(&self) -> GpuVec<T>;

    /// Create a `GpuVec` and initialize it with a slice of content.
    fn create_gpu_vec_init<T>(
        &self,
        content: &[T],
    ) -> GpuVec<T>
    where
        T: GpuVecElem,
    {
        trace!("async creating gpu vec with data");
        let mut gpu_vec = self.create_gpu_vec();
        let len = content.clone().into_iter().count();
        self
            .set_gpu_vec_len(
                &mut gpu_vec,
                len,
            );
        self
            .patch_gpu_vec(
                &mut gpu_vec,
                &[(
                    0,
                    content,
                )],
            );
        gpu_vec
    }

    /// Set the size of a `GpuVec`, reallocating if necessary.
    ///
    /// If this increases the size, all slots after the previous size are
    /// considered to be filled with garbage data.
    fn set_gpu_vec_len<T: GpuVecElem>(
        &self,
        gpu_vec: &mut GpuVec<T>,
        new_len: usize,
    );

    /// Patch some ranges of a `GpuVec`, overwriting existing data.
    ///
    /// Each patch comprises a destination `GpuVec` patch start index to copy
    /// to, and a slice of elements to copy to the `GpuVec` starting at that
    /// index.
    fn patch_gpu_vec<T: GpuVecElem>(
        &self,
        gpu_vec: &mut GpuVec<T>,
        patches: &[(usize, &[T])],
    );
}

impl GpuVecContext for Renderer {
    fn create_gpu_vec<T: GpuVecElem>(&self) -> GpuVec<T> {
        trace!("creating gpu vec");
        MeshPipeline::create_gpu_vec()
    }

    fn set_gpu_vec_len<T: GpuVecElem>(
        &self,
        gpu_vec: &mut GpuVec<T>,
        new_len: usize,
    ) {
        trace!("setting gpu vec len");
        MeshPipeline::set_gpu_vec_len(
            &self.device,
            &self.queue,
            gpu_vec,
            new_len,
        );
    }

    fn patch_gpu_vec<T: GpuVecElem>(
        &self,
        gpu_vec: &mut GpuVec<T>,
        patches: &[(usize, &[T])],
    ) {
        trace!("patching gpu vec");
        MeshPipeline::patch_gpu_vec(
            &self.device,
            &self.queue,
            gpu_vec,
            patches,
        );
    }
}

#[derive(Debug, Clone)]
pub struct AsyncGpuVecContext {
    device: Arc<Device>,
    queue: Arc<Queue>,
}

impl GpuVecContext for AsyncGpuVecContext {
    fn create_gpu_vec<T: GpuVecElem>(&self) -> GpuVec<T> {
        trace!("async creating gpu vec");
        MeshPipeline::create_gpu_vec()
    }

    fn set_gpu_vec_len<T: GpuVecElem>(
        &self,
        gpu_vec: &mut GpuVec<T>,
        new_len: usize,
    ) {
        trace!("async setting gpu vec len");
        let submission_index =
            MeshPipeline::set_gpu_vec_len(
                &self.device,
                &self.queue,
                gpu_vec,
                new_len,
            );
        if let Some(submission_index) = submission_index {
            self.device
                .poll(Maintain::WaitForSubmissionIndex(submission_index));
        }
    }

    fn patch_gpu_vec<T: GpuVecElem>(
        &self,
        gpu_vec: &mut GpuVec<T>,
        patches: &[(usize, &[T])],
    ) {
        trace!("async patching gpu vec");
        let submission_index =
            MeshPipeline::patch_gpu_vec(
                &self.device,
                &self.queue,
                gpu_vec,
                patches,
            );
        if let Some(submission_index) = submission_index {
            self.device
                .poll(Maintain::WaitForSubmissionIndex(submission_index));
        }
    }
}
