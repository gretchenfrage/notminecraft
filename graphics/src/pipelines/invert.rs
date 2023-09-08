
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
    create_depth_texture_like,
};
use wgpu::*;
use vek::*;
use anyhow::*;


#[derive(Debug, Clone)]
pub struct DrawInvert {
    pub image: GpuImageArray,
    pub tex_index: usize,
    pub tex_start: Vec2<f32>,
    pub tex_extent: Extent2<f32>,
}

pub struct InvertPipeline {
    pipeline: RenderPipeline,
    uniform_bind_group_layout: BindGroupLayout,
    drawn_texture: Texture,
    drawn_texture_view: TextureView,
    drawn_sampler: Sampler,
}

#[derive(Debug, Copy, Clone)]
struct UniformData {
    tex_index: u32,
    tex_start: Vec2<f32>,
    tex_extent: Extent2<f32>,
}

std140_struct!(UniformData {
    tex_index: u32,
    tex_start: Vec2<f32>,
    tex_extent: Extent2<f32>,
});

#[derive(Debug, Copy, Clone)]
pub struct PreppedDrawInvert<'a> {
    uniform_offset: u32,
    image: &'a GpuImageArray,
}

fn create_drawn_texture(
    device: &Device,
    size: Extent2<u32>,
) -> (Texture, TextureView, Sampler) {
    let drawn_texture = create_depth_texture_like(
        device,
        size,
        "invert drawn texture",
        SWAPCHAIN_FORMAT,
        Some(TextureUsages::COPY_DST),
    );
    let drawn_texture_view = drawn_texture
        .create_view(&TextureViewDescriptor {
            label: Some("clip texture view"),
            ..Default::default()
        });
    let drawn_sampler = device
        .create_sampler(&SamplerDescriptor {
            label: Some("drawn sampler"),
            ..Default::default()
        });
    (drawn_texture, drawn_texture_view, drawn_sampler)
}

impl InvertPipeline {
    pub(crate) fn new(
        device: &Device,
        modifier_uniform_bind_group_layout: &BindGroupLayout,
        clip_texture_bind_group_layout: &BindGroupLayout,
        gpu_image_manager: &GpuImageArrayManager,
        size: Extent2<u32>,
    ) -> Result<Self>
    {
        let vs_module = device
            .create_shader_module(load_shader!("invert.vert")?);
        let fs_module = device
            .create_shader_module(load_shader!("invert.frag")?);
        let uniform_bind_group_layout = device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("invert uniform bind group layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: Some((UniformData::SIZE as u64).try_into().unwrap()),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
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
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                        count: None,
                    },
                ],
            });
        let pipeline_layout = device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("invert pipeline layout"),
                bind_group_layouts: &[
                    modifier_uniform_bind_group_layout,
                    clip_texture_bind_group_layout,
                    clip_texture_bind_group_layout,
                    &uniform_bind_group_layout,
                    &gpu_image_manager.gpu_image_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
        let pipeline = device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("invert pipeline"),
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
        let (
            drawn_texture,
            drawn_texture_view,
            drawn_sampler,
        ) = create_drawn_texture(device, size);

        Ok(InvertPipeline {
            pipeline,
            uniform_bind_group_layout,
            drawn_texture,
            drawn_texture_view,
            drawn_sampler,
        })
    }

    pub(crate) fn resize(&mut self, device: &Device, size: Extent2<u32>) {
        let (
            drawn_texture,
            drawn_texture_view,
            drawn_sampler,
        ) = create_drawn_texture(device, size);
        self.drawn_texture = drawn_texture;
        self.drawn_texture_view = drawn_texture_view;
        self.drawn_sampler = drawn_sampler;
    }

    pub(crate) fn create_uniform_bind_group(
        &self,
        device: &Device,
        uniform_buffer: &Buffer,
    ) -> BindGroup
    {
        device
            .create_bind_group(&BindGroupDescriptor {
                label: Some("invert uniform bind group"),
                layout: &self.uniform_bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::Buffer(BufferBinding {
                            buffer: uniform_buffer,
                            offset: 0,
                            size: Some((UniformData::SIZE as u64).try_into().unwrap())
                        }),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::TextureView(&self.drawn_texture_view),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::Sampler(&self.drawn_sampler),
                    },
                ],
            })
    }

    pub(crate) fn pre_render<'a>(
        invert: &'a DrawInvert,
        uniform_packer: &mut UniformDataPacker,
    ) -> PreppedDrawInvert<'a>
    {
        debug_assert!(invert.tex_index < invert.image.len());
        let uniform_offset = uniform_packer
            .pack(&UniformData {
                tex_index: invert.tex_index as u32,
                tex_start: invert.tex_start,
                tex_extent: invert.tex_extent,
            });
        PreppedDrawInvert {
            uniform_offset,
            image: &invert.image,
        }
    }

    pub(crate) fn immediate_pre_render(
        &self,
        encoder: &mut CommandEncoder,
        color_texture: &Texture,
        size: Extent2<u32>,
    ) {
        encoder
            .copy_texture_to_texture(
                ImageCopyTexture {
                    texture: color_texture,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                ImageCopyTexture {
                    texture: &self.drawn_texture,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                Extent3d { // TODO: could hypothetically optimize this more
                    width: size.w,
                    height: size.h,
                    depth_or_array_layers: 1,
                },
            );
    }

    pub(crate) fn render<'a>(
        &'a self,
        invert: PreppedDrawInvert<'a>,
        pass: &mut RenderPass<'a>,
        uniform_bind_group: &'a BindGroup,
    )
    {
        pass.set_pipeline(&self.pipeline);
        pass
            .set_bind_group(
                3,
                &uniform_bind_group,
                &[invert.uniform_offset],
            );
        pass
            .set_bind_group(
                4,
                &invert.image.0.texture_bind_group.as_ref().unwrap(),
                &[],
            );
        pass.draw(0..6, 0..1);
    }
}
