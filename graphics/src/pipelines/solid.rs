
use crate::{
    SWAPCHAIN_FORMAT,
    DEPTH_FORMAT,
    shader::load_shader,
};
use wgpu::*;
use anyhow::Result;

pub struct SolidPipeline {
    solid_pipeline: RenderPipeline,
}

impl SolidPipeline {
    pub(crate) fn new(
        device: &Device,
        modifier_uniform_bind_group_layout: &BindGroupLayout,
        clip_texture_bind_group_layout: &BindGroupLayout,
    ) -> Result<Self> 
    {
        let solid_vs_module = device
            .create_shader_module(load_shader!("solid.vert")?);
        let solid_fs_module = device
            .create_shader_module(load_shader!("solid.frag")?);
        let solid_pipeline_layout = device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("solid pipeline layout"),
                bind_group_layouts: &[
                    modifier_uniform_bind_group_layout,
                    clip_texture_bind_group_layout,
                    clip_texture_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
        let solid_pipeline = device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("solid pipeline"),
                layout: Some(&solid_pipeline_layout),
                vertex: VertexState {
                    module: &solid_vs_module,
                    entry_point: "main",
                    buffers: &[],
                },
                fragment: Some(FragmentState {
                    module: &solid_fs_module,
                    entry_point: "main",
                    targets: &[
                        Some(ColorTargetState {
                            format: SWAPCHAIN_FORMAT,
                            blend: Some(BlendState::ALPHA_BLENDING),
                            write_mask: ColorWrites::all(),
                        }),
                    ],
                }),
                primitive: PrimitiveState::default(),
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

        Ok(SolidPipeline { solid_pipeline })
    }

    pub(crate) fn render<'a>(
        &'a self,
        pass: &mut RenderPass<'a>,
    ) {
        pass.set_pipeline(&self.solid_pipeline);
        pass.draw(0..6, 0..1);
    }
}
