//! The server.
//!
//! This top-level server module mostly contains:
//!
//! - The server state top-level struct.
//! - The "sync world" top-level struct.
//! - The server event top-level enum.
//! - Submodules with utilities that are used in the server and not the client.
//! - Submodules with managers to abstract subclusters of server logic.
//! - Other submodules to encapsulate subclusters of server-only logic.

pub mod per_player;
pub mod channel;
pub mod generate_chunk;
pub mod save_content;
pub mod chunk_loader;
pub mod save_db;
pub mod tick_mgr;
pub mod chunk_mgr;
pub mod save_mgr;
pub mod conn_mgr;

use self::{
    channel::*,
    per_player::*,
    save_content::*,
    tick_mgr::TickMgr,
    chunk_mgr::ChunkMgr,
    save_mgr::SaveMgr,
};
use crate::thread_pool::ThreadPool;


/// Event sent to the core server loop from some other thread. See the `channel` module.
pub enum ServerEvent {
    /// Shut down the server.
    Stop,
    /// See inner type docs.
    Network(NetworkEvent),
    /// A job triggered by the conn mgr to load a player's save state from the save file is done
    /// and should be routed back to the conn mgr.
    PlayerSaveStateReady {
        /// The player.
        pk: PlayerKey,
        /// The loaded save file value.
        save_val: Option<PlayerSaveVal>,
    },
    /// A job triggered by the chunk mgr to load a chunk from the save file or generate it for the
    /// first time and should be routed back to the chunk mgr. 
    ChunkReady {
        save_key: ChunkKey,
        save_val: ChunkVal,
        saved: bool,
    },
    /// A job triggered by the save mgr to save the world to the save file is done and should be
    /// routed back to the save mgr.
    SaveOpDone,
}

/// Raw server state.
///
/// Some "system" operations access this directly, such as operations dealing with the lifecycle of
/// chunks and clients, but game logic generally accesses this through `SyncWorld` instead.
pub struct Server {
    /// See type docs.
    pub server_only: ServerOnlyState,
    /// See type docs.
    pub sync_ctx: ServerSyncCtx,
    /// See type docs.
    pub sync_state: ServerSyncState,
}

/// State which game logic gets `&mut` access to. Often this means state which is only represented
/// in server memory and thus game logic can mutate it without worrying about synchronization.
pub struct ServerOnlyState {
    /// A sender handle to the server event channel.
    pub server_send: ServerSend,
    /// A receiver handle to the server event channel.
    pub server_recv: ServerRecv,
    /// Handle to the thread pool
    pub thread_pool: ThreadPool,

    //pub open_game_menu: PerPlayer<Option<OpenGameMenu>>,
    //pub char_states: PerPlayer<CharState>,
}

/// State for which `&mut` references get wrapped in auto-syncing wrappers before game logic gets
/// access to them. Generally this means state which is replicated between the server and client,
/// and/or save file.
pub struct ServerSyncState {
    pub tile_blocks: sync_state_tile_blocks::ServerState,
    pub inventory_slots: sync_state_inventory_slots::ServerState,
}

/// State which game logic gets only shared references to. Often this is because the state is
/// contextual to the replication of edits between the server's memory and other representations
/// of that state.
pub struct ServerSyncCtx {
    /// Helps define game logic. See type docs.
    pub game: Arc<GameData>,
    /// Manages ticks and the passage of time. See type docs.
    pub tick_mgr: TickMgr,
    /// Manages chunks and their loading and unloading. See type docs.
    pub chunk_mgr: ChunkMgr,
    /// Manages the save file. See type docs.
    pub save_mgr: SaveMgr,
    /// Manages clients and their joining and leaving. See type docs.
    pub conn_mgr: ConnMgr,
}

/// Projection of `&mut Server` that game logic gets access to. Designed to automatically keep
/// clients and save file synchronized with the server when mutating synchronized state.
pub struct SyncWorld<'a> {
    /// State game logic gets mut reference to. See type docs.
    pub server_only: &'a mut ServerOnlyState,
    /// State game logic gets shared reference to. See type docs.
    pub sync_ctx: &'a ServerSyncCtx,
    // ==== sync writers ====
    pub tile_blocks: sync_state_tile_blocks::SyncWrite<'a>,
    pub inventory_slots: sync_state_inventory_slots::SyncWrite<'a>,
}


impl Server {
    /// Project into `SyncWorld`.
    pub fn as_sync_world(&mut self) -> SyncWorld {
        SyncWorld::new_manual(&mut self.server_only, &self.sync_ctx, &mut self.sync_state)
    }
}

impl SyncWorld<'a> {
    /// Construct manually (with respect to synchronization logic).
    pub fn new_manual(
        server_only: &'a mut ServerOnlyState,
        sync_ctx: &'a ServerSyncCtx,
        sync_state: &'a mut ServerSyncState,
    ) -> Self {
        let &mut ServerSyncState {

        } = sync_state;
        SyncWorld {
            server_only,
            sync_ctx,


        }
    }
}
