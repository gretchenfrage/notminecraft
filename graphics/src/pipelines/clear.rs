//! Pipeline for clearing the screen.

use crate::{
    SWAPCHAIN_FORMAT,
    shader::load_shader,
};
use wgpu::*;
use anyhow::Result;


pub struct ClearPipeline {
    clear_pipeline: RenderPipeline,
}

impl ClearPipeline {
    pub async fn new(device: &Device) -> Result<Self> {
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
                    targets: &[SWAPCHAIN_FORMAT.into()],
                }),
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
                multiview: None,
            });

        Ok(ClearPipeline { clear_pipeline })
    }

    pub fn clear_screen<'a>(&'a self, pass: &mut RenderPass<'a>) {
        pass.set_pipeline(&self.clear_pipeline);
        pass.draw(0..1, 0..1);
    }
}