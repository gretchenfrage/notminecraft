//! Pipeline for clearing the screen.

use crate::shader::load_shader;
use wgpu::*;
use anyhow::Result;

pub struct ClearPipelineCreator {
    clear_vs_module: ShaderModule,
    clear_fs_module: ShaderModule,
    clear_pipeline_layout: PipelineLayout,
}

pub struct ClearPipeline {
    clear_pipeline: RenderPipeline,
}

impl ClearPipelineCreator {
    pub(crate) async fn new(device: &Device) -> Result<Self> {
        let clear_vs_module = device
            .create_shader_module(load_shader!("clear.vert").await?);
        let clear_fs_module = device
            .create_shader_module(load_shader!("clear.frag").await?);
        let clear_pipeline_layout = device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("clear pipeline layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });
        Ok(ClearPipelineCreator {
            clear_vs_module,
            clear_fs_module,
            clear_pipeline_layout,
        })
    }

    pub(crate) fn create(
        &self,
        device: &Device,
        format: TextureFormat,
    ) -> ClearPipeline {
        let clear_pipeline = device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("clear pipeline"), // TODO label
                layout: Some(&self.clear_pipeline_layout),
                vertex: VertexState {
                    module: &self.clear_vs_module,
                    entry_point: "main",
                    buffers: &[],
                },
                fragment: Some(FragmentState {
                    module: &self.clear_fs_module,
                    entry_point: "main",
                    targets: &[Some(format.into())],
                }),
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
                multiview: None,
            });
        ClearPipeline { clear_pipeline }
    }
}

impl ClearPipeline {
    pub(crate) fn render(
        &self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        color: Color,
    ) {
        let mut pass = encoder
            .begin_render_pass(&RenderPassDescriptor {
                label: Some("clear pass"), // TODO label
                color_attachments: &[
                    Some(RenderPassColorAttachment {
                        view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Clear(color),
                            store: true,
                        },
                    }),
                ],
                depth_stencil_attachment: None,
            });
        pass.set_pipeline(&self.clear_pipeline);
        pass.draw(0..1, 0..1);
    }
}
