//! The client.

pub mod channel;
pub mod network;
pub mod per_player;
pub mod client_loaded_chunks;
pub mod mesh_tile;
pub mod mesh_item;
pub mod chunk_mesh_mgr;
pub mod join_server;
pub mod process_msg;
pub mod gui_state;
pub mod item_grid;
pub mod menu_mgr;
pub mod menu_esc;
pub mod menu_inventory;

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
    menu_mgr::MenuMgr,
};
use crate::{
    server::runner::ServerThread,
    thread_pool::ThreadPool,
    game_data::{
        per_item::PerItem,
        *,
    },
    sync_state_inventory_slots,
    sync_state_entities::{self, LoadedEntities},
};
use chunk_data::*;
use graphics::prelude::*;
use std::{
    sync::Arc,
    collections::HashMap,
};
use uuid::Uuid;
use slab::Slab;
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

/// Client state that already exists in the pre-join state.
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

    pub item_mesh: PerItem<Mesh>,

    /// Client-side space of chunks.
    pub chunks: ClientLoadedChunks,
    pub tile_blocks: PerChunk<ChunkBlocks>,
    pub chunk_mesh_mgr: ChunkMeshMgr,

    /// Client-side space of players.
    pub players: PlayerKeySpace,
    pub player_username: PerPlayer<String>,
    pub player_pos: PerPlayer<Vec3<f32>>,
    pub player_yaw: PerPlayer<f32>,
    pub player_pitch: PerPlayer<f32>,

    /// Client-side space of entities.
    pub entities: LoadedEntities,
    pub chunk_steves: PerChunk<Vec<sync_state_entities::ChunkEntityEntry<sync_state_entities::SteveEntityState, sync_state_entities::SteveEntityClientState>>>,
    pub chunk_pigs: PerChunk<Vec<sync_state_entities::ChunkEntityEntry<sync_state_entities::PigEntityState, sync_state_entities::PigEntityClientState>>>,
}

/// Client state once the client has joined the game.
pub struct Client {
    pub pre_join: PreJoinClient,
    pub self_pk: PlayerKey,
    pub pos: Vec3<f32>,
    pub yaw: f32,
    pub pitch: f32,
    pub menu_mgr: MenuMgr,
    pub inventory_slots: sync_state_inventory_slots::PlayerInventorySlots,
    pub steve_mesh: Mesh,
}
