
use crate::{
    resources::gpu_image::{
        GpuImageArray,
        GpuImageArrayManager,
    },
    SWAPCHAIN_FORMAT,
    DEPTH_FORMAT,
    std140::{
        Std140,
        std140_struct,
    },
    shader::load_shader,
    uniform_buffer::UniformDataPacker,
};
use wgpu::*;
use vek::*;
use anyhow::*;


#[derive(Debug, Clone)]
pub struct DrawImage {
    pub image: GpuImageArray,
    pub tex_index: usize,
    pub tex_start: Vec2<f32>,
    pub tex_extent: Extent2<f32>,
}

pub struct ImagePipeline {
    image_pipeline: RenderPipeline,
    image_uniform_bind_group_layout: BindGroupLayout,
}

#[derive(Debug, Copy, Clone)]
struct ImageUniformData {
    tex_index: u32,
    tex_start: Vec2<f32>,
    tex_extent: Extent2<f32>,
}

std140_struct!(ImageUniformData {
    tex_index: u32,
    tex_start: Vec2<f32>,
    tex_extent: Extent2<f32>,
});

#[derive(Debug, Copy, Clone)]
pub struct PreppedDrawImage<'a> {
    image_uniform_offset: u32,
    image: &'a GpuImageArray,
}

impl ImagePipeline {
    pub(crate) fn new(
        device: &Device,
        modifier_uniform_bind_group_layout: &BindGroupLayout,
        clip_texture_bind_group_layout: &BindGroupLayout,
        gpu_image_manager: &GpuImageArrayManager,
    ) -> Result<Self>
    {
        let image_vs_module = device
            .create_shader_module(load_shader!("image.vert")?);
        let image_fs_module = device
            .create_shader_module(load_shader!("image.frag")?);
        let image_uniform_bind_group_layout = device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("image uniform bind group layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: Some((ImageUniformData::SIZE as u64).try_into().unwrap()),
                        },
                        count: None,
                    },
                ],
            });
        let image_pipeline_layout = device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("image pipeline layout"),
                bind_group_layouts: &[
                    modifier_uniform_bind_group_layout,
                    clip_texture_bind_group_layout,
                    clip_texture_bind_group_layout,
                    &image_uniform_bind_group_layout,
                    &gpu_image_manager.gpu_image_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
        let image_pipeline = device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("image pipeline"),
                layout: Some(&image_pipeline_layout),
                vertex: VertexState {
                    module: &image_vs_module,
                    entry_point: "main",
                    buffers: &[],
                },
                fragment: Some(FragmentState {
                    module: &image_fs_module,
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

        Ok(ImagePipeline {
            image_pipeline,
            image_uniform_bind_group_layout,
        })
    }

    pub(crate) fn create_image_uniform_bind_group(
        &self,
        device: &Device,
        uniform_buffer: &Buffer,
    ) -> BindGroup
    {
        device
            .create_bind_group(&BindGroupDescriptor {
                label: Some("image uniform bind group"),
                layout: &self.image_uniform_bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::Buffer(BufferBinding {
                            buffer: uniform_buffer,
                            offset: 0,
                            size: Some((ImageUniformData::SIZE as u64).try_into().unwrap())
                        }),
                    },
                ],
            })
    }

    pub(crate) fn pre_render<'a>(
        image: &'a DrawImage,
        uniform_packer: &mut UniformDataPacker,
    ) -> PreppedDrawImage<'a>
    {
        debug_assert!(image.tex_index < image.image.len());
        let image_uniform_offset = uniform_packer
            .pack(&ImageUniformData {
                tex_index: image.tex_index as u32,
                tex_start: image.tex_start,
                tex_extent: image.tex_extent,
            });
        PreppedDrawImage {
            image_uniform_offset,
            image: &image.image,
        }
    }

    pub(crate) fn render<'a>(
        &'a self,
        image: PreppedDrawImage<'a>,
        pass: &mut RenderPass<'a>,
        image_uniform_bind_group: &'a BindGroup,
    )
    {
        pass.set_pipeline(&self.image_pipeline);
        pass
            .set_bind_group(
                3,
                &image_uniform_bind_group,
                &[image.image_uniform_offset],
            );
        pass
            .set_bind_group(
                4,
                &image.image.0.texture_bind_group.as_ref().unwrap(),
                &[],
            );
        pass.draw(0..6, 0..1);
    }
}
