
use crate::{
    SWAPCHAIN_FORMAT,
    DEPTH_FORMAT,
    shader::load_shader,
    view_proj::ViewProj,
};
use wgpu::*;
use anyhow::Result;

pub struct SkyPipeline {
    sky_pipeline: RenderPipeline,
}

#[derive(Debug, Copy, Clone)]
pub struct DrawSky {
    pub view_proj: ViewProj,
    /// Point in the day night cycle, where 0 is sunrise, 0.25 is mid day,
    /// 0.5 is sun set, 0.75 is midnight, and 1 is the next sunrise.
    pub day_night_time: f32,
}


impl SkyPipeline {
    pub(crate) fn new(
        device: &Device,
        modifier_uniform_bind_group_layout: &BindGroupLayout,
        clip_texture_bind_group_layout: &BindGroupLayout,
    ) -> Result<Self> 
    {
        let sky_vs_module = device
            .create_shader_module(load_shader!("sky.vert")?);
        let sky_fs_module = device
            .create_shader_module(load_shader!("sky.frag")?);
        let sky_pipeline_layout = device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("sky pipeline layout"),
                bind_group_layouts: &[
                    modifier_uniform_bind_group_layout,
                    clip_texture_bind_group_layout,
                    clip_texture_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
        let sky_pipeline = device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("sky pipeline"),
                layout: Some(&sky_pipeline_layout),
                vertex: VertexState {
                    module: &sky_vs_module,
                    entry_point: "main",
                    buffers: &[],
                },
                fragment: Some(FragmentState {
                    module: &sky_fs_module,
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

        Ok(SkyPipeline { sky_pipeline })
    }

    pub(crate) fn render<'a>(
        &'a self,
        pass: &mut RenderPass<'a>,
    ) {
        pass.set_pipeline(&self.sky_pipeline);
        pass.draw(0..6, 0..1);
    }
}
