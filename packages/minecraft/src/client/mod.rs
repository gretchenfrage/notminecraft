//! The client.

pub mod channel;
pub mod network;
pub mod per_player;
pub mod client_loaded_chunks;
pub mod mesh_tile;
pub mod chunk_mesh_mgr;
pub mod pre_join;

use self::{
    channel::*,
    network::{
        Connection,
        NetworkEvent,
    },
    per_player::*,
    client_loaded_chunks::ClientLoadedChunks,
    chunk_mesh_mgr::{
        ChunkMeshMgr,
        ChunkMesh,
    },
};
use crate::{
    server::runner::ServerThread,
    thread_pool::ThreadPool,
    block_update_queue::BlockUpdateQueue,
    game_data::*,
};
use chunk_data::*;
use graphics::AsyncGpuVecContext;
use std::sync::Arc;
use vek::*;


/// Asynchronous event sent to the client from some other thread. See the `channel` module.
#[derive(Debug)]
pub enum ClientEvent {
    /// Only processed when the client state is still initializing in a background thread. Aborts
    /// that background thread.
    AbortInit,
    /// See inner type docs.
    Network(NetworkEvent),
    /// A job triggered by the chunk mesher to mesh a new chunk in the background is done and the
    /// prepared mesh should be installed into the client.
    ChunkMeshed {
        cc: Vec3<i64>,
        ci: usize,
        chunk_mesh: ChunkMesh,
    }
}

pub struct PreJoinClient {
    /// Helps define game logic. See type docs.
    pub game: Arc<GameData>,
    /// A sender handle to the client asynchronous event channel.
    pub client_send: ClientSender,
    /// A receiver handle to the client asynchronous event channel.
    pub client_recv: ClientReceiver,
    /// Connection to the server.
    pub connection: Connection,
    /// Internal server, if this client is the host.
    pub server: Option<ServerThread>,
    /// Handle to the thread pool.
    pub thread_pool: ThreadPool,
    /// Handle for uploading data to the GPU asynchronously.
    pub gpu_vec_ctx: AsyncGpuVecContext,
    // /// Services requests to asynchronously load new chunks in background.
    // pub chunk_mesher: ChunkMesher,

    /// Client-side space of chunks.
    pub chunks: ClientLoadedChunks,
    pub tile_blocks: PerChunk<ChunkBlocks>,
    // pub chunk_mesh_state: PerChunk<ChunkMeshState>,
    // /// Only nonempty within an update.
    // pub mesh_block_update_queue: BlockUpdateQueue,

    /// Client-side space of players.
    pub players: PlayerKeySpace,
    pub player_username: PerPlayer<String>,
    pub player_pos: PerPlayer<Vec3<f32>>,
    pub player_yaw: PerPlayer<f32>,
    pub player_pitch: PerPlayer<f32>,
}
