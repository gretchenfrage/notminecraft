
use crate::{
    asset::loader::{
        AssetLoader,
        Properties,
    },
    util::hex_color::hex_color,
};
use mesh_data::{
    MeshData,
    Quad,
};
use graphics::frame_content::Mesh;
use std::borrow::Borrow;


pub fn block_item_mesh(tex_index: usize) -> MeshData {
    let mut mesh_buf = MeshData::new();
    let shade_a = 1.0;
    let shade_b = 0x48 as f32 / 0x8f as f32;
    let shade_c = 0x39 as f32 / 0x8f as f32;
    mesh_buf
        .add_quad(&Quad {
            pos_start: [1.0, 1.0, 0.0].into(),
            pos_ext_1: [-1.0, 0.0, 0.0].into(),
            pos_ext_2: [0.0, 0.0, 1.0].into(),
            tex_start: 0.0.into(),
            tex_extent: 1.0.into(),
            vert_colors: [[shade_a, shade_a, shade_a, 1.0].into(); 4],
            tex_index,
        });
    mesh_buf
        .add_quad(&Quad {
            pos_start: [0.0, 0.0, 0.0].into(),
            pos_ext_1: [0.0, 1.0, 0.0].into(),
            pos_ext_2: [1.0, 0.0, 0.0].into(),
            tex_start: 0.0.into(),
            tex_extent: 1.0.into(),
            vert_colors: [[shade_b, shade_b, shade_b, 1.0].into(); 4],
            tex_index,
        });
    mesh_buf
        .add_quad(&Quad {
            pos_start: [1.0, 0.0, 0.0].into(),
            pos_ext_1: [0.0, 1.0, 0.0].into(),
            pos_ext_2: [0.0, 0.0, 1.0].into(),
            tex_start: 0.0.into(),
            tex_extent: 1.0.into(),
            vert_colors: [[shade_c, shade_c, shade_c, 1.0].into(); 4],
            tex_index,
        });
    mesh_buf
}


#[derive(Debug)]
pub struct ItemMesh {
    pub mesh: ItemMeshMesh,
    pub name: String,
}

#[derive(Debug)]
pub enum ItemMeshMesh {
    Block(Mesh),
    Item(usize),
}

impl ItemMesh {
    pub fn load_basic_block<S: Borrow<str>>(
        loader: &AssetLoader,
        tex_index: usize,
        lang: &Properties,
        lang_key: S,
    ) -> Self {
        ItemMesh {
            mesh: ItemMeshMesh::Block(
                loader.load_mesh_data(&block_item_mesh(tex_index))
            ),
            name: lang[lang_key].to_owned(),
        }
    }

    pub fn load_basic_item<S: Borrow<str>>(
        tex_index: usize,
        lang: &Properties,
        lang_key: S,
    ) -> Self {
        ItemMesh {
            mesh: ItemMeshMesh::Item(tex_index),
            name: lang[lang_key].to_owned(),
        }
    }

    pub fn load_grass_block(
        loader: &AssetLoader,
        lang: &Properties,
    ) -> Self {
        use crate::asset::*;
        let mut mesh_buf = MeshData::new();
        let shade_a = 1.0;
        let shade_b = 0x48 as f32 / 0x8f as f32;
        let shade_c = 0x39 as f32 / 0x8f as f32;
        let grass_color = hex_color(0x74b44aff) / hex_color(0x969696ff);
        mesh_buf
            .add_quad(&Quad {
                pos_start: [1.0, 1.0, 0.0].into(),
                pos_ext_1: [-1.0, 0.0, 0.0].into(),
                pos_ext_2: [0.0, 0.0, 1.0].into(),
                tex_start: 0.0.into(),
                tex_extent: 1.0.into(),
                vert_colors: [grass_color * Rgba::new(shade_a, shade_a, shade_a, 1.0); 4],
                tex_index: BTI_GRASS_TOP,
            });
        mesh_buf
            .add_quad(&Quad {
                pos_start: [0.0, 0.0, 0.0].into(),
                pos_ext_1: [0.0, 1.0, 0.0].into(),
                pos_ext_2: [1.0, 0.0, 0.0].into(),
                tex_start: 0.0.into(),
                tex_extent: 1.0.into(),
                vert_colors: [[shade_b, shade_b, shade_b, 1.0].into(); 4],
                tex_index: BTI_GRASS_SIDE,
            });
        mesh_buf
            .add_quad(&Quad {
                pos_start: [1.0, 0.0, 0.0].into(),
                pos_ext_1: [0.0, 1.0, 0.0].into(),
                pos_ext_2: [0.0, 0.0, 1.0].into(),
                tex_start: 0.0.into(),
                tex_extent: 1.0.into(),
                vert_colors: [[shade_c, shade_c, shade_c, 1.0].into(); 4],
                tex_index: BTI_GRASS_SIDE,
            });

        ItemMesh {
            mesh: ItemMeshMesh::Block(
                loader.load_mesh_data(&mesh_buf)
            ),
            name: lang["tile.grass.name"].to_owned(),
        }
    }
}
