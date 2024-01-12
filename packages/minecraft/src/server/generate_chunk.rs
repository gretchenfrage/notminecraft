//! Generating chunks of the world for the first time.

use crate::{
    server::save_content::ChunkSaveVal,
    game_data::*,
};
use chunk_data::*;
use std::sync::Arc;
use vek::*;
use bracket_noise::prelude::FastNoise;


/// Generate a chunk of the world for the first time.
pub fn generate_chunk(game: &Arc<GameData>, cc: Vec3<i64>) -> ChunkSaveVal {
    let mut chunk_tile_blocks = ChunkBlocks::new(&game.blocks);
    let mut noise = FastNoise::new();
    noise.set_frequency(1.0 / 75.0);
    for x in 0..CHUNK_EXTENT.x {
        for z in 0..CHUNK_EXTENT.z {
            let height =
                noise.get_noise(
                    (x + cc.x * CHUNK_EXTENT.x) as f32,
                    (z + cc.z * CHUNK_EXTENT.z) as f32
                )
                / 2.0
                * 20.0
                + 40.0
                - (cc.y * CHUNK_EXTENT.y) as f32;
            let height = height.floor() as i64;

            for y in 0..i64::min(height, CHUNK_EXTENT.y) {
                let ltc = Vec3 { x, y, z };
                let lti = ltc_to_lti(ltc);

                chunk_tile_blocks.set(lti, game.content.stone.bid_stone, ());
            }
        }
    }
    ChunkSaveVal { chunk_tile_blocks }
}
