
use crate::{
    SWAPCHAIN_FORMAT,
    DEPTH_FORMAT,
    std140::{
        Std140,
        std140_struct,
    },
    shader::load_shader,
    uniform_buffer::UniformDataPacker,
};
use std::sync::Arc;
use image::RgbaImage;
use wgpu::{
    *,
    util::DeviceExt,
};
use vek::*;
use anyhow::*;


#[derive(Debug, Clone)]
pub struct DrawImage {
    pub image: GpuImage,
    pub tex_start: Vec2<f32>,
    pub tex_extent: Extent2<f32>,
}


/// 2D RGBA image loaded into a GPU texture.
///
/// Internally reference-counted.
#[derive(Debug, Clone)]
pub struct GpuImage(Arc<GpuImageInner>);

#[derive(Debug)]
struct GpuImageInner {
    size: Extent2<u32>,
    texture_bind_group: BindGroup, // TODO proper error for 0-size images
}

impl GpuImage {
    /// Get image size in pixels.
    pub fn size(&self) -> Extent2<u32> {
        self.0.size
    }
}


pub struct ImagePipeline {
    image_pipeline: RenderPipeline,
    image_uniform_bind_group_layout: BindGroupLayout,
    image_texture_bind_group_layout: BindGroupLayout,
    image_sampler: Sampler,
}

#[derive(Debug, Copy, Clone)]
struct ImageUniformData {
    tex_start: Vec2<f32>,
    tex_extent: Extent2<f32>,
}

std140_struct!(ImageUniformData {
    tex_start: Vec2<f32>,
    tex_extent: Extent2<f32>,
});

#[derive(Debug, Copy, Clone)]
pub struct PreppedDrawImage<'a> {
    image_uniform_offset: u32,
    image: &'a GpuImage,
}

impl ImagePipeline {
    pub(crate) async fn new(
        device: &Device,
        modifier_uniform_bind_group_layout: &BindGroupLayout,
        clip_texture_bind_group_layout: &BindGroupLayout,
    ) -> Result<Self>
    {
        let image_vs_module = device
            .create_shader_module(&load_shader!("image.vert").await?);
        let image_fs_module = device
            .create_shader_module(&load_shader!("image.frag").await?);
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
        let image_texture_bind_group_layout = device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("image texture bind group layout"),
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
        let image_pipeline_layout = device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("image pipeline layout"),
                bind_group_layouts: &[
                    modifier_uniform_bind_group_layout,
                    clip_texture_bind_group_layout,
                    clip_texture_bind_group_layout,
                    &image_uniform_bind_group_layout,
                    &image_texture_bind_group_layout,
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
                        ColorTargetState {
                            format: SWAPCHAIN_FORMAT,
                            blend: Some(BlendState::ALPHA_BLENDING),
                            write_mask: ColorWrites::all(),
                        },
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

        let image_sampler = device
            .create_sampler(&SamplerDescriptor {
                label: Some("image sampler"),
                address_mode_u: AddressMode::Repeat,
                address_mode_v: AddressMode::Repeat,
                ..Default::default()
            });

        Ok(ImagePipeline {
            image_pipeline,
            image_texture_bind_group_layout,
            image_sampler,
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
        let image_uniform_offset = uniform_packer
            .pack(&ImageUniformData {
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
                &image.image.0.texture_bind_group,
                &[],
            );
        pass.draw(0..6, 0..1);
    }

    pub(crate) fn load_image(
        &self,
        device: &Device,
        queue: &Queue,
        image: &RgbaImage,
    ) -> GpuImage
    {
        let texture_format = TextureFormat::Rgba8Unorm;

        // create texture
        let texture = device
            .create_texture_with_data(
                &queue,
                &TextureDescriptor {
                    label: Some("image texture"),
                    size: Extent3d {
                        width: image.width(),
                        height: image.height(),
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: texture_format,
                    usage: TextureUsages::TEXTURE_BINDING,
                },
                image.as_raw(),
            );

        // create texture view
        let texture_view = texture
            .create_view(&TextureViewDescriptor {
                label: Some("image texture view"),
                ..Default::default()
            });

        // create bind group
        let texture_bind_group = device
            .create_bind_group(&BindGroupDescriptor {
                label: Some("image texture bind group"),
                layout: &self.image_texture_bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&texture_view),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&self.image_sampler),
                    },
                ],
            });

        // done
        GpuImage(Arc::new(GpuImageInner {
            texture_bind_group,
            size: Extent2::new(image.width(), image.height()),
        }))
    }
}
