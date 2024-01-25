//! The client.

pub mod channel;
pub mod network;
pub mod per_player;
pub mod client_loaded_chunks;
pub mod chunk_mesh;

use self::{
    network::{
        Connection,
        NetworkEvent,
    },
    client_loaded_chunks::ClientLoadedChunks,
    per_player::*,
    chunk_mesh::ChunkMeshState,
};
use crate::{
    server::runner::ServerThread,
};
use chunk_data::*;
use vek::*;

/// Asynchronous event sent to the client from some other thread. See the `channel` module.
#[derive(Debug)]
pub enum ClientEvent {
    /// Only processed when the client state is still initializing in a background thread. Aborts
    /// that background thread.
    AbortInit,
    /// See inner type docs.
    Network(NetworkEvent),
}

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
