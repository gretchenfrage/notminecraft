
use mesh_data::{
    MeshData,
    Quad,
};
use vek::*;


pub fn block_item_mesh() -> MeshData {
    let mut mesh_buf = MeshData::new();
    let shade = 0.5;
    mesh_buf
        .add_quad(&Quad {
            pos_start: [0.0, 0.0, 0.0].into(),
            pos_ext_1: [0.0, 1.0, 0.0].into(),
            pos_ext_2: [1.0, 0.0, 0.0].into(),
            tex_start: 0.0.into(),
            tex_extent: 1.0.into(),
            vert_colors: [[shade, shade, shade, 1.0].into(); 4],
            tex_index: 0,
        });
    mesh_buf
        .add_quad(&Quad {
            pos_start: [1.0, 0.0, 0.0].into(),
            pos_ext_1: [0.0, 1.0, 0.0].into(),
            pos_ext_2: [0.0, 0.0, 1.0].into(),
            tex_start: 0.0.into(),
            tex_extent: 1.0.into(),
            vert_colors: [[shade, shade, shade, 1.0].into(); 4],
            tex_index: 0,
        });
    mesh_buf
        .add_quad(&Quad {
            pos_start: [0.0, 1.0, 0.0].into(),
            pos_ext_1: [0.0, 0.0, 1.0].into(),
            pos_ext_2: [1.0, 0.0, 0.0].into(),
            tex_start: 0.0.into(),
            tex_extent: 1.0.into(),
            vert_colors: [Rgba::white(); 4],
            tex_index: 0,
        });
    mesh_buf
}
