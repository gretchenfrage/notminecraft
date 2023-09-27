
use crate::game_data::{
    GameData,
    mesh_logic::BlockMeshLogic,
};
use chunk_data::*;
use mesh_data::{
    MeshData,
    Quad,
    QUAD_INDICES,
    FLIPPED_QUAD_INDICES,
};
use vek::*;


/// Fills `mesh_buf` with the mesh data for `tile`, with the mesh being
/// relative to the tile's position.
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
                    if mesh_logic.rgb_u8_meta {
                        tile.get(tile_blocks).raw_meta::<Rgb<u8>>().map(|n| n as f32 / 0xff as f32).into()
                    } else {
                        Rgba::white()
                    },
                    gtc,
                    getter,
                    tile_blocks,
                    game,
                );
            }
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
    // short circuit if obscured
    let gtc2 = gtc + face.to_vec();
    let obscured = getter
        .gtc_get(gtc2)
        .map(|tile2| {
            let bid2 = tile2.get(tile_blocks).get();
            game.blocks_mesh_logic.get(bid2).obscures(-face)
        })
        .unwrap_or(true);
    if obscured {
        return;
    }

    // get quad start and extents
    let (pos_start, pos_exts) = face.quad_start_extents();

    // calculate vertex lighting
    let mut vert_rgbs = [color.rgb(); 4];

    // calculate ambient occlusion
    for (corner, ext_coefs) in [
        [Pole::Neg, Pole::Neg],
        [Pole::Pos, Pole::Neg],
        [Pole::Pos, Pole::Pos],
        [Pole::Neg, Pole::Pos],
    ].into_iter().enumerate() {
        let sides_occlude: [bool; 2] = [0, 1].map(|i| {
            let gtc3 = gtc2 + pos_exts[i].to_vec() * ext_coefs[i].to_int();
            getter
                .gtc_get(gtc3)
                .map(|tile3| {
                    let bid3 = tile3.get(tile_blocks).get();
                    game.blocks_mesh_logic.get(bid3).obscures(-ext_coefs[i] * pos_exts[i])
                })
                .unwrap_or(false)
        });
        let corner_occlude: f32 = {
            let mut gtc3 = gtc2;
            for i in 0..2 {
                gtc3 += pos_exts[i].to_vec() * ext_coefs[i].to_int();
            }
            getter
                .gtc_get(gtc3)
                .map(|tile3| {
                    let bid3 = tile3.get(tile_blocks).get();
                    let mesh_logic = game.blocks_mesh_logic.get(bid3);
                    let mut corner_obscure = 0.0;
                    for i in 0..2 {
                        if mesh_logic.obscures(-ext_coefs[i] * pos_exts[i]) {
                            corner_obscure += 0.5;
                        }
                    }
                    corner_obscure
                })
                .unwrap_or(0.0)
        };
        let occlude_lighting =
            1.0 - (0.25 / 3.0) * if sides_occlude[0] && sides_occlude[1] {
                3.0
            } else {
                corner_occlude
                + sides_occlude[0] as i32 as f32
                + sides_occlude[1] as i32 as f32
            };
        vert_rgbs[corner] *= occlude_lighting;
    }

    // calculate axis lighting
    let axis_lighting = 1.0 - match face {
        Face::PosY => 0,
        Face::PosX | Face::NegX => 1,
        Face::PosZ | Face::NegZ => 2,
        Face::NegY => 3,
    } as f32 * 0.07;
    for vert_rgb in &mut vert_rgbs {
        *vert_rgb *= axis_lighting;
    }
    
    // mesh the quad
    let pos_start = pos_start.to_poles().map(|pole| match pole {
        Pole::Neg => 0.0,
        Pole::Pos => 1.0,
    });
    let [
        pos_ext_1,
        pos_ext_2,
    ] = pos_exts.map(|pos_ext| pos_ext.to_vec().map(|n| n as f32));
    let vert_colors = vert_rgbs.map(|rgb| Rgba::from((rgb, color.a)));
    let quad = Quad {
        pos_start,
        pos_ext_1: pos_ext_1.into(),
        pos_ext_2: pos_ext_2.into(),
        tex_start: 0.0.into(),
        tex_extent: 1.0.into(),
        vert_colors,
        tex_index,
    };
    let flip = vert_rgbs[0].sum() + vert_rgbs[2].sum()
        < vert_rgbs[1].sum() + vert_rgbs[3].sum();
    let indices =
        if flip { FLIPPED_QUAD_INDICES }
        else { QUAD_INDICES };
    mesh_buf.extend(quad.to_vertices(), indices);
}
