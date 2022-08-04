
use crate::{
    vertex::{
        VertexStruct,
        vertex_struct,
    },
    shader::load_shader,
    SWAPCHAIN_FORMAT,
};
use std::{
    sync::Arc,
    marker::PhantomData,
    borrow::Borrow,
    iter::once,
    mem::size_of,
};
use wgpu::{
    *,
    util::{
        DeviceExt,
        BufferInitDescriptor,
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
use anyhow::*;
use tracing::*;


const INDEX_FORMAT: IndexFormat = IndexFormat::Uint32;
const INDICES_PER_TRIANGLE: usize = 3;


// ==== gpu vec ====

/// Vector-like resizable and updatable array of elements on the GPU,
/// comprising a GPU memory allocation, a length, and a capacity.
#[derive(Debug)]
pub struct GpuVec<T> {
    buffer: Option<Buffer>,
    len: usize,
    capacity: usize,
    _p: PhantomData<T>,
}

impl<T> GpuVec<T> {
    /// The current length, in elements.
    pub fn len(&self) -> usize {
        self.len
    }
}

/// Types which can be stored in a `GpuVec`. Not intended to be implemented
/// externally.
pub trait GpuVecElem {
    const USAGES: BufferUsages;
    const SIZE: usize;

    fn write(&self, dst: &mut Vec<u8>);
}


// ==== gpu image array ====

/// Array of 2D RGBA images of the same size loaded into a GPU texture.
///
/// Internally reference-counted.
#[derive(Debug, Clone)]
pub struct GpuImageArray(Arc<GpuImageArrayInner>);

#[derive(Debug)]
struct GpuImageArrayInner {
    size: Extent2<u32>,
    len: usize,
    texture_bind_group: Option<BindGroup>,

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


// ==== mesh ====

/// Vertex and index data for a mesh, stored on the GPU.
#[derive(Debug)]
pub struct Mesh {
    /// Vertices within the mesh. Are grouped together into triangles by the
    /// `triangles` field.
    pub vertices: GpuVec<Vertex>,
    /// Groups of 3 vertices which form triangles. Each `Triangle` is an array
    /// of 3 indices of the `vertices` array.
    ///
    /// TODO: do they need to be clockwise or counter-clockwise?
    pub triangles: GpuVec<Triangle>,
}

/// Vertex within a `Mesh`.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Vertex {
    /// Vertex position in 3D space.
    pub pos: Vec3<f32>,
    /// Vertex texture coordinates, wherein <0,0> is the top-left of the
    /// texture, and <1,1> is the bottom-right of the texture.
    pub tex: Vec2<f32>,
    /// Vertex color. Typical complications for 3D semitransparency apply.
    pub color: Rgba<f32>,
    /// Texture index within the texture array. For the mesh to be valid,
    /// `tex_index` must be the same between all vertices within each triangle.
    pub tex_index: usize,
}

/// Triangle within a `Mesh`.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Triangle(pub [usize; 3]);


// ==== pipeline ====

#[derive(Debug, Clone)]
pub struct DrawMesh<'a> {
    pub mesh: &'a Mesh,
    pub textures: GpuImageArray,
}

pub struct MeshPipeline {
    mesh_pipeline: RenderPipeline,
    mesh_texture_bind_group_layout: BindGroupLayout,
    mesh_texture_sampler: Sampler,
}

struct MeshVertex {
    pos: Vec3<f32>,
    tex: Vec3<f32>,
    color: Rgba<u8>,
}

vertex_struct!(MeshVertex {
    (pos:  Vec3<f32>) (layout(location=0) in vec3),
    (tex:  Vec3<f32>) (layout(location=1) in vec3),
    (color: Rgba<u8>) (layout(location=2) in vec4),
});

impl MeshPipeline {
    pub(crate) async fn new(
        device: &Device,
        modifier_uniform_bind_group_layout: &BindGroupLayout,
        clip_texture_bind_group_layout: &BindGroupLayout,
    ) -> Result<Self>
    {
        let mesh_vs_module = device
            .create_shader_module(&load_shader!("mesh.vert").await?);
        let mesh_fs_module = device
            .create_shader_module(&load_shader!("mesh.frag").await?);
        let mesh_texture_bind_group_layout = device
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
        let mesh_pipeline_layout = device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("mesh pipeline layout"),
                bind_group_layouts: &[
                    modifier_uniform_bind_group_layout,
                    clip_texture_bind_group_layout,
                    clip_texture_bind_group_layout,
                    &mesh_texture_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
        let mesh_pipeline = device
            .create_render_pipeline(&RenderPipelineDescriptor {
                label: Some("mesh pipeline"),
                layout: Some(&mesh_pipeline_layout),
                vertex: VertexState {
                    module: &mesh_vs_module,
                    entry_point: "main",
                    buffers: &[
                        VertexBufferLayout {
                            array_stride: MeshVertex::SIZE as u64,
                            step_mode: VertexStepMode::Vertex,
                            attributes: MeshVertex::ATTRIBUTES,
                        },
                    ],
                },
                fragment: Some(FragmentState {
                    module: &mesh_fs_module,
                    entry_point: "main",
                    targets: &[
                        ColorTargetState {
                            format: SWAPCHAIN_FORMAT,
                            blend: Some(BlendState::ALPHA_BLENDING),
                            write_mask: ColorWrites::all(),
                        }
                    ],
                }),
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
                multiview: None,
            });
        let mesh_texture_sampler = device
            .create_sampler(&SamplerDescriptor {
                label: Some("mesh sampler"),
                address_mode_u: AddressMode::Repeat,
                address_mode_v: AddressMode::Repeat,
                ..Default::default()
            });

        Ok(MeshPipeline {
            mesh_pipeline,
            mesh_texture_bind_group_layout,
            mesh_texture_sampler,
        })
    }

    pub(crate) fn render<'a>(
        &'a self,
        mesh: &'a DrawMesh<'a>,
        pass: &mut RenderPass<'a>,
    )
    {
        if mesh.mesh.triangles.len > 0 {
            pass.set_pipeline(&self.mesh_pipeline);
            pass
                .set_bind_group(
                    3,
                    mesh.textures.0.texture_bind_group.as_ref().unwrap(),
                    &[],
                );
            pass
                .set_vertex_buffer(
                    0,
                    mesh.mesh.vertices.buffer.as_ref().unwrap().slice(..),
                );
            pass
                .set_index_buffer(
                    mesh.mesh.triangles.buffer.as_ref().unwrap().slice(..),
                    INDEX_FORMAT,
                );
            pass
                .draw_indexed(
                    0..(mesh.mesh.triangles.len * INDICES_PER_TRIANGLE) as u32,
                    0,
                    0..1,
                );
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
                },
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
                layout: &self.mesh_texture_bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&texture_view),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&self.mesh_texture_sampler),
                    },
                ],
            });

        GpuImageArray(Arc::new(GpuImageArrayInner {
            size,
            len,
            texture_bind_group: Some(texture_bind_group),
        }))
    }

/*
impl crate::Renderer {
    pub fn create_gpu_vec<T: GpuVecElem>(&self) -> GpuVec<T> {}

    pub fn set_gpu_vec_len<T: GpuVecElem>(
        &self,
        gpu_vec: &mut GpuVec<T>,
        new_len: u32,
    ) {}

    pub fn patch_gpu_vec<T: GpuVecElem>(
        &self,
        gpu_vec: &mut GpuVec<T>,
        patches: &[(u32, &[T])],
    ) {}
}
*/
    pub(crate) fn create_gpu_vec<T: GpuVecElem>() -> GpuVec<T> {
        GpuVec {
            buffer: None,
            len: 0,
            capacity: 0,
            _p: PhantomData,
        }
    }

    pub(crate) fn set_gpu_vec_len<T: GpuVecElem>(
        device: &Device,
        queue: &Queue,
        gpu_vec: &mut GpuVec<T>,
        new_len: usize,
    )
    {
        if new_len > gpu_vec.capacity  {
            while new_len > gpu_vec.capacity {
                if gpu_vec.capacity == 0 {
                    const GPU_VEC_STARTING_CAPACITY: usize = 512;
                    gpu_vec.capacity = GPU_VEC_STARTING_CAPACITY;
                } else {
                    gpu_vec.capacity *= 2;
                }
            }
            trace!("increasing gpu vec capacity");
            let new_buffer = device
                .create_buffer(&BufferDescriptor {
                    label: Some("mesh buffer"),
                    size: gpu_vec.capacity as u64,
                    usage: BufferUsages::COPY_SRC
                        | BufferUsages::COPY_DST
                        | T::USAGES,
                    mapped_at_creation: false,
                });
            if gpu_vec.len != 0 {
                let mut encoder = device
                    .create_command_encoder(&CommandEncoderDescriptor {
                        label: Some("upsize GpuVec command encoder"),
                    });
                encoder
                    .copy_buffer_to_buffer(
                        gpu_vec.buffer.as_ref().unwrap(),
                        0,
                        &new_buffer,
                        0,
                        gpu_vec.len as u64,
                    );
                queue.submit(once(encoder.finish()));
            }
            gpu_vec.buffer = Some(new_buffer);
        }
        gpu_vec.len = new_len;
    }

    pub fn patch_gpu_vec<T: GpuVecElem>(
        device: &Device,
        queue: &Queue,
        gpu_vec: &mut GpuVec<T>,
        patches: &[(usize, &[T])],
    )
    {
        struct CopyRange {
            src_byte_offset: u64,
            dst_byte_offset: u64,
            num_bytes: u64,
        }

        let mut src_byte_data = Vec::new();
        let mut copy_ranges = Vec::new();

        for &(dst_elem_index, patch) in patches {
            copy_ranges.push(CopyRange {
                src_byte_offset: (src_byte_data.len()) as u64,
                dst_byte_offset: (dst_elem_index * T::SIZE) as u64,
                num_bytes: (patch.len() * T::SIZE) as u64,
            });
            for elem in patch {
                elem.write(&mut src_byte_data);
            }
        }

        for copy_range in &copy_ranges {
            assert!(
                copy_range.dst_byte_offset + copy_range.num_bytes
                <= (gpu_vec.len * T::SIZE) as u64,
                "GpuVec patch exceeds length",
            );
        }

        if copy_ranges.is_empty() {
            return;
        }

        let src_buffer = device
            .create_buffer_init(&BufferInitDescriptor {
                label: Some("patch GpuVec src buffer"),
                contents: &src_byte_data,
                usage: BufferUsages::COPY_SRC,
            });
        let mut encoder = device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("patch GpuVec command encoder"),
            });
        for copy_range in copy_ranges {
            if copy_range.num_bytes == 0 {
                continue;
            }
            encoder
                .copy_buffer_to_buffer(
                    &src_buffer,
                    copy_range.src_byte_offset,
                    &gpu_vec.buffer.as_ref().unwrap(),
                    copy_range.dst_byte_offset,
                    copy_range.num_bytes,
                );
        }
        queue.submit(once(encoder.finish()));
    }
}

impl GpuVecElem for Vertex {
    const USAGES: BufferUsages = BufferUsages::VERTEX;
    const SIZE: usize = MeshVertex::SIZE;

    fn write(&self, dst: &mut Vec<u8>) {
        MeshVertex {
            pos: self.pos,
            tex: Vec3 {
                x: self.tex.x,
                y: self.tex.y,
                z: self.tex_index as f32,
            },
            color: self.color.map(|n| (n * 255.0) as u8),
        }.write(dst)
    }
}

impl GpuVecElem for Triangle {
    const USAGES: BufferUsages = BufferUsages::INDEX;
    const SIZE: usize = size_of::<u32>() * INDICES_PER_TRIANGLE;

    fn write(&self, dst: &mut Vec<u8>) {
        for index in self.0 {
            dst.extend(u32::to_le_bytes(index as u32));
        }
    }
}
