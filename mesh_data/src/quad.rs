
use graphics::frame_content::Vertex;
use vek::*;


pub const VERTICES_PER_QUAD: usize = 4;
pub const INDICES_PER_QUAD: usize = 6;


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

impl Quad {
    pub fn to_vertices(&self) -> [Vertex; VERTICES_PER_QUAD] {
        [
            // bottom-left
            Vertex {
                pos: self.pos_start,
                tex: self.tex_start + Vec2::new(0.0, self.tex_extent.h),
                color: self.vert_colors[0],
                tex_index: self.tex_index,
            },
            // top-left
            Vertex {
                pos: self.pos_start + self.pos_ext_1,
                tex: self.tex_start,
                color: self.vert_colors[1],
                tex_index: self.tex_index,
            },
            // top-right
            Vertex {
                pos: self.pos_start + self.pos_ext_1 + self.pos_ext_2,
                tex: self.tex_start + Vec2::new(self.tex_extent.w, 0.0),
                color: self.vert_colors[2],
                tex_index: self.tex_index,
            },
            // bottom-right
            Vertex {
                pos: self.pos_start + self.pos_ext_2,
                tex: self.tex_start + self.tex_extent,
                color: self.vert_colors[3],
                tex_index: self.tex_index,
            },
        ]
    }
}

pub const QUAD_INDICES: [usize; INDICES_PER_QUAD] = [
    0, 1, 3,
    3, 1, 2,
];
