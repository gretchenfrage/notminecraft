//! The client.

pub mod connection;
pub mod per_player;
pub mod client_loaded_chunks;
pub mod chunk_mesh;

use self::{
    connection::Connection,
    client_loaded_chunks::ClientLoadedChunks,
    per_player::*,
    chunk_mesh::ChunkMeshState,
};
use crate::{
    server::runner::ServerThread,
};
use chunk_data::*;
use vek::*;


pub struct PreJoinClient {
    /// Connection to the server.
    pub connection: Connection,
    /// Internal server, if this client is the host.
    pub server: Option<ServerThread>,

    /// Client-side space of chunks.
    pub chunks: ClientLoadedChunks,
    pub chunk_mesh_state: PerChunk<ChunkMeshState>,
    pub tile_blocks: PerChunk<ChunkBlocks>,

    /// Client-side space of players.
    pub players: PlayerKeySpace,
    pub player_username: PerPlayer<String>,
    pub player_pos: PerPlayer<Vec3<f32>>,
    pub player_pitch: PerPlayer<f32>,
    pub player_yaw: PerPlayer<f32>,
}
