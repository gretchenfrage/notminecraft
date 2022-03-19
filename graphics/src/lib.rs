
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
        std140_struct,
        pad,
    },
};
use std::{
    path::Path,
    sync::Arc,
    mem::take,
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


mod pipelines;
mod std140;
mod shader;
mod vertex;

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
}

const SWAPCHAIN_FORMAT: TextureFormat = TextureFormat::Bgra8Unorm;

struct UniformBufferState {
    uniform_buffer: Buffer,
    uniform_buffer_len: usize,

    canvas2d_uniform_bind_group: BindGroup,
}

pub use crate::pipelines::image::GpuImage;



impl Renderer {
    /// Create a new renderer on a given window.
    pub async fn new(window: Arc<Window>) -> Result<Self> {
        trace!("beginning initializing renderer");

        // create the instance, surface, and adapter
        trace!("creating instance");
        let size = window.inner_size();
        let instance = Instance::new(Backends::PRIMARY);
        trace!("creating surface");
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
                label: Some("solid uniform bind group layout"),
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
        })
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
        let mut canvas_out_vars = Canvas2dOutVars::default();
        let uniform_offset_align = self.device
            .limits()
            .min_uniform_buffer_offset_alignment as usize;
        f(Canvas2d {
            renderer: self,
            out_vars: &mut canvas_out_vars,

            uniform_offset_align,

            transform: Canvas2dTransform::identity(),
        });

        // text pre-render
        self.text_pipeline.pre_render(&self.device, &self.queue, &canvas_out_vars);

        // write uniform data to uniform buffer
        trace!("writing uniform data");
        if !canvas_out_vars.uniform_data_buf.is_empty() {
            let dst = self
                .uniform_buffer_state
                .as_ref()
                .filter(|state| state.uniform_buffer_len >= canvas_out_vars.uniform_data_buf.len());
            if let Some(dst) = dst {
                // buffer already exists and is big enough to hold data
                trace!("re-using uniform buffer");
                self.queue.write_buffer(&dst.uniform_buffer, 0, &canvas_out_vars.uniform_data_buf);
            } else {
                // buffer doesn't exist or isn't big enough
                trace!("creating new uniform buffer");
                let uniform_buffer = self.device
                    .create_buffer_init(&BufferInitDescriptor {
                        label: Some("uniform buffer"),
                        contents: &canvas_out_vars.uniform_data_buf,
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
                self.uniform_buffer_state = Some(UniformBufferState {
                    uniform_buffer,
                    uniform_buffer_len: canvas_out_vars.uniform_data_buf.len(),
                    canvas2d_uniform_bind_group
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
            for draw_call in take(&mut canvas_out_vars.draw_calls) {
                match draw_call {
                    Canvas2dDrawCall::Solid(call) => self
                        .solid_pipeline
                        .render_call(
                            call,
                            &mut pass,
                            &self.uniform_buffer_state,
                        ),
                    Canvas2dDrawCall::Image(call) => self
                        .image_pipeline
                        .render_call(
                            call,
                            &mut pass,
                            &self.uniform_buffer_state,
                            &canvas_out_vars,
                        ),
                    Canvas2dDrawCall::Text(call) => self
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
    pub fn load_image(&self, file_data: &[u8]) -> Result<GpuImage> {
        self.image_pipeline.load_image(file_data, &self.device, &self.queue)
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
        self.text_pipeline.load_font(file_data)
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
    out_vars: &'a mut Canvas2dOutVars,
    
    // alignment for all offsets into uniform_data
    uniform_offset_align: usize,

    transform: Canvas2dTransform,
}

#[derive(Default)]
struct Canvas2dOutVars {
    uniform_data_buf: Vec<u8>,
    image_array: Vec<GpuImage>,
    draw_calls: Vec<Canvas2dDrawCall>,
    next_draw_text_call_index: usize,
}

enum Canvas2dDrawCall {
    Solid(DrawCallSolid),
    Image(DrawCallImage),
    Text(DrawCallText),
}

/// Accumulated transforms on a `Canvas2d`.
#[derive(Debug, Copy, Clone)]
struct Canvas2dTransform {
    affine: Mat3<f32>,
    color: Rgba<f32>,

    clip_min_x: Option<f32>,
    clip_max_x: Option<f32>,
    clip_min_y: Option<f32>,
    clip_max_y: Option<f32>,
}

impl Canvas2dTransform {
    /// Identity transform.
    fn identity() -> Self {
        Canvas2dTransform {
            affine: Mat3::identity(),
            color: Rgba::white(),
            clip_min_x: None,
            clip_max_x: None,
            clip_min_y: None,
            clip_max_y: None,
        }
    }

    /// Apply translation.
    fn with_translate(self, t: Vec2<f32>) -> Self {
        Canvas2dTransform {
            affine: self.affine * Mat3::<f32>::translation_2d(t),
            ..self
        }
    }

    /// Apply scaling.
    ///
    /// Assumes no negative scaling, that would break the clipping logic.
    ///
    /// TODO: Figure out a more mathematically robust clipping logic.
    fn with_scale(self, s: Vec2<f32>) -> Self {
        Canvas2dTransform {
            affine: self.affine * Mat3::<f32>::scaling_3d([s.x, s.y, 1.0]),
            ..self
        }
    }

    /// Apply color multiplication.
    fn with_color(self, c: Rgba<f32>) -> Self {
        Canvas2dTransform {
            color: self.color * c,
            ..self
        }
    }

    /// Apply min-x clipping.
    fn with_clip_min_x(self, min_x: f32) -> Self {
        let min_x = (self.affine * Vec3::new(min_x, 0.0, 1.0)).x;
        Canvas2dTransform {
            clip_min_x: Some(self.clip_min_x
                .map(|x| f32::max(x, min_x))
                .unwrap_or(min_x)),
            ..self
        }
    }

    /// Apply max-x clipping.
    fn with_clip_max_x(self, max_x: f32) -> Self {
        let max_x = (self.affine * Vec3::new(max_x, 0.0, 1.0)).x;
        Canvas2dTransform {
            clip_max_x: Some(self.clip_max_x
                .map(|x| f32::min(x, max_x))
                .unwrap_or(max_x)),
            ..self
        }
    }

    /// Apply min-y clipping.
    fn with_clip_min_y(self, min_y: f32) -> Self {
        let min_y = (self.affine * Vec3::new(0.0, min_y, 1.0)).y;
        Canvas2dTransform {
            clip_min_y: Some(self.clip_min_y
                .map(|x| f32::max(x, min_y))
                .unwrap_or(min_y)),
            ..self
        }
    }

    /// Apply max-y clipping.
    fn with_clip_max_y(self, max_y: f32) -> Self {
        let max_y = (self.affine * Vec3::new(0.0, max_y, 1.0)).y;
        Canvas2dTransform {
            clip_max_y: Some(self.clip_max_y
                .map(|x| f32::min(x, max_y))
                .unwrap_or(max_y)),
            ..self
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct Canvas2dUniformData {
    transform: Mat3<f32>,
    color: Rgba<f32>,
    clip_min_x: f32,
    clip_max_x: f32,
    clip_min_y: f32,
    clip_max_y: f32,
}

std140_struct! {
    Canvas2dUniformData {
        transform: Mat3<f32>,
        color: Rgba<f32>,
        clip_min_x: f32,
        clip_max_x: f32,
        clip_min_y: f32,
        clip_max_y: f32,
    }
}

impl<'a> Canvas2d<'a> {
    /// Borrow as a canvas which, when drawn to, draws to self with the given
    /// translation.
    pub fn with_translate<'b>(&'b mut self, t: impl Into<Vec2<f32>>) -> Canvas2d<'b> {
        Canvas2d {
            renderer: &mut *self.renderer,
            out_vars: &mut *self.out_vars,
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
            out_vars: &mut *self.out_vars,
            transform: self.transform.with_scale(s),
            ..*self
        }
    }

    /// Borrow as a canvas which, when drawn to, clips out everything below a
    /// certain x value before drawing to self.
    pub fn with_clip_min_x<'b>(&'b mut self, min_x: f32) -> Canvas2d<'b> {
        Canvas2d {
            renderer: &mut *self.renderer,
            out_vars: &mut *self.out_vars,
            transform: self.transform.with_clip_min_x(min_x),
            ..*self
        }
    }

    /// Borrow as a canvas which, when drawn to, clips out everything above a
    /// certain x value before drawing to self.
    pub fn with_clip_max_x<'b>(&'b mut self, max_x: f32) -> Canvas2d<'b> {
        Canvas2d {
            renderer: &mut *self.renderer,
            out_vars: &mut *self.out_vars,
            transform: self.transform.with_clip_max_x(max_x),
            ..*self
        }
    }
    
    /// Borrow as a canvas which, when drawn to, clips out everything below a
    /// certain y value before drawing to self.
    pub fn with_clip_min_y<'b>(&'b mut self, min_y: f32) -> Canvas2d<'b> {
        Canvas2d {
            renderer: &mut *self.renderer,
            out_vars: &mut *self.out_vars,
            transform: self.transform.with_clip_min_y(min_y),
            ..*self
        }
    }
    
    /// Borrow as a canvas which, when drawn to, clips out everything above a
    /// certain y value before drawing to self.
    pub fn with_clip_max_y<'b>(&'b mut self, max_y: f32) -> Canvas2d<'b> {
        Canvas2d {
            renderer: &mut *self.renderer,
            out_vars: &mut *self.out_vars,
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
            out_vars: &mut *self.out_vars,
            transform: self.transform.with_color(c),
            ..*self
        }
    }

    /// Push canvas2d uniform data onto `self.uniform_data_buf` based on
    /// current transform, and return the offset.
    fn push_uniform_data(&mut self) -> usize {
        let uniform_data = Canvas2dUniformData {
            transform: self.transform.affine,
            color: self.transform.color,
            clip_min_x: self.transform.clip_min_x.unwrap_or(f32::NEG_INFINITY),
            clip_max_x: self.transform.clip_max_x.unwrap_or(f32::INFINITY),
            clip_min_y: self.transform.clip_min_y.unwrap_or(f32::NEG_INFINITY),
            clip_max_y: self.transform.clip_max_y.unwrap_or(f32::INFINITY),
        };
        pad(&mut self.out_vars.uniform_data_buf, self.uniform_offset_align);
        // TODO make padding logic less jankily connected
        uniform_data.pad_write(&mut self.out_vars.uniform_data_buf)
    }

    /// Draw a solid white square from <0,0> to <1,1>.
    pub fn draw_solid(&mut self) {
        prep_draw_solid_call(self);
    }

    /// Draw the given image from <0, 0> to <1, 1>.
    pub fn draw_image(&mut self, image: &GpuImage) {
        prep_draw_image_call(self, image);
    }

    /// Draw the given text block with <0, 0> as the top-left corner.
    pub fn draw_text(&mut self, text_block: &LayedOutTextBlock) {
        unsafe { &mut *(&mut self.renderer.text_pipeline as *mut TextPipeline) } // TODO THIS IS SO GOD DAMN MESSED UP BUT I don't wanna do the necessary refactor atm
            .prep_draw_text_call(self, text_block);
    }
}


pub use crate::pipelines::text::{
    TextBlock,
    HorizontalAlign,
    VerticalAlign,
    FontId,
    TextSpan,
    LayedOutTextBlock,
    pt_to_px,
};
