
use crate::game_data::{
    GameData,
    BlockMeshLogic,
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
    let mesh_logic = game.block_mesh_logics
        .get(bid)
        .unwrap_or(&BlockMeshLogic::Simple(0));

    match mesh_logic {
        &BlockMeshLogic::Invisible => (),
        &BlockMeshLogic::Simple(tex_index) => {
            for face in FACES {
                let gtc2 = gtc + face.to_vec();
                let obscured = getter
                    .gtc_get(gtc2)
                    .and_then(|tile2| {
                        let bid2 = tile2.get(tile_blocks).get();
                        game.block_obscures.get(bid2)
                    })
                    .map(|obscures| obscures[-face])
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
                            vert_colors: [Rgba::white(); 4],
                            tex_index,
                        });
                }
            }
        }
    }
}
