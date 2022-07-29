//! Pipeline for applying clips.
// TODO name consistency?

use crate::{
    SWAPCHAIN_FORMAT,
    shader::load_shader,
    create_depth_texture_like,
    std140::{
        Std140,
        std140_struct,
    },
    render_instrs::ClipEdit,
    uniform_buffer::UniformDataPacker,
};
use std::mem::swap;
use wgpu::*;
use vek::*;
use anyhow::Result;
use winit_main::reexports::dpi::PhysicalSize;


pub const CLIP_FORMAT: TextureFormat = TextureFormat::R32Float;

#[derive(Debug)]
pub struct ClipPipeline {
    clip_sampler: Sampler,
    pub clip_texture_bind_group_layout: BindGroupLayout,
    pub clip_min_texture: ClipTexture,
    pub clip_max_texture: ClipTexture,
    extra_clip_texture: ClipTexture,
    clip_edit_uniform_bind_group_layout: BindGroupLayout,
    clip_edit_pipeline: RenderPipeline,
}

#[derive(Debug)]
pub struct ClipTexture {
    pub view: TextureView,
    pub bind_group: BindGroup,
}

#[derive(Debug, Copy, Clone)]
struct ClipEditUniformData {
    sign: f32,
    clip: Vec4<f32>,
}

std140_struct!(ClipEditUniformData {
    sign: f32,
    clip: Vec4<f32>,
});

#[derive(Debug, Copy, Clone)]
pub struct PreppedClipEdit {
    max_clip_min: bool,
    uniform_offset: u32,
}

fn create_clip_texture(
    device: &Device,
    clip_sampler: &Sampler,
    clip_texture_bind_group_layout: &BindGroupLayout,
    size: PhysicalSize<u32>,
) -> ClipTexture
{
    let texture = create_depth_texture_like(
        device,
        size,
        "clip texture",
        CLIP_FORMAT,
    );

    let view = texture
        .create_view(&TextureViewDescriptor {
            label: Some("clip texture view"),
            ..Default::default()
        });

    let bind_group = device
        .create_bind_group(&BindGroupDescriptor {
            label: Some("clip texture bind group"),
            layout: clip_texture_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(clip_sampler),
                },
            ],
        });

    ClipTexture {
        view,
        bind_group,
    }
}

impl ClipPipeline {
    pub(crate) async fn new(
        device: &Device,
        size: PhysicalSize<u32>,
    ) -> Result<Self> {
        let clip_sampler = device
            .create_sampler(&SamplerDescriptor {
                label: Some("clip sampler"),
                ..Default::default()
            });

        let clip_texture_bind_group_layout = device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("clip texture bind group layout"),
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

        let clip_min_texture = create_clip_texture(
            device,
            &clip_sampler,
            &clip_texture_bind_group_layout,
            size,
        );
        let clip_max_texture = create_clip_texture(
            device,
            &clip_sampler,
            &clip_texture_bind_group_layout,
            size,
        );
        let extra_clip_texture = create_clip_texture(
            device,
            &clip_sampler,
            &clip_texture_bind_group_layout,
            size,
        );

        let clip_edit_uniform_bind_group_layout = device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("clip edit uniform bind group layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: Some(
                                (ClipEditUniformData::SIZE as u64).try_into().unwrap()
                            ),
                        },
                        count: None,
                    },
                ],
            });

        let clip_edit_vs_module = device
            .create_shader_module(&load_shader!("clip_edit.vert").await?);
        let clip_edit_fs_module = device
            .create_shader_module(&load_shader!("clip_edit.frag").await?);
        let clip_edit_pipeline_layout = device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("clip edit pipeline layout"),
                bind_group_layouts: &[
                    &clip_edit_uniform_bind_group_layout,
                    &clip_texture_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
        let clip_edit_pipeline = device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("clip edit pipeline"),
                layout: Some(&clip_edit_pipeline_layout),
                vertex: VertexState {
                    module: &clip_edit_vs_module,
                    entry_point: "main",
                    buffers: &[],
                },
                fragment: Some(FragmentState {
                    module: &clip_edit_fs_module,
                    entry_point: "main",
                    targets: &[
                        ColorTargetState {
                            format: CLIP_FORMAT,
                            blend: None, // TODO ???
                            write_mask: ColorWrites::ALL,
                        },
                    ],
                }),
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
                multiview: None,
            });

        Ok(ClipPipeline {
            clip_sampler,
            clip_texture_bind_group_layout,
            clip_min_texture,
            clip_max_texture,
            extra_clip_texture,
            clip_edit_uniform_bind_group_layout,
            clip_edit_pipeline,
        })
    }

    pub(crate) fn create_clip_edit_uniform_bind_group(
        &self,
        device: &Device,
        uniform_buffer: &Buffer,
    ) -> BindGroup
    {
        device
            .create_bind_group(&BindGroupDescriptor {
                label: Some("clip edit uniform bind group"),
                layout: &self.clip_edit_uniform_bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::Buffer(BufferBinding {
                            buffer: uniform_buffer,
                            offset: 0,
                            size: Some(
                                (ClipEditUniformData::SIZE as u64).try_into().unwrap(),
                            ),
                        }),
                    },
                ],
            })
    }

    pub(crate) fn resize(&mut self, device: &Device, size: PhysicalSize<u32>) {
        let clip_min_texture = create_clip_texture(
            device,
            &self.clip_sampler,
            &self.clip_texture_bind_group_layout,
            size,
        );
        let clip_max_texture = create_clip_texture(
            device,
            &self.clip_sampler,
            &self.clip_texture_bind_group_layout,
            size,
        );
        let extra_clip_texture = create_clip_texture(
            device,
            &self.clip_sampler,
            &self.clip_texture_bind_group_layout,
            size,
        );
    }

    pub(crate) fn pre_render(
        &self,
        edit: ClipEdit,
        uniform_packer: &mut UniformDataPacker,
        //uniform_vec: &mut Vec<u8>,
    ) -> PreppedClipEdit {
        /*let uniform_data = ClipEditUniformData {
            sign: if edit.max_clip_min { 1.0 } else { -1.0 },
            affine: edit.affine,
        };*/
        //let uniform_offset = uniform_data.pad_write(uniform_vec) as u32;
        /*
        let uniform_offset = uniform_packer
            .pack(&ClipEditUniformData {
                sign: if edit.max_clip_min { 1.0 } else { -1.0 },
                affine: edit.affine,
            });*/
        let uniform_offset = uniform_packer
            .pack(&ClipEditUniformData {
                sign: if edit.max_clip_min { 1.0 } else { -1.0 },
                clip: edit.clip,
            });
        PreppedClipEdit {
            max_clip_min: edit.max_clip_min,
            uniform_offset,
        }
    }

    pub(crate) fn render(
        &mut self,
        edit: PreppedClipEdit,
        encoder: &mut CommandEncoder,
        clip_edit_uniform_bind_group: &BindGroup,
    ) {
        // TODO: should I be reducing the number of render passes?
        let incumbent =
            if edit.max_clip_min { &mut self.clip_min_texture }
            else { &mut self.clip_max_texture };
        let mut pass = encoder
            .begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[
                    RenderPassColorAttachment {
                        view: &self.extra_clip_texture.view,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: true,
                        },
                    },
                ],
                depth_stencil_attachment: None,
            });
        pass.set_pipeline(&self.clip_edit_pipeline);
        pass.set_bind_group(0, &clip_edit_uniform_bind_group, &[edit.uniform_offset]);
        pass.set_bind_group(1, &incumbent.bind_group, &[]);
        pass.draw(0..6, 0..1);
        drop(pass);
        swap(incumbent, &mut self.extra_clip_texture);
    }
}