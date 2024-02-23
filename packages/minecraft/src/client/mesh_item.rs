//! Meshing item models.

use crate::{
    game_data::{
        item_mesh_logic::ItemMeshLogic,
        per_item::PerItem,
        *,
    },
    item::*,
};
use graphics::prelude::*;
use mesh_data::*;
use std::sync::Arc;
use vek::*;


/// Generate and upload the meshes for all item models.
pub fn create_item_meshes(
    game: &Arc<GameData>,
    gpu_vec_ctx: &impl GpuVecContext,
) -> PerItem<Mesh> {
    let mut item_meshes = PerItem::new_no_default();
    let mut mesh_buf = MeshData::new();
    for iid in game.items.iter() {
        mesh_item(&mut mesh_buf, iid, &game);
        item_meshes.set(iid, mesh_buf.upload(gpu_vec_ctx));
        mesh_buf.clear();
    }
    item_meshes
}

/// Generate the mesh for a single item model. 
pub fn mesh_item(
    mesh_buf: &mut MeshData,
    iid: RawItemId,
    game: &Arc<GameData>,
) {
    match &game.items_mesh_logic[iid] {
        &ItemMeshLogic::FullCube {
            top_tex_index,
            left_tex_index,
            right_tex_index,
        } => {
            const LEFT_SHADE: f32 = 0x48 as f32 / 0x8f as f32;
            const RIGHT_SHADE: f32 = 0x39 as f32 / 0x8f as f32;

            mesh_buf.add_quad(&Quad {
                pos_start: [1.0, 1.0, 0.0].into(),
                pos_ext_1: [-1.0, 0.0, 0.0].into(),
                pos_ext_2: [0.0, 0.0, 1.0].into(),
                tex_start: 0.0.into(),
                tex_extent: 1.0.into(),
                vert_colors: [Rgba::white(); 4],
                tex_index: top_tex_index,
            });
            mesh_buf.add_quad(&Quad {
                pos_start: [0.0, 0.0, 0.0].into(),
                pos_ext_1: [0.0, 1.0, 0.0].into(),
                pos_ext_2: [1.0, 0.0, 0.0].into(),
                tex_start: 0.0.into(),
                tex_extent: 1.0.into(),
                vert_colors: [[LEFT_SHADE, LEFT_SHADE, LEFT_SHADE, 1.0].into(); 4],
                tex_index: left_tex_index,
            });
            mesh_buf.add_quad(&Quad {
                pos_start: [1.0, 0.0, 0.0].into(),
                pos_ext_1: [0.0, 1.0, 0.0].into(),
                pos_ext_2: [0.0, 0.0, 1.0].into(),
                tex_start: 0.0.into(),
                tex_extent: 1.0.into(),
                vert_colors: [[RIGHT_SHADE, RIGHT_SHADE, RIGHT_SHADE, 1.0].into(); 4],
                tex_index: right_tex_index,
            });
        }
    }
}
