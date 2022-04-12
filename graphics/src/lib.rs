
use crate::{
    pipelines::{
        clear::ClearPipeline,
        solid::{
            SolidPipeline,
            DrawCallSolid,
            prep_draw_solid_call,
        },
        image::{
            ImagePipeline,
            DrawCallImage,
            prep_draw_image_call,
        },
        text::{
            TextPipeline,
            DrawCallText,
        },
    },
    std140::{
        Std140,
        pad,
    },
    transform2d::{
        Canvas2dTransform,
        Canvas2dUniformData,
    },
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


mod pipelines;
mod std140;
mod shader;
mod vertex;
mod transform2d;


const SWAPCHAIN_FORMAT: TextureFormat = TextureFormat::Bgra8Unorm;


/// Top-level resource for drawing frames onto a window.
pub struct Renderer {
    surface: Surface,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    uniform_buffer_state: Option<UniformBufferState>,
    canvas2d_uniform_bind_group_layout: BindGroupLayout,
    clear_pipeline: ClearPipeline,
    solid_pipeline: SolidPipeline,
    image_pipeline: ImagePipeline,    
    text_pipeline: TextPipeline,

    // safety: surface must be dropped before window
    _window: Arc<Window>,
}

struct UniformBufferState {
    uniform_buffer: Buffer,
    uniform_buffer_len: usize,

    canvas2d_uniform_bind_group: BindGroup,
    image_uniform_bind_group: BindGroup,
}


pub use crate::pipelines::image::GpuImage;

pub use crate::pipelines::text::{
    TextBlock,
    HorizontalAlign,
    VerticalAlign,
    FontId,
    TextSpan,
    LayedOutTextBlock,
    pt_to_px,
};


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
                    limits: Limits::default(),
                },
                None,
            )
            .await?;

        // create the layout for the standard bind group all canvas2d shaders
        // use for canvas2d transformations
        let canvas2d_uniform_bind_group_layout = device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("canvas2d uniform bind group layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: Some((Canvas2dUniformData::SIZE as u64).try_into().unwrap()),
                        },
                        count: None,
                    },
                ],
            });

        // create the clear pipeline
        trace!("creating clear pipeline");
        let clear_pipeline = ClearPipeline::new(&device).await?;

        // create the solid pipeline
        trace!("creating solid pipeline");
        let solid_pipeline = SolidPipeline::new(
            &device,
            &canvas2d_uniform_bind_group_layout,
        ).await?;

        // create the image pipeline
        trace!("creating image pipeline");
        let image_pipeline = ImagePipeline::new(
            &device,
            &canvas2d_uniform_bind_group_layout,
        ).await?;

        // create the text pipeline
        trace!("creating text pipeline");
        let text_pipeline = TextPipeline::new(
            &device,
            &canvas2d_uniform_bind_group_layout,
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
            config,
            uniform_buffer_state: None,
            canvas2d_uniform_bind_group_layout,
            clear_pipeline,
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
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
    }

    /// Draw a frame. The callback can draw onto the Canvas2d. Then it will be
    /// displayed on the window from <0,0> (top left corner) to <1,1> (bottom
    /// right corner).
    pub fn draw_frame(&mut self, f: impl FnOnce(Canvas2d)) -> Result<()> {
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
        let view = frame
            .texture
            .create_view(&TextureViewDescriptor::default());

        // accumulate draw data from the callback
        trace!("accumulating draw data");
        let mut canvas_target = Canvas2dTarget::new(&self.device);
        f(Canvas2d {
            renderer: self,
            target: &mut canvas_target,
            transform: Canvas2dTransform::identity(),
        });

        // text pre-render
        self.text_pipeline.pre_render(&self.device, &self.queue, &canvas_target);

        // write uniform data to uniform buffer
        trace!("writing uniform data");
        if !canvas_target.uniform_data_buf.is_empty() {
            let dst = self
                .uniform_buffer_state
                .as_ref()
                .filter(|state| state.uniform_buffer_len >= canvas_target.uniform_data_buf.len());
            if let Some(dst) = dst {
                // buffer already exists and is big enough to hold data
                trace!("re-using uniform buffer");
                self.queue.write_buffer(&dst.uniform_buffer, 0, &canvas_target.uniform_data_buf);
            } else {
                // buffer doesn't exist or isn't big enough
                trace!("creating new uniform buffer");
                let uniform_buffer = self.device
                    .create_buffer_init(&BufferInitDescriptor {
                        label: Some("uniform buffer"),
                        contents: &canvas_target.uniform_data_buf,
                        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                    });
                let canvas2d_uniform_bind_group = self.device
                    .create_bind_group(&BindGroupDescriptor {
                        label: Some("solid uniform bind group"),
                        layout: &self.canvas2d_uniform_bind_group_layout,
                        entries: &[
                            BindGroupEntry {
                                binding: 0,
                                resource: BindingResource::Buffer(BufferBinding {
                                    buffer: &uniform_buffer,
                                    offset: 0,
                                    size: Some((Canvas2dUniformData::SIZE as u64).try_into().unwrap()),
                                }),
                            },
                        ],
                    });
                let image_uniform_bind_group = self.image_pipeline
                    .create_bind_group(
                        &self.device,
                        &uniform_buffer,
                    );
                self.uniform_buffer_state = Some(UniformBufferState {
                    uniform_buffer,
                    uniform_buffer_len: canvas_target.uniform_data_buf.len(),
                    canvas2d_uniform_bind_group,
                    image_uniform_bind_group,
                });
            }
        }

        // begin encoder and pass
        trace!("creating encoder and pass");
        let mut encoder = self.device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: None,
            });
        {
            let mut pass = encoder
                .begin_render_pass(&RenderPassDescriptor {
                    label: None,
                    color_attachments: &[
                        RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Clear(Color::WHITE),
                                store: true,
                            }
                        }
                    ],
                    depth_stencil_attachment: None,
                });

            // clear the screen
            trace!("clearing screen");
            self.clear_pipeline.clear_screen(&mut pass);

            // make draw calls
            trace!("making draw calls");
            for draw_call in &canvas_target.draw_calls {
                match draw_call {
                    &Canvas2dDrawCall::Solid(ref call) => self
                        .solid_pipeline
                        .render_call(
                            call,
                            &mut pass,
                            &self.uniform_buffer_state,
                        ),
                    &Canvas2dDrawCall::Image(ref call) => self
                        .image_pipeline
                        .render_call(
                            call,
                            &mut pass,
                            &self.uniform_buffer_state,
                        ),
                    &Canvas2dDrawCall::Text(ref call) => self
                        .text_pipeline
                        .render_call(
                            call,
                            &mut pass,
                            &self.uniform_buffer_state,
                        )
                };
            }
            
            // finish
            trace!("finishing frame");

            // end scope to drop
        }
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
        Ok(self.load_image_preloaded(image))
    }

    /// Load an already parsed image onto the GPU.
    pub fn load_image_preloaded(&self, image: impl Borrow<DynamicImage>) -> GpuImage {
        self.image_pipeline
            .load_image(&self.device, &self.queue, &image.borrow().to_rgba8())
    }

    /// Read an OTF / TTF / etc font from a file and load it onto the renderer.
    ///
    /// Just reads the file with tokio then passes it to `self.load_font`.
    pub async fn load_font_file(&mut self, path: impl AsRef<Path>) -> Result<FontId> {
        let file_data = fs::read(path).await?;
        self.load_font(&file_data)
    }

    /// Load a font onto the renderer from OTF / TTF / etc file data.
    ///
    /// Be mindful that there is currently no way to un-load a font from the
    /// renderer.
    pub fn load_font(&mut self, file_data: &[u8]) -> Result<FontId> {
        let font = FontArc::try_from_vec(file_data.into())?;
        Ok(self.load_font_preloaded(font))
    }

    /// Load an already parsed font.
    pub fn load_font_preloaded(&mut self, font: FontArc) -> FontId {
        self.text_pipeline.load_font(font)
    }

    /// Pre-compute the layout for a text block.
    pub fn lay_out_text(&self, text_block: &TextBlock) -> LayedOutTextBlock {
        self.text_pipeline.lay_out_text(text_block)
    }
}


/// Target for drawing 2 dimensionally onto. Each successive draw call is
/// blended over the previously drawn data.
pub struct Canvas2d<'a> {
    renderer: &'a mut Renderer,
    target: &'a mut Canvas2dTarget,
    transform: Canvas2dTransform,
}

/// State that exists per canvas2d usage, which the canvas2d writes to.
struct Canvas2dTarget {
    /// Required alignment for all offsets into uniform_data.
    uniform_offset_align: usize,
    uniform_data_buf: Vec<u8>,
    draw_calls: Vec<Canvas2dDrawCall>,
    next_draw_text_call_index: usize,
}

enum Canvas2dDrawCall {
    Solid(DrawCallSolid),
    Image(DrawCallImage),
    Text(DrawCallText),
}

impl<'a> Canvas2d<'a> {
    /// View a `&mut Canvas` as a `Canvas` with no transformations.
    pub fn reborrow<'b>(&'b mut self) -> Canvas2d<'b> {
        Canvas2d {
            renderer: &mut *self.renderer,
            target: &mut *self.target,
            ..*self,
        }
    }

    /// Borrow as a canvas which, when drawn to, draws to self with the given
    /// translation.
    pub fn with_translate<'b>(&'b mut self, t: impl Into<Vec2<f32>>) -> Canvas2d<'b> {
        Canvas2d {
            renderer: &mut *self.renderer,
            target: &mut *self.target,
            transform: self.transform.with_translate(t.into()),
            ..*self
        }
    }
    
    /// Borrow as a canvas which, when drawn to, draws to self with the given
    /// scaling.
    ///
    /// Panics if either axis is negative.
    pub fn with_scale<'b>(&'b mut self, s: impl Into<Vec2<f32>>) -> Canvas2d<'b> {
        let s = s.into();
        assert!(s.x >= 0.0, "negative scaling");
        assert!(s.y >= 0.0, "negative scaling");
        Canvas2d {
            renderer: &mut *self.renderer,
            target: &mut *self.target,
            transform: self.transform.with_scale(s),
            ..*self
        }
    }

    /// Borrow as a canvas which, when drawn to, clips out everything below a
    /// certain x value before drawing to self.
    pub fn with_clip_min_x<'b>(&'b mut self, min_x: f32) -> Canvas2d<'b> {
        Canvas2d {
            renderer: &mut *self.renderer,
            target: &mut *self.target,
            transform: self.transform.with_clip_min_x(min_x),
            ..*self
        }
    }

    /// Borrow as a canvas which, when drawn to, clips out everything above a
    /// certain x value before drawing to self.
    pub fn with_clip_max_x<'b>(&'b mut self, max_x: f32) -> Canvas2d<'b> {
        Canvas2d {
            renderer: &mut *self.renderer,
            target: &mut *self.target,
            transform: self.transform.with_clip_max_x(max_x),
            ..*self
        }
    }
    
    /// Borrow as a canvas which, when drawn to, clips out everything below a
    /// certain y value before drawing to self.
    pub fn with_clip_min_y<'b>(&'b mut self, min_y: f32) -> Canvas2d<'b> {
        Canvas2d {
            renderer: &mut *self.renderer,
            target: &mut *self.target,
            transform: self.transform.with_clip_min_y(min_y),
            ..*self
        }
    }
    
    /// Borrow as a canvas which, when drawn to, clips out everything above a
    /// certain y value before drawing to self.
    pub fn with_clip_max_y<'b>(&'b mut self, max_y: f32) -> Canvas2d<'b> {
        Canvas2d {
            renderer: &mut *self.renderer,
            target: &mut *self.target,
            transform: self.transform.with_clip_max_y(max_y),
            ..*self
        }
    }

    /// Borrow as a canvas which, when drawn to, multiplies all colors by the
    /// given color value before drawing to self.
    pub fn with_color<'b>(&'b mut self, c: impl Into<Rgba<u8>>) -> Canvas2d<'b> {
        let c = c.into().map(|b| b as f32 / 0xFF as f32);
        Canvas2d {
            renderer: &mut *self.renderer,
            target: &mut *self.target,
            transform: self.transform.with_color(c),
            ..*self
        }
    }

    /// Draw a solid white square from <0,0> to <1,1>.
    pub fn draw_solid(&mut self) {
        prep_draw_solid_call(self);
    }

    /// Draw the given image from <0, 0> to <1, 1> with the given texture
    /// start and extent.
    ///
    /// Texture coordinates <0, 0> refers to the top-left of the image, and 
    /// texture coordinates <1, 1> refers to the bottom-right of the image.
    ///
    /// If texture coordinates go beyond the [0, 1] range, the image will
    /// repeat.
    pub fn draw_image(
        &mut self,
        image: &GpuImage,
        tex_start: impl Into<Vec2<f32>>,
        tex_extent: impl Into<Extent2<f32>>,
    ) {
        prep_draw_image_call(self, image, tex_start.into(), tex_extent.into());
    }

    /// Draw the given text block with <0, 0> as the top-left corner.
    pub fn draw_text(&mut self, text_block: &LayedOutTextBlock) {
        self.renderer.text_pipeline
            .prep_draw_text_call(
                &mut self.target,
                &self.transform,
                text_block,
            );
    }
}

impl Canvas2dTarget {
    /// Construct a new canvas2d target.
    fn new(device: &Device) -> Self {
        let uniform_offset_align = device
            .limits()
            .min_uniform_buffer_offset_alignment as usize;

        Canvas2dTarget {
            uniform_offset_align,
            uniform_data_buf: Vec::new(),
            draw_calls: Vec::new(),
            next_draw_text_call_index: 0,
        }
    }

    /// Given a canvas2d transform, produce its uniform data and push it to the
    /// uniform data buf, padding as necessary, and return its offset.
    fn push_uniform_data<T: Std140>(&mut self, data: &T) -> usize {
        pad(&mut self.uniform_data_buf, self.uniform_offset_align);
        // TODO make padding logic less jankily connected
        data.pad_write(&mut self.uniform_data_buf)
    }
}
