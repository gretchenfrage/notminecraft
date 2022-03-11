
use std::{
    sync::Arc,
    path::Path,
};
use anyhow::Result;
use winit_main::reexports::{
    window::Window,
    dpi::PhysicalSize,
};
use wgpu::*;
use tokio::fs;
use shaderc::{
    Compiler,
    ShaderKind,
};


/// Top-level resource for drawing frames onto a window.
pub struct Renderer {
    surface: Surface,
    device: Device,
    queue: Queue,
    render_pipeline: RenderPipeline,
    config: SurfaceConfiguration,
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

        /*
        let vs_module = device
            .create_shader_module(&include_spirv!("shader.vert.spv"));
        let fs_module = device
            .create_shader_module(&include_spirv!("shader.frag.spv"));*/
        let vs_module = device
            .create_shader_module(&load_shader("shader.vert").await?);
        let fs_module = device
            .create_shader_module(&load_shader("shader.frag").await?);

        let pipeline_layout = device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        let swapchain_format = TextureFormat::Bgra8Unorm;

        let render_pipeline = device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &vs_module,
                    entry_point: "main",
                    buffers: &[],
                },
                fragment: Some(FragmentState {
                    module: &fs_module,
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
            render_pipeline,
            config,
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
                            load: LoadOp::Clear(Color::GREEN),
                            store: true,
                        }
                    }
                ],
                depth_stencil_attachment: None,
            });
        pass.set_pipeline(&self.render_pipeline);
        pass.draw(0..3, 0..1);
        drop(pass);
        self.queue.submit(Some(encoder.finish()));
        frame.present();

        Ok(())
    }
}

async fn load_shader(name: &'static str) -> Result<ShaderModuleDescriptor<'static>> {
    let path = Path::new("src").join(name);
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
pub struct Canvas2d {

}

impl Canvas2d {
    /// Draw a solid white square from <0,0> to <1,1>.
    pub fn draw_solid(&mut self) {

    }
}
