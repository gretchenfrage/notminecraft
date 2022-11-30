
use crate::{
    game_data::{
        BTI_DIRT,
        BTI_GRASS_SIDE,
        BTI_GRASS_TOP,
        BTI_DOOR_UPPER,
        BTI_DOOR_LOWER,
        GameData,
        BlockMeshLogic,
        DoorMeta,
        DoorPart,
        DoorDir,
    },
    util::hex_color::hex_color,
};
use chunk_data::{
    FACES,
    Face,
    TileKey,
    Getter,
    PerChunk,
    ChunkBlocks,
    cc_ltc_to_gtc,
    lti_to_ltc,
};
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
    let mesh_logic = game.block_mesh_logics.get(bid);

    match mesh_logic {
        &BlockMeshLogic::Invisible => (),
        &BlockMeshLogic::Simple(tex_index) => {
            for face in FACES {
                mesh_simple_face(
                    mesh_buf,
                    face,
                    tex_index,
                    Rgba::white(),
                    gtc,
                    getter,
                    tile_blocks,
                    game,
                );
            }
        }
        &BlockMeshLogic::SimpleFaces(tex_indices) => {
            for face in FACES {
                mesh_simple_face(
                    mesh_buf,
                    face,
                    tex_indices[face],
                    Rgba::white(),
                    gtc,
                    getter,
                    tile_blocks,
                    game,
                );
            }
        }
        &BlockMeshLogic::Grass => {
            for face in FACES {
                let grass_color = hex_color(0x74b44aff) / hex_color(0x969696ff);
                let (tex_index, color) = match face {
                    Face::PosY => (BTI_GRASS_TOP, grass_color),
                    Face::NegY => (BTI_DIRT, Rgba::white()),
                    _ => (BTI_GRASS_SIDE, Rgba::white()),
                };
                mesh_simple_face(
                    mesh_buf,
                    face,
                    tex_index,
                    color,
                    gtc,
                    getter,
                    tile_blocks,
                    game,
                );
            }
        }
        &BlockMeshLogic::Door => {
            let DoorMeta {
                part,
                dir,
            } = tile.get(tile_blocks).meta(game.bid_door);
            let tex_index = match part {
                DoorPart::Upper => BTI_DOOR_UPPER,
                DoorPart::Lower => BTI_DOOR_LOWER,
            };
            let (pos_start, pos_ext_1, pos_ext_2) = match dir {
                DoorDir::PosX => ([1, 0, 0], [0, 1,  0], [ 0, 0,  1]),
                DoorDir::NegX => ([0, 0, 1], [0, 1,  0], [ 0, 0, -1]),
                DoorDir::PosZ => ([1, 0, 1], [0, 1,  0], [-1, 0,  0]),
                DoorDir::NegZ => ([0, 0, 0], [0, 1,  0], [ 1, 0,  0]),
            };

            let pos_start = Vec3::from(pos_start)
                .map(|n: i32| n as f32);
            let pos_ext_1 = Extent3::from(pos_ext_1)
                .map(|n: i32| n as f32);
            let pos_ext_2 = Extent3::from(pos_ext_2)
                .map(|n: i32| n as f32);
            
            mesh_buf
                .add_quad(&Quad {
                    pos_start,
                    pos_ext_1,
                    pos_ext_2,
                    tex_start: 0.0.into(),
                    tex_extent: 1.0.into(),
                    vert_colors: [Rgba::white(); 4],
                    tex_index,
                });
        }
    }
}

fn mesh_simple_face(
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
            game.block_obscures.get(bid2)[-face]
        })
        .unwrap_or(false);
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
