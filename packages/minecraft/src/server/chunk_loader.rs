//! Utility for reading chunks from the save file or generating new chunks as appropriate.

use crate::{
    game_data::*,
    server::{
        save_content::*,
        save_db::SaveDb,
        generate_chunk::generate_chunk,
    },
};
use std::sync::Arc;
use anyhow::*;


/// Utility for reading chunks from the save file or generating new chunks as appropriate.
///
/// Clone-shareable.
#[derive(Clone)]
pub struct ChunkLoader {
    game: Arc<GameData>,
    save: SaveDb,
}

/// Chunk that is ready to be loaded into the world.
#[derive(Debug)]
pub struct ReadyChunk {
    /// Chunk coordinate.
    pub cc: Vec3<i64>,
    /// Chunk content, as in the save file.
    pub chunk_val: ChunkVal,
    /// Whether the chunk is already saved to the save file.
    pub saved: bool,
}


impl ChunkLoader {
    /// Construct a chunk loader.
    pub fn new(game: Arc<GameData>, save: SaveDb) -> Self {
        ChunkLoader { game, save }
    }

    /// Prepare a chunk to be loaded into the server. This is a blocking operation.
    ///
    /// Errors on save file IO error.
    pub fn load(&mut self, cc: Vec3<i64>) -> Result<ReadyChunk> {
        if let Some(chunk_val) = self.save.read(ChunkKey { cc })? {
            // attempt to read from save file
            Ok(ReadyChunk { cc, chunk_val, saved: true })
        } else {
            // fall back to generating
            let chunk_val = generate_chunk(&self.game);
            Ok(ReadyChunk { cc, chunk_val, saved: false })
        }
    }
}
