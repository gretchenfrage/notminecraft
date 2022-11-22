
use crate::{
    shader::load_shader,
    DEPTH_FORMAT,
};
use wgpu::*;
use anyhow::Result;
/*
pub struct ClearPipelineCreator {
    clear_vs_module: ShaderModule,
    clear_fs_module: ShaderModule,
    clear_pipeline_layout: PipelineLayout,
}
*/
pub struct ClearDepthPipeline {
    clear_depth_pipeline: RenderPipeline,
}

impl ClearDepthPipeline {
    pub(crate) fn new(device: &Device) -> Result<Self> {
        let clear_depth_vs_module = device
            .create_shader_module(load_shader!("clear_depth.vert")?);
        let clear_depth_fs_module = device
            .create_shader_module(load_shader!("clear_depth.frag")?);
        let clear_depth_pipeline_layout = device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("clear depth pipeline layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });
        let clear_depth_pipeline = device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("clear depth pipeline"),
                layout: Some(&clear_depth_pipeline_layout),
                vertex: VertexState {
                    module: &clear_depth_vs_module,
                    entry_point: "main",
                    buffers: &[],
                },
                fragment: Some(FragmentState {
                    module: &clear_depth_fs_module,
                    entry_point: "main",
                    targets: &[],
                }),
                primitive: PrimitiveState::default(),
                depth_stencil: Some(DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: CompareFunction::Always,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: MultisampleState::default(),
                multiview: None,
            });
        Ok(ClearDepthPipeline {
            clear_depth_pipeline,
        })
    }

    pub(crate) fn render(
        &self,
        encoder: &mut CommandEncoder,
        view: &TextureView,
        value: f32,
    ) {
        let mut pass = encoder
            .begin_render_pass(&RenderPassDescriptor {
                label: Some("clear depth pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(value),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
        pass.set_pipeline(&self.clear_depth_pipeline);
        pass.draw(0..1, 0..1);
    }
}
