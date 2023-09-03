
use crate::game_data::{
    GameData,
    mesh_logic::BlockMeshLogic,
};
use chunk_data::*;
use mesh_data::{
    MeshData,
    Quad,
};
use vek::*;


pub fn mesh_tile(
    mesh_buf: &mut MeshData,
    tile: TileKey,
    getter: &Getter,
    tile_blocks: &PerChunk<ChunkBlocks>,
    game: &GameData,
) {
    debug_assert!(mesh_buf.is_empty());

    let gtc = cc_ltc_to_gtc(tile.cc, lti_to_ltc(tile.lti));
    let bid = tile.get(tile_blocks).get();
    let mesh_logic = game.blocks_mesh_logic.get(bid);

    match mesh_logic {
        &BlockMeshLogic::NoMesh => (),
        &BlockMeshLogic::FullCube(mesh_logic) => {
            for face in FACES {
                mesh_simple_face(
                    mesh_buf,
                    face,
                    mesh_logic.tex_indices[face],
                    Rgba::white(),
                    gtc,
                    getter,
                    tile_blocks,
                    game,
                );
            }
        }
    }
}

pub fn mesh_simple_face(
    mesh_buf: &mut MeshData,
    face: Face,
    tex_index: usize,
    color: Rgba<f32>,
    gtc: Vec3<i64>,
    getter: &Getter,
    tile_blocks: &PerChunk<ChunkBlocks>,
    game: &GameData,
) {
    let gtc2 = gtc + face.to_vec();
    let obscured = getter
        .gtc_get(gtc2)
        .map(|tile2| {
            let bid2 = tile2.get(tile_blocks).get();
            game.blocks_mesh_logic.get(bid2).obscures(-face)
        })
        .unwrap_or(true);
    if !obscured {
        let (
            pos_start,
            pos_ext_1,
            pos_ext_2,
        ) = match face {
            Face::PosX => ([1, 0, 0], [0, 1,  0], [ 0, 0,  1]),
            Face::NegX => ([0, 0, 1], [0, 1,  0], [ 0, 0, -1]),
            Face::PosY => ([0, 1, 0], [0, 0,  1], [ 1, 0,  0]),
            Face::NegY => ([0, 0, 1], [0, 0, -1], [ 1, 0,  0]),
            Face::PosZ => ([1, 0, 1], [0, 1,  0], [-1, 0,  0]),
            Face::NegZ => ([0, 0, 0], [0, 1,  0], [ 1, 0,  0]),
        };

        let pos_start = Vec3::from(pos_start)
            .map(|n: i32| n as f32);
        let pos_ext_1 = Extent3::from(pos_ext_1)
            .map(|n: i32| n as f32);
        let pos_ext_2 = Extent3::from(pos_ext_2)
            .map(|n: i32| n as f32);

        let face_darken = match face {
            Face::PosY => 0,
            Face::PosX | Face::NegX => 1,
            Face::PosZ | Face::NegZ => 2,
            Face::NegY => 3,
        };

        let mut color = color;
        for i in 0..3 {
            color[i] *= 1.0 - 0.07 * face_darken as f32;
        }
        
        mesh_buf
            .add_quad(&Quad {
                pos_start,
                pos_ext_1,
                pos_ext_2,
                tex_start: 0.0.into(),
                tex_extent: 1.0.into(),
                vert_colors: [color; 4],
                tex_index,
            });
    }
}
