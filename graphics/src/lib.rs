
use crate::{
    pipelines::{
        clear::{
            ClearPipelineCreator,
            ClearPipeline,
        },
        clip::{
            ClipPipeline,
            PreppedClipEdit,
            CLIP_FORMAT,
        },
        solid::SolidPipeline,
        image::{
            ImagePipeline,
            PreppedDrawImage,
        },
        text::{
            TextPipeline,
            PreppedDrawText,
        },
    },
    std140::{
        Std140,
        pad,
        std140_struct,
    },
    frame_content::{
        FrameContent,
        GpuImage,
        TextBlock,
        LayedOutTextBlock,
        FontId,
    },
    render_instrs::{
        frame_render_compiler,
        RenderInstr,
        DrawObjNorm,
    },
    uniform_buffer::UniformBuffer,
};
use std::{
    path::Path,
    sync::Arc,
    borrow::Borrow,
};
use anyhow::Result;
use tracing::*;
use winit_main::reexports::{
    window::Window,
    dpi::PhysicalSize,
};
use wgpu::{
    *,
    util::{
        DeviceExt,
        BufferInitDescriptor,
    },
};
use vek::*;
use tokio::fs;
use image::DynamicImage;
use glyph_brush::ab_glyph::FontArc;
use opentype437::Font437;


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
    device: Device,
    queue: Queue,
    depth_texture: Texture,
    config: SurfaceConfiguration,
    uniform_buffer: UniformBuffer,
    modifier_uniform_bind_group_layout: BindGroupLayout,
    clear_color_pipeline: ClearPipeline,
    clear_clip_pipeline: ClearPipeline,
    clip_pipeline: ClipPipeline,
    solid_pipeline: SolidPipeline,
    image_pipeline: ImagePipeline,    
    text_pipeline: TextPipeline,

    // safety: surface must be dropped before window
    _window: Arc<Window>,
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

/*

pub use crate::pipelines::text::{
    TextBlock,
    HorizontalAlign,
    VerticalAlign,
    FontId,
    TextSpan,
    LayedOutTextBlock,
    pt_to_px,
};
*/

fn create_depth_texture_like(
    device: &Device,
    size: PhysicalSize<u32>,
    label: &'static str,
    format: TextureFormat,
) -> Texture {
    device
        .create_texture(&TextureDescriptor {
            label: Some(label),
            size: Extent3d {
                width: size.width,
                height: size.height,
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
            .ok_or_else(|| anyhow::Error::msg("failed to find an appropriate adapter"))?;

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

        // create the clear pipeline
        trace!("creating clear pipeline");
        let clear_pipeline_creator = ClearPipelineCreator::new(&device).await?;
        let clear_color_pipeline = clear_pipeline_creator
            .create(&device, SWAPCHAIN_FORMAT);
        let clear_clip_pipeline = clear_pipeline_creator
            .create(&device, CLIP_FORMAT);

        // create the clip pipeline
        trace!("creating clip pipeline");
        let clip_pipeline = ClipPipeline::new(&device, size).await?;

        // create the solid pipeline
        trace!("creating solid pipeline");
        let solid_pipeline = SolidPipeline::new(
            &device,
            &modifier_uniform_bind_group_layout,
            &clip_pipeline.clip_texture_bind_group_layout,
        ).await?;

        
        // create the image pipeline
        trace!("creating image pipeline");
        let image_pipeline = ImagePipeline::new(
            &device,
            &modifier_uniform_bind_group_layout,
            &clip_pipeline.clip_texture_bind_group_layout,
        ).await?;
        
        // create the text pipeline
        trace!("creating text pipeline");
        let text_pipeline = TextPipeline::new(
            &device,
            &modifier_uniform_bind_group_layout,
            &clip_pipeline.clip_texture_bind_group_layout,
        ).await?;
        
        // set up the swapchain
        trace!("configuring swapchain");
        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: SWAPCHAIN_FORMAT,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Mailbox,
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
            clear_color_pipeline,
            clear_clip_pipeline,
            clip_pipeline,
            solid_pipeline,
            image_pipeline,
            text_pipeline,
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

    /// Resize the surface, in reponse to a change in window size.
    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        // resize surface
        trace!("resizing surface");
        self.config.width = size.width;
        self.config.height = size.height;
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
    
    /// Draw a frame. The callback can draw onto the Canvas2d. Then it will be
    /// displayed on the window from <0,0> (top left corner) to <1,1> (bottom
    /// right corner).
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
            },
            ClearClip,
            EditClip(PreppedClipEdit),
        }

        #[derive(Debug)]
        enum PreppedRenderObj<'a> {
            Solid,
            Image(PreppedDrawImage<'a>),
            Text(PreppedDrawText),
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
                        DrawObjNorm::Image(image) => PreppedRenderObj::Image(
                            ImagePipeline::pre_render(
                                image,
                                &mut uniform_packer,
                            )
                        ),
                        DrawObjNorm::Text(text) => PreppedRenderObj::Text(
                            text_pre_renderer.pre_render(text)
                        ),
                    };
                    PreppedRenderInstr::Draw {
                        obj,
                        muo,
                        depth,
                    }
                },
                RenderInstr::ClearClip => PreppedRenderInstr::ClearClip,
                RenderInstr::EditClip(clip_edit) => {
                    let prepped_clip_edit = self.clip_pipeline
                        .pre_render(clip_edit, &mut uniform_packer);
                    PreppedRenderInstr::EditClip(prepped_clip_edit)
                }
                RenderInstr::ClearDepth => unimplemented!(),
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
                } => {
                    // TODO batch
                    let depth_stencil_attachment = if depth {
                        Some(RenderPassDepthStencilAttachment {
                            view: &depth_texture,
                            depth_ops: Some(Operations {
                                load: LoadOp::Load,
                                store: true,
                            }),
                            stencil_ops: None,
                        })
                    } else {
                        None
                    };
                    let mut pass = encoder
                        .begin_render_pass(&RenderPassDescriptor {
                            label: Some("draw render pass"),
                            color_attachments: &[
                                RenderPassColorAttachment {
                                    view: &color_texture,
                                    resolve_target: None,
                                    ops: Operations {
                                        load: LoadOp::Load,
                                        store: true,
                                    },
                                },
                            ],
                            depth_stencil_attachment,
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
                            self.solid_pipeline.render(&mut pass);
                        }
                        PreppedRenderObj::Image(image) => {
                            self.image_pipeline.render(
                                image,
                                &mut pass,
                                self.uniform_buffer.unwrap_image_uniform_bind_group(),
                            );
                        }
                        PreppedRenderObj::Text(text) => {
                            self.text_pipeline.render(
                                text,
                                &mut pass,
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
                    self.clip_pipeline.render(
                        clip_edit,
                        &mut encoder,
                        self.uniform_buffer.unwrap_clip_edit_uniform_bind_group(),
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
    
    /// Read a PNG / JPG / etc image from a file and load it onto the GPU.
    ///
    /// Just reads the file with tokio then passes it to `self.load_image`.
    pub async fn load_image_file(&self, path: impl AsRef<Path>) -> Result<GpuImage> {
        let file_data = fs::read(path).await?;
        self.load_image(&file_data)
    }

    /// Load an image onto the GPU from PNG / JPG / etc file data.
    pub fn load_image(&self, file_data: impl AsRef<[u8]>) -> Result<GpuImage> {
        let image = image::load_from_memory(file_data.as_ref())?;
        Ok(self.load_image_raw(image))
    }

    /// Load an image onto the GPU which has already been decompressed.
    pub fn load_image_raw(&self, image: impl Borrow<DynamicImage>) -> GpuImage {
        self.image_pipeline
            .load_image(&self.device, &self.queue, &image.borrow().to_rgba8())
    }
    
    /// Read an OTF / TTF / etc font from a file and load it onto the renderer.
    pub async fn load_font_file(&mut self, path: impl AsRef<Path>) -> Result<FontId> {
        let file_data = fs::read(path).await?;
        self.load_font(&file_data)
    }

    /// Load a font onto the renderer from OTF / TTF / etc file data.
    ///
    /// Be mindful that there is currently no way to un-load a font from the
    /// renderer.
    pub fn load_font(&mut self, file_data: impl AsRef<[u8]>) -> Result<FontId> {
        let font = FontArc::try_from_vec(file_data.as_ref().into())?;
        Ok(self.text_pipeline.load_font(font))
    }

    /// Read a PNG / JPG / etc code point 437 glyph atlas from a file and load
    /// it as a font onto the renderer.
    ///
    /// Be mindful that there is currently no way to un-load a font from the
    /// renderer.
    pub async fn load_font_437_file(&mut self, path: impl AsRef<Path>) -> Result<FontId> {
        let file_data = fs::read(path).await?;
        self.load_font_437(file_data)
    }

    /// Load a font onto the renderer from a code point 437 glyph atlas PNG /
    /// JPG / etc file data.
    ///
    /// Be mindful that there is currently no way to un-load a font from the
    /// renderer.
    pub fn load_font_437(&mut self, file_data: impl AsRef<[u8]>) -> Result<FontId> {
        let image = image::load_from_memory(file_data.as_ref())?;
        self.load_font_437_raw(image)
    }

    /// Load a font onto the renderer from a code point 437 glyph atlas which
    /// has already been decompressed.
    ///
    /// Be mindful that there is currently no way to un-load a font from the
    /// renderer.
    pub fn load_font_437_raw(&mut self, image: impl Borrow<DynamicImage>) -> Result<FontId> {
        let font = FontArc::new(Font437::new(image)?);
        Ok(self.text_pipeline.load_font(font))
    }

    /// Pre-compute the layout for a text block.
    pub fn lay_out_text(&self, text_block: &TextBlock) -> LayedOutTextBlock {
        self.text_pipeline.lay_out_text(text_block)
    }
}
