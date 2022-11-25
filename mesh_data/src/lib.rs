
#[allow(unused_imports)]
#[macro_use]
extern crate tracing;


mod quad;
mod differ;


use graphics::{
    Renderer,
    frame_content::{
        Vertex,
        Mesh,
    },
};

pub use crate::{
    quad::{
        Quad,
        QUAD_INDICES,
    },
    differ::{
        MeshDiffer,
        GpuVecDiff,
    },
};


#[derive(Debug, Clone, PartialEq, Default)]
pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<usize>,
}

impl MeshData {
    pub fn new() -> Self {
        MeshData::default()
    }

    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty() && self.indices.is_empty()
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }

    pub fn extend<V, I>(&mut self, submesh_vertices: V, submesh_indices: I)
    where
        V: IntoIterator<Item=Vertex>,
        I: IntoIterator<Item=usize>,
    {
        let start_num_vertices = self.vertices.len();
        self.vertices.extend(submesh_vertices);
        let indices = submesh_indices
            .into_iter()
            .map(|mut vert_idx| {
                vert_idx += start_num_vertices;
                debug_assert!(
                    vert_idx < self.vertices.len(),
                    "index extends beyond own submesh",
                );
                vert_idx
            });
        self.indices.extend(indices);
        debug_assert!(
            self.indices.len() % 3 == 0,
            "submesh contains non-multiple of 3 number of indices",
        );
    }

    pub fn add_quad(&mut self, quad: &Quad) {
        self.extend(quad.to_vertices(), QUAD_INDICES);
    }

    pub fn upload(&self, renderer: &Renderer) -> Mesh {
        Mesh {
            vertices: renderer.create_gpu_vec_init(&self.vertices),
            indices: renderer.create_gpu_vec_init(&self.indices),
        }
    }

    pub fn validate_indices(&self) {
        assert!(self.indices.len() % 3 == 0);
        for &index in &self.indices {
            assert!(index < self.vertices.len());
        }
    }

    pub fn triangles<'s>(&'s self) -> impl Iterator<Item=[usize; 3]> + 's
    {
        self.indices
            .chunks(3)
            .map(|chunk| [chunk[0], chunk[1], chunk[2]])
    }
}
