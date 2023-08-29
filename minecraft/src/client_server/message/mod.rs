//! Messages sent between client and server.

use super::client::edit::Edit;
use crate::game_binschema::GameBinschema;
use chunk_data::*;
use vek::*;


/// Message sent from client to server.
#[derive(Debug, GameBinschema)]
pub enum UpMessage {
    SetTileBlock(UpMessageSetTileBlock),
}

#[derive(Debug, GameBinschema)]
pub struct UpMessageSetTileBlock {
    pub gtc: Vec3<i64>,
    pub bid: RawBlockId,
}

/// Message sent from server to client.
#[derive(Debug, GameBinschema)]
pub enum DownMessage {
    LoadChunk(DownMessageLoadChunk),
    ApplyEdit(DownMessageApplyEdit),
}

#[derive(Debug, GameBinschema)]
pub struct DownMessageLoadChunk {
    pub cc: Vec3<i64>,
    pub ci: usize,
    pub chunk_tile_blocks: ChunkBlocks,
}

#[derive(Debug, GameBinschema)]
pub struct DownMessageApplyEdit {
    pub ci: usize,
    pub edit: Edit,
}
