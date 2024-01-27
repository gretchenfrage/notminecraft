
use crate::{
    resources::gpu_image::{
        GpuImageArrayManager,
        GpuImageArray,
    },
    vertex::{
        VertexStruct,
        vertex_struct,
    },
    shader::load_shader,
    SWAPCHAIN_FORMAT,
    DEPTH_FORMAT,
};
use std::{
    marker::PhantomData,
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
use vek::*;
use anyhow::*;
use tracing::*;


const INDEX_FORMAT: IndexFormat = IndexFormat::Uint32;


// ==== gpu vec ====

/// Vector-like resizable and updatable array of elements on the GPU,
/// comprising a GPU memory allocation, a length, and a capacity.
#[derive(Debug)]
pub struct GpuVec<T> {
    buffer: Option<Buffer>,
    // NOTE: length is in elements, but capacity is in bytes
    len: usize,
    capacity: usize,
    _p: PhantomData<T>,

    #[cfg(debug_assertions)]
    dbg_content: Vec<Option<T>>,
}

impl<T> GpuVec<T> {
    /// Construct an empty GPU vec.
    pub fn new() -> Self
    where
        T: GpuVecElem,
    {
        MeshPipeline::create_gpu_vec()
    }

    /// The current length, in elements.
    pub fn len(&self) -> usize {
        self.len
    }

    // TODO: continue to expose?
    #[cfg(debug_assertions)]
    pub fn dbg_content(&self) -> Option<&[Option<T>]> {
        Some(&self.dbg_content)
    }

    #[cfg(not(debug_assertions))]
    pub fn dbg_content(&self) -> Option<&[Option<T>]> {
        None
    }
}

impl<T: GpuVecElem> Default for GpuVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Types which can be stored in a `GpuVec`. Not intended to be implemented
/// externally.
pub trait GpuVecElem: Copy {
    const USAGES: BufferUsages;
    const SIZE: usize;

    fn write(&self, dst: &mut Vec<u8>);
}


// ==== mesh ====

/// Vertex and index data for a mesh, stored on the GPU.
#[derive(Debug, Default)]
pub struct Mesh {
    /// Vertices within the mesh. Are grouped together into triangles by the
    /// `indices` field.
    pub vertices: GpuVec<Vertex>,
    /// Groups of 3 vertices which form triangles. Each vertex index is an
    /// index into the `vertices` array. The `indices` array's length must be
    /// a multiple of 3 for the mesh to be valid, and each consecutive group of
    /// 3 vertex indices forms a triangle. 
    ///
    /// TODO: do they need to be clockwise or counter-clockwise?
    pub indices: GpuVec<usize>,
}

impl Mesh {
    /// Construct an empty mesh.
    pub fn new() -> Self {
        Self::default()
    }
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


// ==== pipeline ====

#[derive(Debug, Clone)]
pub struct DrawMesh<'a> {
    pub mesh: &'a Mesh,
    pub textures: GpuImageArray,
}

pub struct MeshPipeline {
    mesh_pipeline: RenderPipeline,
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
    pub(crate) fn new(
        device: &Device,
        modifier_uniform_bind_group_layout: &BindGroupLayout,
        clip_texture_bind_group_layout: &BindGroupLayout,
        gpu_image_manager: &GpuImageArrayManager,
    ) -> Result<Self>
    {
        let mesh_vs_module = device
            .create_shader_module(load_shader!("mesh.vert")?);
        let mesh_fs_module = device
            .create_shader_module(load_shader!("mesh.frag")?);
        
        let mesh_pipeline_layout = device
            .create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("mesh pipeline layout"),
                bind_group_layouts: &[
                    modifier_uniform_bind_group_layout,
                    clip_texture_bind_group_layout,
                    clip_texture_bind_group_layout,
                    &gpu_image_manager.gpu_image_bind_group_layout,
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
                        Some(ColorTargetState {
                            format: SWAPCHAIN_FORMAT,
                            blend: Some(BlendState::ALPHA_BLENDING),
                            write_mask: ColorWrites::all(),
                        }),
                    ],
                }),
                primitive: PrimitiveState {
                    front_face: FrontFace::Cw,
                    cull_mode: Some(Face::Back),
                    //cull_mode: None, // TODO: lol I don't understand handedness
                    ..Default::default()
                },
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
        

        Ok(MeshPipeline {
            mesh_pipeline,
        })
    }

    pub(crate) fn render<'a>(
        &'a self,
        mesh: &'a DrawMesh<'a>,
        pass: &mut RenderPass<'a>,
        _dbg_transform: Mat4<f32>,
    )
    {
        assert!(
            mesh.mesh.indices.len % 3 == 0,
            "attempt to render mesh with non-multiple of 3 number of indices",
        );

        #[cfg(debug_assertions)]
        if false { // TODO: come up with a better way to enable or disable
            let mut tex_index = None;
            for (
                vert_idx_idx,
                vert_idx,
            ) in mesh.mesh.indices.dbg_content.iter().copied().enumerate()
            {
                if vert_idx_idx % 3 == 0 {
                    tex_index = None;
                }

                let vert_idx = vert_idx
                    .expect("attempt to render mesh with uninitialized index element");
                assert!(
                    vert_idx < mesh.mesh.vertices.len,
                    "attempt to render mesh with vertex index beyond end of vertex gpu vec",
                );
                let vertex = mesh.mesh.vertices.dbg_content[vert_idx]
                    .expect("attempt to render mesh with vertex index referencing uninitialized vertex element");

                assert!(
                    vertex.tex_index < mesh.textures.0.len,
                    "attempt to render mesh with (non-unused) texture index beyond end of gpu image array",
                );

                if let Some(tex_index) = tex_index {
                    assert!(
                        vertex.tex_index == tex_index,
                        "attempt to render mesh with different texture indices within same triangle",
                    );
                }
                tex_index = Some(vertex.tex_index);
            }
        }

        /*
        use crate::modifier::Transform3;
        for &triangle in &mesh.mesh.triangles.dbg_content {
            let triangle = triangle.unwrap();
            let transed_tri_pos = triangle.0
                .map(|i| mesh.mesh.vertices.dbg_content[i].unwrap())
                .map(|v| v.pos)
                .map(|p| Transform3(_dbg_transform).apply(p)); // TODO why are we convering back and forth like this
            dbg!(transed_tri_pos);
        }*/
        

        if mesh.mesh.indices.len > 0 {
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
                    mesh.mesh.indices.buffer.as_ref().unwrap().slice(..),
                    INDEX_FORMAT,
                );
            pass
                .draw_indexed(
                    0..mesh.mesh.indices.len as u32,
                    0,
                    0..1,
                );
        }
    }

    pub(crate) fn create_gpu_vec<T: GpuVecElem>() -> GpuVec<T> {
        #[cfg(debug_assertions)]
        return GpuVec {
            buffer: None,
            len: 0,
            capacity: 0,
            _p: PhantomData,
            dbg_content: Vec::new(),
        };
        #[cfg(not(debug_assertions))]
        return GpuVec {
            buffer: None,
            len: 0,
            capacity: 0,
            _p: PhantomData,
        };
    }

    pub(crate) fn set_gpu_vec_len<T: GpuVecElem>(
        device: &Device,
        queue: &Queue,
        gpu_vec: &mut GpuVec<T>,
        new_len: usize,
    ) -> Option<SubmissionIndex>
    {
        let submission_index =
            if new_len * T::SIZE > gpu_vec.capacity  {
                while new_len * T::SIZE > gpu_vec.capacity {
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
                let submission_index =
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
                                (gpu_vec.len * T::SIZE) as u64,
                            );
                        Some(queue.submit(once(encoder.finish())))
                    } else {
                        None
                    };
                gpu_vec.buffer = Some(new_buffer);
                submission_index
            } else {
                None
            };
        gpu_vec.len = new_len;

        #[cfg(debug_assertions)]
        {
            while gpu_vec.dbg_content.len() > gpu_vec.len {
                gpu_vec.dbg_content.pop().unwrap();
            }
            while gpu_vec.dbg_content.len() < gpu_vec.len {
                gpu_vec.dbg_content.push(None);
            }
        }

        submission_index
    }

    pub fn patch_gpu_vec<T>(
        device: &Device,
        queue: &Queue,
        gpu_vec: &mut GpuVec<T>,
        patches: &[(usize, &[T])],
    ) -> Option<SubmissionIndex>
    where
        T: GpuVecElem,
    {
        struct CopyRange {
            src_byte_offset: u64,
            dst_byte_offset: u64,
            num_bytes: u64,
        }

        let mut src_byte_data = Vec::new();
        let mut copy_ranges = Vec::new();

        #[cfg(debug_assertions)]
        let mut dbg_to_set = Vec::new();

        for &(dst_elem_index, patch) in patches {
            let src_byte_offset = (src_byte_data.len()) as u64;

            let mut patch_len = 0;
            for (_i, &elem) in patch.into_iter().enumerate() {
                elem.write(&mut src_byte_data);
                patch_len += 1;

                #[cfg(debug_assertions)]
                dbg_to_set.push((dst_elem_index + _i, elem));
            }

            copy_ranges.push(CopyRange {
                src_byte_offset,
                dst_byte_offset: (dst_elem_index * T::SIZE) as u64,
                num_bytes: (patch_len * T::SIZE) as u64,
            });
        }

        for copy_range in &copy_ranges {
            assert!(
                copy_range.dst_byte_offset + copy_range.num_bytes
                <= (gpu_vec.len * T::SIZE) as u64,
                "GpuVec patch exceeds length",
            );
        }

        if copy_ranges.is_empty() {
            return None;
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
        let submission_index = queue.submit(once(encoder.finish()));

        #[cfg(debug_assertions)]
        for (i, elem) in dbg_to_set {
            gpu_vec.dbg_content[i] = Some(elem);
        }

        Some(submission_index)
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

impl GpuVecElem for usize {
    const USAGES: BufferUsages = BufferUsages::INDEX;
    const SIZE: usize = size_of::<u32>();

    fn write(&self, dst: &mut Vec<u8>) {
        dst.extend(u32::to_le_bytes(*self as u32));
    }
}
