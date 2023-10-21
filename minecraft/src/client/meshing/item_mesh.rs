
use graphics::frame_content::Mesh;
use crate::{
    game_data::{
        per_item::PerItem,
        item_mesh_logic::ItemMeshLogic,
    },
    gui::prelude::*,
};
use mesh_data::*;
use vek::*;


#[derive(Debug)]
pub struct ItemMesh {
    pub mesh: Mesh,
}

pub fn items_mesh(ctx: &GuiGlobalContext) -> PerItem<ItemMesh> {
    let mut items_mesh = PerItem::new_no_default();
    for iid in ctx.game.items.iter() {
        items_mesh.set(iid, match &ctx.game.items_mesh_logic[iid] {
            &ItemMeshLogic::FullCube {
                top_tex_index,
                left_tex_index,
                right_tex_index,
            } => {
                const LEFT_SHADE: f32 = 0x48 as f32 / 0x8f as f32;
                const RIGHT_SHADE: f32 = 0x39 as f32 / 0x8f as f32;

                let mut mesh_buf = MeshData::new();
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
                ItemMesh {
                    mesh: mesh_buf.upload(&*ctx.renderer.borrow()),
                }
            }
        });
    }
    items_mesh
}
