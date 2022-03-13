
use crate::std140::{
    Std140,
    std140_struct,
    pad,
};
use std::{
    sync::Arc,
    path::Path,
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
use tokio::fs;
use shaderc::{
    Compiler,
    ShaderKind,
};
use vek::*;


mod std140;


/// Top-level resource for drawing frames onto a window.
pub struct Renderer {
    surface: Surface,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    uniform_buffer_state: Option<UniformBufferState>,

    clear_pipeline: RenderPipeline,
    
    solid_pipeline: RenderPipeline,
    solid_uniform_bind_group_layout: BindGroupLayout,
}

struct UniformBufferState {
    uniform_buffer: Buffer,
    uniform_buffer_len: usize,

    solid_uniform_bind_group: BindGroup,
}

impl Renderer {
    /// Create a new renderer on a given window.
    pub async fn new(window: Arc<Window>) -> Result<Self> {
        let size = window.inner_size();
        let instance = Instance::new(Backends::PRIMARY);
        let surface = unsafe { instance.create_surface(&*window) };
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| anyhow::Error::msg("failed to find an appropriate adapter"))?;

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

        let swapchain_format = TextureFormat::Bgra8Unorm;

        let clear_vs_module = device
            .create_shader_module(&load_shader("clear.vert").await?);
        let clear_fs_module = device
            .create_shader_module(&load_shader("clear.frag").await?);
        let clear_pipeline_layout = device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("clear pipeline layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });
        let clear_pipeline = device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("clear pipeline"),
                layout: Some(&clear_pipeline_layout),
                vertex: VertexState {
                    module: &clear_vs_module,
                    entry_point: "main",
                    buffers: &[],
                },
                fragment: Some(FragmentState {
                    module: &clear_fs_module,
                    entry_point: "main",
                    targets: &[swapchain_format.into()],
                }),
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
                multiview: None,
            });

        let solid_vs_module = device
            .create_shader_module(&load_shader("solid.vert").await?);
        let solid_fs_module = device
            .create_shader_module(&load_shader("solid.frag").await?);
        let solid_uniform_bind_group_layout = device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("solid uniform bind group layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: None, // TODO Some((DrawSolidUniformData::SIZE as u64).try_into().unwrap()),
                        },
                        count: None,
                    },
                ],
            });
        let solid_pipeline_layout = device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("solid pipeline layout"),
                bind_group_layouts: &[
                    &solid_uniform_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
        let solid_pipeline = device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("solid"),
                layout: Some(&solid_pipeline_layout),
                vertex: VertexState {
                    module: &solid_vs_module,
                    entry_point: "main",
                    buffers: &[],
                },
                fragment: Some(FragmentState {
                    module: &solid_fs_module,
                    entry_point: "main",
                    targets: &[swapchain_format.into()],
                }),
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
                multiview: None,
            });

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Mailbox,
        };

        surface.configure(&device, &config);

        Ok(Renderer {
            surface,
            device,
            queue,
            config,
            uniform_buffer_state: None,

            clear_pipeline,

            solid_pipeline,
            solid_uniform_bind_group_layout,
        })
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

        // begin encoder and pass
        trace!("creating encoder and pass");
        let mut encoder = self.device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: None,
            });
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
        pass.set_pipeline(&self.clear_pipeline);
        pass.draw(0..1, 0..1);
        
        // accumulate uniform data for this frame
        trace!("accumulating uniform data");
        let mut uniform_data = Vec::new();
        let mut draw_solid_calls = Vec::new();
        f(Canvas2d {
            uniform_data: &mut uniform_data,
            draw_solid_calls: &mut draw_solid_calls,

            uniform_offset_align: self.device.limits().min_uniform_buffer_offset_alignment as usize,

            transform: Canvas2dTransform::identity(),
        });

        // write uniform data to uniform buffer
        trace!("writing uniform data");
        if !uniform_data.is_empty() {
            let dst = self
                .uniform_buffer_state
                .as_ref()
                .filter(|state| state.uniform_buffer_len >= uniform_data.len());
            if let Some(dst) = dst {
                // buffer already exists and is big enough to hold data
                trace!("re-using uniform buffer");
                self.queue.write_buffer(&dst.uniform_buffer, 0, &uniform_data);
            } else {
                // buffer doesn't exist or isn't big enough
                trace!("creating new uniform buffer");
                let uniform_buffer = self.device
                    .create_buffer_init(&BufferInitDescriptor {
                        label: Some("uniform buffer"),
                        contents: &uniform_data,
                        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                    });
                let solid_uniform_bind_group = self.device
                    .create_bind_group(&BindGroupDescriptor {
                        label: Some("solid uniform bind group"),
                        layout: &self.solid_uniform_bind_group_layout,
                        entries: &[
                            BindGroupEntry {
                                binding: 0,
                                resource: BindingResource::Buffer(BufferBinding {
                                    buffer: &uniform_buffer,
                                    offset: 0,
                                    size: Some((DrawSolidUniformData::SIZE as u64).try_into().unwrap()),
                                }),
                            },
                        ],
                    });
                self.uniform_buffer_state = Some(UniformBufferState {
                    uniform_buffer,
                    uniform_buffer_len: uniform_data.len(),
                    solid_uniform_bind_group
                });
            }
        }

        // make draw calls
        trace!("making draw calls");
        if !draw_solid_calls.is_empty() {
            let uniform_buffer_state = self
                .uniform_buffer_state
                .as_ref()
                .unwrap();

            pass.set_pipeline(&self.solid_pipeline);
            for offset in draw_solid_calls {
                pass.set_bind_group(
                    0,
                    &uniform_buffer_state.solid_uniform_bind_group,
                    &[offset as u32],
                );
                pass.draw(0..6, 0..1);
            }
        }
        
        // finish
        trace!("finishing frame");
        trace!("dropping pass");
        drop(pass);
        trace!("submitting queue");
        self.queue.submit(Some(encoder.finish()));        
        trace!("presenting frame");
        frame.present();
        trace!("done");
        Ok(())
    }
}

async fn load_shader(name: &'static str) -> Result<ShaderModuleDescriptor<'static>> {
    let path = Path::new("src/shaders").join(name);
    let glsl = fs::read(&path).await?;
    let glsl = String::from_utf8(glsl)
        .map_err(|_| anyhow::Error::msg("shader not utf-8"))?;

    let kind =
        if name.ends_with(".vert") { ShaderKind::Vertex }
        else if name.ends_with(".frag") { ShaderKind::Fragment }
        else { return Err(anyhow::Error::msg("unknown chader kind")) };

    let mut compiler = Compiler::new()
        .ok_or_else(|| anyhow::Error::msg("not shaderc compiler"))?;

    let artifact = compiler.compile_into_spirv(
        &glsl,
        kind,
        name,
        "main",
        None,
    )?;

    Ok(ShaderModuleDescriptor {
        label: Some(name),
        source: ShaderSource::SpirV(artifact.as_binary().to_owned().into()),
    })
}

/// Target for drawing 2 dimensionally onto. Each successive draw call is
/// blended over the previously drawn data.
pub struct Canvas2d<'a> {
    uniform_data: &'a mut Vec<u8>,
    draw_solid_calls: &'a mut Vec<usize>,

    // alignment for all offsets into uniform_data
    uniform_offset_align: usize,

    transform: Canvas2dTransform,
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
            affine: Mat3::<f32>::translation_2d(t) * self.affine,
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
            affine: Mat3::<f32>::scaling_3d([s.x, s.y, 1.0]) * self.affine,
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
struct DrawSolidUniformData {
    transform: Mat3<f32>,
    color: Rgba<f32>,
    clip_min_x: f32,
    clip_max_x: f32,
    clip_min_y: f32,
    clip_max_y: f32,
}

std140_struct! {
    DrawSolidUniformData {
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
            uniform_data: &mut *self.uniform_data,
            draw_solid_calls: &mut *self.draw_solid_calls,
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
            uniform_data: &mut *self.uniform_data,
            draw_solid_calls: &mut *self.draw_solid_calls,
            transform: self.transform.with_scale(s),
            ..*self
        }
    }

    /// Borrow as a canvas which, when drawn to, clips out everything below a
    /// certain x value before drawing to self.
    pub fn with_clip_min_x<'b>(&'b mut self, min_x: f32) -> Canvas2d<'b> {
        Canvas2d {
            uniform_data: &mut *self.uniform_data,
            draw_solid_calls: &mut *self.draw_solid_calls,
            transform: self.transform.with_clip_min_x(min_x),
            ..*self
        }
    }

    /// Borrow as a canvas which, when drawn to, clips out everything above a
    /// certain x value before drawing to self.
    pub fn with_clip_max_x<'b>(&'b mut self, max_x: f32) -> Canvas2d<'b> {
        Canvas2d {
            uniform_data: &mut *self.uniform_data,
            draw_solid_calls: &mut *self.draw_solid_calls,
            transform: self.transform.with_clip_max_x(max_x),
            ..*self
        }
    }
    
    /// Borrow as a canvas which, when drawn to, clips out everything below a
    /// certain y value before drawing to self.
    pub fn with_clip_min_y<'b>(&'b mut self, min_y: f32) -> Canvas2d<'b> {
        Canvas2d {
            uniform_data: &mut *self.uniform_data,
            draw_solid_calls: &mut *self.draw_solid_calls,
            transform: self.transform.with_clip_min_y(min_y),
            ..*self
        }
    }
    
    /// Borrow as a canvas which, when drawn to, clips out everything above a
    /// certain y value before drawing to self.
    pub fn with_clip_max_y<'b>(&'b mut self, max_y: f32) -> Canvas2d<'b> {
        Canvas2d {
            uniform_data: &mut *self.uniform_data,
            draw_solid_calls: &mut *self.draw_solid_calls,
            transform: self.transform.with_clip_max_y(max_y),
            ..*self
        }
    }

    /// Borrow as a canvas which, when drawn to, multiplies all colors by the
    /// given color value before drawing to self.
    pub fn with_color<'b>(&'b mut self, c: impl Into<Rgba<u8>>) -> Canvas2d<'b> {
        let c = c.into().map(|b| b as f32 / 0xFF as f32);
        Canvas2d {
            uniform_data: &mut *self.uniform_data,
            draw_solid_calls: &mut *self.draw_solid_calls,
            transform: self.transform.with_color(c),
            ..*self
        }
    }

    /// Draw a solid white square from <0,0> to <1,1>.
    pub fn draw_solid(&mut self) {
        let uniform_data = DrawSolidUniformData {
            transform: self.transform.affine,
            color: self.transform.color,
            clip_min_x: self.transform.clip_min_x.unwrap_or(f32::NEG_INFINITY),
            clip_max_x: self.transform.clip_max_x.unwrap_or(f32::INFINITY),
            clip_min_y: self.transform.clip_min_y.unwrap_or(f32::NEG_INFINITY),
            clip_max_y: self.transform.clip_max_y.unwrap_or(f32::INFINITY),
        };
        pad(self.uniform_data, self.uniform_offset_align);
        let uniform_offset = uniform_data.pad_write(self.uniform_data);
        self.draw_solid_calls.push(uniform_offset);
    }
}
