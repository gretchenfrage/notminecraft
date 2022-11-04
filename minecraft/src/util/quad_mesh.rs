
use graphics::{
    Renderer,
    frame_content::{
        Mesh,
        Vertex,
        Triangle,
    },
};
use std::borrow::Borrow;
use vek::*;


/// Utility for meshes constituting planar quads.
///
/// Provides an API similar to how a `GpuMesh<Quad>` would be. Converts each
/// quad to 4 vertices and 2 triangles. This ends up more efficient than
/// naively using 6 vertices.
#[derive(Debug)]
pub struct QuadMesh(pub Mesh);

impl Borrow<Mesh> for QuadMesh {
    fn borrow(&self) -> &Mesh {
        &self.0
    }
}

/// Quad within a `QuadMesh`.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Quad {
    /// Pos of bottom-left corner
    pub pos_start: Vec3<f32>,
    /// Pos difference from bottom-left to top-left corner
    pub pos_ext_1: Extent3<f32>,
    /// Pos difference from bottom-left to bottom-right corner
    pub pos_ext_2: Extent3<f32>,

    /// Tex of top-left corner
    pub tex_start: Vec2<f32>,
    /// Tex difference from top-left to bottom-right corner
    pub tex_extent: Extent2<f32>,

    /// Colors of vertices, starting bottom-left and going clockwise
    pub vert_colors: [Rgba<f32>; 4],

    /// Texture index
    pub tex_index: usize,
}

const VERTS_PER_QUAD: usize = 4;
const TRIS_PER_QUAD: usize = 2;

fn to_verts(quad: &Quad) -> [Vertex; VERTS_PER_QUAD] {
    [
        // bottom-left
        Vertex {
            pos: quad.pos_start,
            tex: quad.tex_start + Vec2::new(0.0, quad.tex_extent.h),
            color: quad.vert_colors[0],
            tex_index: quad.tex_index,
        },
        // top-left
        Vertex {
            pos: quad.pos_start + quad.pos_ext_1,
            tex: quad.tex_start,
            color: quad.vert_colors[1],
            tex_index: quad.tex_index,
        },
        // top-right
        Vertex {
            pos: quad.pos_start + quad.pos_ext_1 + quad.pos_ext_2,
            tex: quad.tex_start + Vec2::new(quad.tex_extent.w, 0.0),
            color: quad.vert_colors[2],
            tex_index: quad.tex_index,
        },
        // bottom-right
        Vertex {
            pos: quad.pos_start + quad.pos_ext_2,
            tex: quad.tex_start + quad.tex_extent,
            color: quad.vert_colors[3],
            tex_index: quad.tex_index,
        },
    ]
}

fn to_triangles(quad_idx: usize) -> [Triangle; TRIS_PER_QUAD] {
    [
        // bottom-left triangle
        Triangle([0, 1, 3]),
        // top-right triangle
        Triangle([3, 1, 2]),
    ]
        .map(|tri| tri.map(|k| quad_idx * TRIS_PER_QUAD * 2 + k))
}


impl QuadMesh {
    pub fn create(renderer: &Renderer) -> Self {
        QuadMesh(Mesh {
            vertices: renderer.create_gpu_vec(),
            triangles: renderer.create_gpu_vec(),
        })
    }

    pub fn create_init<I>(renderer: &Renderer, content: I) -> Self
    where
        I: IntoIterator + Clone,
        <I as IntoIterator>::IntoIter: Clone,
        <I as IntoIterator>::Item: Borrow<Quad>,
    {
        QuadMesh(Mesh {
            vertices: renderer.create_gpu_vec_init(
                content
                    .clone()
                    .into_iter()
                    .flat_map(|quad| to_verts(quad.borrow()))
            ),
            triangles: renderer.create_gpu_vec_init(
                (0..content.into_iter().count())
                    .flat_map(|quad_idx| to_triangles(quad_idx))
            ),
        })
        // TODO efficiency
    }


    pub fn set_len(&mut self, renderer: &Renderer, new_len: usize) {
        renderer.set_gpu_vec_len(&mut self.0.vertices, new_len * VERTS_PER_QUAD);
        renderer.set_gpu_vec_len(&mut self.0.triangles, new_len * TRIS_PER_QUAD);
    }

    pub fn patch(
        &mut self,
        renderer: &Renderer,
        patches: &[(usize, &[Quad])],
    ) {
        self
            .patch_iters(
                renderer,
                patches
                    .iter()
                    .map(|&(i, patch)| (
                        i,
                        patch.iter().copied(),
                    )),
            )
    }

    pub fn patch_iters<I1, I2>(&mut self, renderer: &Renderer, patches: I1)
    where
        I1: IntoIterator<Item=(usize, I2)> + Clone,
        I2: IntoIterator<Item=Quad>,
    {
        renderer
            .patch_gpu_vec_iters(
                &mut self.0.vertices,
                patches
                    .clone()
                    .into_iter()
                    .map(|(i, patch)| (
                        i * VERTS_PER_QUAD,
                        patch
                            .into_iter()
                            .flat_map(|quad| to_verts(&quad)),
                    )),
            );
        renderer
            .patch_gpu_vec_iters(
                &mut self.0.triangles,
                patches
                    .into_iter()
                    .map(|(i, patch)| (
                        i * TRIS_PER_QUAD,
                        (0..patch.into_iter().count())
                            .flat_map(move |j| to_triangles(i + j)),
                    )),
            );
    }
}
