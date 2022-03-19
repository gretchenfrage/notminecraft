//! 2D Pipeline for drawing an image.

use crate::{
    SWAPCHAIN_FORMAT,
    Canvas2d,
    Canvas2dDrawCall,
    Canvas2dOutVars,
    UniformBufferState,
    shader::load_shader,
};
use std::sync::Arc;
use wgpu::{
    *,
    util::DeviceExt,
};
use vek::*;
use anyhow::Result;


pub struct ImagePipeline {
    image_pipeline: RenderPipeline,
    image_texture_bind_group_layout: BindGroupLayout,
    image_sampler: Sampler,
}

pub struct DrawCallImage {
    uniform_offset: usize,
    image_index: usize,
}

pub fn prep_draw_image_call(canvas: &mut Canvas2d, image: &GpuImage) {
    // push uniform data
    let uniform_offset = canvas.push_uniform_data();

    // push image
    let image_index = canvas.out_vars.image_array.len();
    canvas.out_vars.image_array.push(image.clone());

    // push draw call
    let call = DrawCallImage {
        uniform_offset,
        image_index,
    };
    canvas.out_vars.draw_calls.push(Canvas2dDrawCall::Image(call));
}

impl ImagePipeline {
    pub(crate) async fn new(
        device: &Device,
        canvas2d_uniform_bind_group_layout: &BindGroupLayout,
    ) -> Result<Self> {
        let image_vs_module = device
            .create_shader_module(&load_shader("image.vert").await?);
        let image_fs_module = device
            .create_shader_module(&load_shader("image.frag").await?);
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
                    &canvas2d_uniform_bind_group_layout,
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
                depth_stencil: None,
                multisample: MultisampleState::default(),
                multiview: None,
            });
        let image_sampler = device
            .create_sampler(&SamplerDescriptor {
                label: Some("image sampler"),
                ..Default::default()
            });

        Ok(ImagePipeline {
            image_pipeline,
            image_texture_bind_group_layout,
            image_sampler,
        })
    }

    pub(crate) fn render_call<'a>(
        &'a self,
        call: DrawCallImage,
        pass: &mut RenderPass<'a>,
        uniform_buffer_state: &'a Option<UniformBufferState>,
        canvas_out_vars: &'a Canvas2dOutVars,
    ) {
        let uniform_buffer_state = uniform_buffer_state
            .as_ref()
            .unwrap();

        let image = &canvas_out_vars.image_array[call.image_index];
        pass.set_pipeline(&self.image_pipeline);
        pass.set_bind_group(
            0,
            &uniform_buffer_state.canvas2d_uniform_bind_group,
            &[call.uniform_offset as u32],
        );
        pass.set_bind_group(
            1,
            &image.0.texture_bind_group,
            &[],
        );
        pass.draw(0..6, 0..1);
    }

    pub(crate) fn load_image(
        &self,
        file_data: &[u8],
        device: &Device,
        queue: &Queue,
    ) -> Result<GpuImage> {
        let texture_format = TextureFormat::Rgba8Unorm;

        // load image
        let image = image::load_from_memory(file_data)?
            .into_rgba8();

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
        Ok(GpuImage(Arc::new(GpuImageInner {
            texture_bind_group,
            size: Extent2::new(image.width(), image.height()),
        })))
    }
}


/// 2D RGBA image loaded into a GPU texture.
///
/// Internally reference-counted.
#[derive(Clone)]
pub struct GpuImage(Arc<GpuImageInner>);

struct GpuImageInner {
    size: Extent2<u32>,
    texture_bind_group: BindGroup,
}

impl GpuImage {
    /// Get image size in pixels.
    pub fn size(&self) -> Extent2<u32> {
        self.0.size
    }
}
