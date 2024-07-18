//! Generating chunks of the world for the first time.

use crate::{
    server::save_content::*,
    game_data::*,
    sync_state_entities::*,
};
use chunk_data::*;
use std::sync::Arc;
use bracket_noise::prelude::FastNoise;
use rand_chacha::ChaCha8Rng;
use uuid::Uuid;
use vek::*;
use rand::prelude::*;


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

    let mut hasher = hmac_sha256::Hash::new();
    hasher.update(&cc.x.to_le_bytes());
    hasher.update(&cc.y.to_le_bytes());
    hasher.update(&cc.z.to_le_bytes());
    let hash = hasher.finalize();
    let mut rng = ChaCha8Rng::from_seed(hash);
    
    let mut steves = Vec::new();

    if rng.gen::<u32>() % 8 == 0 {
        steves.push(EntityData {
            uuid: Uuid::new_v4(),
            rel_pos: Vec3::new(rng.gen(), rng.gen(), rng.gen()) * CHUNK_EXTENT.map(|n| n as f32),
            state: SteveEntityState {
                name: format!("steve #{}", 0),
                vel: Default::default(),
            },
        });
    }

    let mut pigs = Vec::new();

    if rng.gen::<u32>() % 8 == 0 {
        pigs.push(EntityData {
            uuid: Uuid::new_v4(),
            rel_pos: Vec3::new(rng.gen(), rng.gen(), rng.gen()) * CHUNK_EXTENT.map(|n| n as f32),
            state: PigEntityState {
                color: Rgb::new(rng.gen(), rng.gen(), rng.gen()),
                vel: Default::default(),
            },
        });
    }

    ChunkSaveVal {
        chunk_tile_blocks,
        steves,
        pigs,
    }
}
