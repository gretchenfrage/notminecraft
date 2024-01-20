
use std::{
    sync::Arc,
    borrow::Borrow,
};
use wgpu::{
    *,
    util::{
        DeviceExt,
        TextureDataOrder,
    },
};
use image::{
    DynamicImage,
    imageops::{
        self,
        FilterType,
    },
};
use vek::*;


#[derive(Debug)]
pub struct GpuImageArrayManager {
    pub gpu_image_bind_group_layout: BindGroupLayout,
    pub gpu_image_sampler: Sampler,
}

impl GpuImageArrayManager {
    pub(crate) fn new(device: &Device) -> Self {
        let gpu_image_bind_group_layout = device
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("mesh texture bind group layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float {
                                filterable: false,
                            },
                            view_dimension: TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                        count: None,
                    }
                ],
            });
        let gpu_image_sampler = device
            .create_sampler(&SamplerDescriptor {
                label: Some("mesh sampler"),
                address_mode_u: AddressMode::Repeat,
                address_mode_v: AddressMode::Repeat,
                ..Default::default()
            });
        GpuImageArrayManager {
            gpu_image_bind_group_layout,
            gpu_image_sampler,
        }
    }

    pub fn load_image_array<I>(
        &self,
        device: &Device,
        queue: &Queue,
        size: Extent2<u32>,
        images: I,
    ) -> GpuImageArray
    where
        I: IntoIterator,
        <I as IntoIterator>::Item: Borrow<DynamicImage>,
    {
        let mut len = 0;
        let mut image_data = Vec::new();
        
        for image in images {
            len += 1;

            let mut image = image
                .borrow()
                .to_rgba8();
            if Extent2::new(image.width(), image.height()) != size {
                image = imageops::resize(
                    &image,
                    size.w,
                    size.h,
                    FilterType::Nearest,
                );
            }
            image_data.extend(image.as_raw());
        }

        if len == 0 {
            return GpuImageArray(Arc::new(GpuImageArrayInner {
                size,
                len,
                texture_bind_group: None,
            }));
        }

        let texture = device
            .create_texture_with_data(
                queue,
                &TextureDescriptor {
                    label: Some("image array"),
                    size: Extent3d {
                        width: size.w,
                        height: size.h,
                        depth_or_array_layers: len as u32,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Rgba8Unorm,
                    usage: TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                },
                TextureDataOrder::LayerMajor,
                &image_data,
            );

        let texture_view = texture
            .create_view(&TextureViewDescriptor {
                label: Some("image array texture view"),
                dimension: Some(TextureViewDimension::D2Array),
                ..Default::default()
            });

        let texture_bind_group = device
            .create_bind_group(&BindGroupDescriptor {
                label: Some("image array texture bind group"),
                layout: &self.gpu_image_bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&texture_view),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&self.gpu_image_sampler),
                    },
                ],
            });

        GpuImageArray(Arc::new(GpuImageArrayInner {
            size,
            len,
            texture_bind_group: Some(texture_bind_group),
        }))
    }
}

/// Array of 2D RGBA images of the same size loaded into a GPU texture.
///
/// Internally reference-counted.
#[derive(Debug, Clone)]
pub struct GpuImageArray(pub(crate) Arc<GpuImageArrayInner>);

#[derive(Debug)]
pub struct GpuImageArrayInner {
    pub size: Extent2<u32>,
    pub len: usize,
    pub texture_bind_group: Option<BindGroup>,
}

impl GpuImageArray {
    /// Get image size in pixels.
    ///
    /// Guaranteed to not have 0 components.
    pub fn size(&self) -> Extent2<u32> {
        self.0.size
    }

    /// Get length of array.
    pub fn len(&self) -> usize {
        self.0.len
    }
}
