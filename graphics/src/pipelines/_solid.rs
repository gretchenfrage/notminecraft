//! 2D Pipeline for drawing a solid rectangle.

use crate::{
    SWAPCHAIN_FORMAT,
    Canvas2d,
    Canvas2dDrawCall,
    UniformBufferState,
    shader::load_shader,
};
use wgpu::*;
use anyhow::Result;

pub struct SolidPipeline {
    solid_pipeline: RenderPipeline,
}

pub struct DrawCallSolid {
    uniform_offset: usize,
}

pub fn prep_draw_solid_call(canvas: &mut Canvas2d) {
    // push uniform data
    let uniform_offset = canvas.target
        .push_uniform_data(&canvas.transform.to_uniform_data());

    // push draw call
    let call = DrawCallSolid { uniform_offset };
    canvas.target.draw_calls.push(Canvas2dDrawCall::Solid(call));
}

impl SolidPipeline {
    pub(crate) async fn new(
        device: &Device,
        canvas2d_uniform_bind_group_layout: &BindGroupLayout,
    ) -> Result<Self> {
        let solid_vs_module = device
            .create_shader_module(&load_shader!("solid.vert").await?);
        let solid_fs_module = device
            .create_shader_module(&load_shader!("solid.frag").await?);
        let solid_pipeline_layout = device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("solid pipeline layout"),
                bind_group_layouts: &[
                    canvas2d_uniform_bind_group_layout,
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
                        ColorTargetState {
                            format: SWAPCHAIN_FORMAT,
                            blend: Some(BlendState::ALPHA_BLENDING),
                            write_mask: ColorWrites::all(),
                        },
                    ],
                }),
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
                multiview: None,
            });

        Ok(SolidPipeline { solid_pipeline })
    }

    pub(crate) fn render_call<'a>(
        &'a self,
        call: &'a DrawCallSolid,
        pass: &mut RenderPass<'a>,
        uniform_buffer_state: &'a Option<UniformBufferState>,
    ) {
        let uniform_buffer_state = uniform_buffer_state
            .as_ref()
            .unwrap();

        pass.set_pipeline(&self.solid_pipeline);
        pass.set_bind_group(
            0,
            &uniform_buffer_state.canvas2d_uniform_bind_group,
            &[call.uniform_offset as u32],
        );
        pass.draw(0..6, 0..1);
    }
}