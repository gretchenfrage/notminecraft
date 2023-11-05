
use crate::{
    SWAPCHAIN_FORMAT,
    DEPTH_FORMAT,
    shader::load_shader,
};
use wgpu::*;
use anyhow::Result;

pub struct LinePipeline {
    line_pipeline: RenderPipeline,
}

impl LinePipeline {
    pub(crate) fn new(
        device: &Device,
        modifier_uniform_bind_group_layout: &BindGroupLayout,
        clip_texture_bind_group_layout: &BindGroupLayout,
    ) -> Result<Self> 
    {
        let line_vs_module = device
            .create_shader_module(load_shader!("line.vert")?);
        let line_fs_module = device
            .create_shader_module(load_shader!("line.frag")?);
        let line_pipeline_layout = device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("line pipeline layout"),
                bind_group_layouts: &[
                    modifier_uniform_bind_group_layout,
                    clip_texture_bind_group_layout,
                    clip_texture_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
        let line_pipeline = device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("line pipeline"),
                layout: Some(&line_pipeline_layout),
                vertex: VertexState {
                    module: &line_vs_module,
                    entry_point: "main",
                    buffers: &[],
                },
                fragment: Some(FragmentState {
                    module: &line_fs_module,
                    entry_point: "main",
                    targets: &[
                        Some(ColorTargetState {
                            format: SWAPCHAIN_FORMAT,
                            blend: Some(BlendState::ALPHA_BLENDING),
                            write_mask: ColorWrites::all(),
                        }),
                    ],
                }),
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::LineList,
                    ..Default::default()
                },
                depth_stencil: Some(DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: CompareFunction::LessEqual,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: MultisampleState::default(),
                multiview: None,
            });

        Ok(LinePipeline { line_pipeline })
    }

    pub(crate) fn render<'a>(
        &'a self,
        pass: &mut RenderPass<'a>,
    ) {
        pass.set_pipeline(&self.line_pipeline);
        pass.draw(0..2, 0..1);
    }
}
