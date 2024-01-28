//! Meshing a tile.

use crate::game_data::{
    logic::block_mesh_logic::*,
    *,
};
use mesh_data::*;
use chunk_data::*;
use std::sync::Arc;
use vek::*;


const OCCLUSION: f32 = 1.0;


/// Mesh a single tile in isolation, relative to its gtc.
pub fn mesh_tile(
    mesh_buf: &mut MeshData,
    tile: TileKey,
    game: &Arc<GameData>,
    getter: &Getter,
    tile_blocks: &PerChunk<ChunkBlocks>,
) {
    let bid1 = tile.get(tile_blocks).get();
    match &game.blocks_mesh_logic[bid1] {
        &BlockMeshLogic::NoMesh => (),
        &BlockMeshLogic::FullCube(BlockMeshLogicFullCube { tex_indices, .. }) => {
            // mesh each face
            let gtc1 = tile.gtc();
            for face in FACES {
                // skip if obscured
                let gtc2 = gtc1 + face.to_vec();
                if getter
                    .gtc_get(gtc2)
                    .map(|tile2| {
                        let bid2 = tile2.get(tile_blocks).get();
                        game.blocks_mesh_logic[bid2].obscures(-face)
                    })
                    .unwrap_or(true)
                {
                    continue;
                }

                // begin meshing
                let (pos_start, pos_exts) = face.quad_start_extents();
                let mut vert_rgbs = [Rgb::white(); 4];

                // ambient occlusion
                for (corner, ext_coefs) in [
                    (0, [Pole::Neg, Pole::Neg]),
                    (1, [Pole::Pos, Pole::Neg]),
                    (2, [Pole::Pos, Pole::Pos]),
                    (3, [Pole::Neg, Pole::Pos]),
                ] {
                    // calculate occlusion level from 0 through 3
                    let sides = [0, 1].map(|i| ext_coefs[i] * pos_exts[i]);
                    let [a, b] = sides.map(|side| getter
                        .gtc_get(gtc2 + side.to_vec())
                        .map(|tile3| {
                            let bid3 = tile3.get(tile_blocks).get();
                            game.blocks_mesh_logic[bid3].obscures(-side) as i32
                        })
                        .unwrap_or(0));
                    let c = getter
                        .gtc_get(gtc2 + sides[0].to_vec() + sides[1].to_vec())
                        .map(|tile3| {
                            let bid3 = tile3.get(tile_blocks).get();
                            sides.into_iter()
                                .all(|side| game.blocks_mesh_logic[bid3].obscures(-side)) as i32
                        })
                        .unwrap_or(0);
                    let ab = a * b;
                    let occlusion = 3 * ab + (a + b + c) * (1 - ab);

                    // light accordingly
                    vert_rgbs[corner] *= 1.0 - occlusion as f32 / 3.0 * OCCLUSION;
                }

                // axis lighting
                let axis_lighting = match face {
                    Face::PosY => 0,
                    Face::PosX | Face::NegX => 1,
                    Face::PosZ | Face::NegZ => 2,
                    Face::NegY => 3,
                };
                for vert_rgb in &mut vert_rgbs {
                    *vert_rgb *= 1.0 - axis_lighting as f32 * 0.07;
                }

                // add quad to mesh
                let pos_start = pos_start.to_poles().map(|pole| match pole {
                    Pole::Neg => 0.0,
                    Pole::Pos => 1.0,
                });
                let [
                    pos_ext_1,
                    pos_ext_2,
                ] = pos_exts.map(|pos_ext| pos_ext.to_vec().map(|n| n as f32));
                let vert_colors = vert_rgbs.map(|rgb| Rgba::from((rgb, 1.0)));
                let quad = Quad {
                    pos_start,
                    pos_ext_1: pos_ext_1.into(),
                    pos_ext_2: pos_ext_2.into(),
                    tex_start: 0.0.into(),
                    tex_extent: 1.0.into(),
                    vert_colors,
                    tex_index: tex_indices[face],
                };
                let flip = vert_rgbs[0].sum() + vert_rgbs[2].sum()
                    < vert_rgbs[1].sum() + vert_rgbs[3].sum();
                let indices =
                    if flip { FLIPPED_QUAD_INDICES }
                    else { QUAD_INDICES };
                mesh_buf.extend(quad.to_vertices(), indices);
            }
        }
    }
}
