
use crate::{
    shader::load_shader,
    SWAPCHAIN_FORMAT,
};
use wgpu::*;
use anyhow::Result;


#[derive(Debug)]
pub struct FinishPipeline {
    pipeline: RenderPipeline,
    bind_group_layout: BindGroupLayout,
    color_sampler: Sampler,
    bind_group: BindGroup,
}

fn create_bind_group(
    device: &Device,
    bind_group_layout: &BindGroupLayout,
    color_texture_view: &TextureView,
    color_sampler: &Sampler,
) -> BindGroup {
    device
        .create_bind_group(&BindGroupDescriptor {
            label: Some("finish bind group"),
            layout: bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(color_texture_view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(color_sampler),
                },
            ],
        })
}

impl FinishPipeline {
    pub(crate) fn new(device: &Device, color_texture_view: &TextureView) -> Result<Self> {
        let bind_group_layout = device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("finish bind group layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float {
                                filterable: false,
                            },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                        count: None,
                    },
                ],
            });
        let vs_module = device
            .create_shader_module(load_shader!("finish.vert")?);
        let fs_module = device
            .create_shader_module(load_shader!("finish.frag")?);
        let pipeline_layout = device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("finish pipeline layout"),
                bind_group_layouts: &[
                    &bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
        let pipeline = device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("finish pipeline"),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &vs_module,
                    entry_point: "main",
                    buffers: &[],
                },
                fragment: Some(FragmentState {
                    module: &fs_module,
                    entry_point: "main",
                    targets: &[
                        Some(ColorTargetState {
                            format: SWAPCHAIN_FORMAT,
                            blend: None,
                            write_mask: ColorWrites::ALL,
                        }),
                    ],
                }),
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
                multiview: None,
            });
        let color_sampler = device
            .create_sampler(&SamplerDescriptor {
                label: Some("finish color sampler"),
                ..Default::default()
            });
        let bind_group = create_bind_group(
            device,
            &bind_group_layout,
            color_texture_view,
            &color_sampler,
        );
        Ok(FinishPipeline {
            pipeline,
            bind_group_layout,
            color_sampler,
            bind_group,
        })
    }

    pub(crate) fn resize(&mut self, device: &Device, color_texture_view: &TextureView) {
        self.bind_group = create_bind_group(
            device,
            &self.bind_group_layout,
            color_texture_view,
            &self.color_sampler,
        );
    }

    pub(crate) fn render(&mut self, encoder: &mut CommandEncoder, surface: &SurfaceTexture) {
        let surface_view = surface.texture
            .create_view(&TextureViewDescriptor {
                label: Some("surface texture view"),
                ..Default::default()
            });
        let mut pass = encoder
            .begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[
                    Some(RenderPassColorAttachment {
                        view: &surface_view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: true,
                        },
                    }),
                ],
                depth_stencil_attachment: None,
            });
        pass.set_pipeline(&self.pipeline);
        pass
            .set_bind_group(
                0,
                &self.bind_group,
                &[],
            );
        pass.draw(0..6, 0..1);
    }
}
