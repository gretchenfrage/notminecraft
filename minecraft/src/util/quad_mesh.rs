
use graphics::{
    Renderer,
    frame_content::Mesh,
};
use vek::*;


/// Utility for meshes constituting planar quads.
///
/// Provides an API similar to how a `GpuMesh<Quad>` would be. Converts each
/// quad to 4 vertices and 6 indices. Since indices are much smaller than
/// vertices, this ends up more efficient than naively using 6 vertices.
#[derive(Debug)]
pub struct QuadMesh(pub Mesh);

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
    pub vert_colorss: [Rgba<f32>; 4],

    /// Texture index
    pub tex_index: usize,
}


impl QuadMesh {
    pub fn create(renderer: &Renderer) -> Self {
        QuadMesh(Mesh {
            vertices: renderer.create_gpu_vec(),
            triangles: renderer.create_gpu_vec(),
        })
    }



    pub fn set_len(&mut self, renderer: &Renderer, new_len: usize) {
        renderer.set_gpu_vec_len(&mut self.0.vertices, new_len * 4);
        renderer.set_gpu_vec_len(&mut self.0.triangles, new_len * 6);
    }

    pub fn patch_gpu_vec()

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
                        i * 4,
                        patch
                            .into_iter()
                            .flat_map(|quad| [
                                // bottom-left
                                MeshVertex {
                                    pos: quad.pos_start,
                                    tex: quad.tex_start + Vec2::new(0.0, quad.tex_extent.y),
                                    colors: quad.vert_colorss[0],
                                    tex_index: quad.tex_index,
                                },
                                // top-left
                                MeshVertex {
                                    pos: quad.pos_start + quad.pos_ext_1,
                                    tex: quad.tex_start,
                                    colors: quad.vert_colorss[1],
                                    tex_index: quad.tex_index,
                                },
                                // top-right
                                MeshVertex {
                                    pos: quad.pos_start + quad.pos_ext_1 + quad.pos_ext_2,
                                    tex: quad.tex_start + Vec2::new(quad.tex_extent.x, 0.0),
                                    colors: quad.vert_colorss[2],
                                    tex_index: quad.tex_index,
                                },
                                // bottom-right
                                MeshVertex {
                                    pos: quad.pos_start + quad.pos_ext_2,
                                    tex: quad.tex_start + quad.tex_extent,
                                    colors: quad.vert_colorss[3],
                                    tex_index: quad.tex_index,
                                },
                            ]),
                    )),
            );
        renderer
            .patch_gpu_vec_iters(
                &mut self.0.indices,
                patches
                    .into_iter()
                    .map(|(i, patch)| (
                        i * 6,
                        (0..patch.into_iter().count())
                            .flat_map(|j|
                                [
                                    // bottom-left triangle
                                    0,
                                    1,
                                    3,
                                    // top-right triangle
                                    3,
                                    1,
                                    2,
                                ]
                                .map(|k| (i + j) * 6 + k)
                            ),
                    )),
            );
    }
}
