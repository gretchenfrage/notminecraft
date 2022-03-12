
use std::{
    sync::Arc,
    path::Path,
    mem::size_of,
};
use anyhow::Result;
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


/// Top-level resource for drawing frames onto a window.
pub struct Renderer {
    surface: Surface,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,

    clear_pipeline: RenderPipeline,
    
    solid_pipeline: RenderPipeline,
    solid_uniform_bind_group_layout: BindGroupLayout,
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
                label: Some("solid bind group layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: Some((size_of::<SolidUniforms>() as u64).try_into().unwrap()),
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
        let frame = self.surface
            .get_current_texture()?;
        let view = frame
            .texture
            .create_view(&TextureViewDescriptor::default());

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

        pass.set_pipeline(&self.clear_pipeline);
        pass.draw(0..0, 0..1);
        drop(pass);
        self.queue.submit(Some(encoder.finish()));
        
        f(Canvas2d {
            renderer: self,
            target: &view,

            transform: Mat3::identity(),
            color: Rgba::white(),
        });
        
        frame.present();

        Ok(())
    }
}


#[derive(Copy, Clone, Debug)]
#[repr(C)]
#[repr(align(16))]
struct Std140Vec3 {
    xyz: [f32; 3],
    pad: u32,
}

impl From<Vec3<f32>> for Std140Vec3 {
    fn from(vec: Vec3<f32>) -> Std140Vec3 {
        Std140Vec3 {
            xyz: vec.into_array(),
            pad: 0,
        }
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
#[repr(align(16))]
struct Std140Mat3([Std140Vec3; 3]);

impl From<Mat3<f32>> for Std140Mat3 {
    fn from(mat: Mat3<f32>) -> Std140Mat3 {
        Std140Mat3(mat.cols.into_array().map(Std140Vec3::from))
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
#[repr(align(64))]
struct SolidUniforms {
    transform: Std140Mat3,
    color: [f32; 4],
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
    renderer: &'a Renderer,
    target: &'a TextureView,

    transform: Mat3<f32>,
    color: Rgba<f32>,
}

impl<'a> Canvas2d<'a> {
    /// Borrow as a canvas which, when drawn to, draws to self with the given
    /// translation.
    pub fn with_translate<'b>(&'b mut self, t: impl Into<Vec2<f32>>) -> Canvas2d<'b> {
        Canvas2d {
            transform: Mat3::<f32>::translation_2d(t) * self.transform,
            ..*self
        }
    }
    
    /// Borrow as a canvas which, when drawn to, draws to self with the given
    /// scaling.
    pub fn with_scale<'b>(&'b mut self, s: impl Into<Vec2<f32>>) -> Canvas2d<'b> {
        let s = s.into();
        Canvas2d {
            transform: Mat3::<f32>::scaling_3d([s.x, s.y, 1.0]) * self.transform,
            ..*self
        }
    }

    /// Borrow as a canvas which, when drawn to, multiplies all colors by the
    /// given color value before drawing to self.
    pub fn with_color<'b>(&'b mut self, c: impl Into<Rgba<u8>>) -> Canvas2d<'b> {
        Canvas2d {
            color: self.color * c.into().map(|b| b as f32 / 256.0),
            ..*self
        }
    }

    /// Draw a solid white square from <0,0> to <1,1>.
    pub fn draw_solid(&mut self) {
        let mut encoder = self.renderer.device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("solid command encoder"),
            });
        
        let uniform_struct = SolidUniforms {
            transform: Std140Mat3::from(self.transform),
            color: self.color.into_array(),
        };
        let uniform_bytes: &[u8] = unsafe {
            &*(
                &uniform_struct
                as *const SolidUniforms
                as *const [u8; size_of::<SolidUniforms>()]
            )
        };

        let uniform_buffer = self.renderer.device
            .create_buffer_init(&BufferInitDescriptor {
                label: Some("solid uniform buffer"),
                contents: uniform_bytes,
                usage: BufferUsages::UNIFORM,
            });
        let uniform_bind_group = self.renderer.device
            .create_bind_group(&BindGroupDescriptor {
                label: Some("solid uniform bind group"),
                layout: &self.renderer.solid_uniform_bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::Buffer(BufferBinding {
                            buffer: &uniform_buffer,
                            offset: 0,
                            size: Some((size_of::<SolidUniforms>() as u64).try_into().unwrap()),
                        }),
                    }
                ],
            });

        let mut pass = encoder
            .begin_render_pass(&RenderPassDescriptor {
                label: Some("solid render pass"),
                color_attachments: &[
                    RenderPassColorAttachment {
                        view: &*self.target,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: true,
                        },
                    },
                ],
                depth_stencil_attachment: None,
            });
        pass.set_pipeline(&self.renderer.solid_pipeline);
        pass.set_bind_group(0, &uniform_bind_group, &[]);
        pass.draw(0..6, 0..1);
        drop(pass);
        self.renderer.queue.submit(Some(encoder.finish()));
    }
}
