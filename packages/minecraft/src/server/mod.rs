//! The server.


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
    pub open_game_menu: PerClientConn<Option<OpenGameMenu>>,
    pub char_states: PerClientConn<CharState>,
}

/// State for which `&mut` references get wrapped in auto-syncing wrappers before game logic gets
/// access to them. Generally this means state which is replicated between the server and client,
/// and/or save file.
pub struct ServerSyncState {
    pub tile_blocks: sync_state_tile_blocks::ServerStorage,
    pub inventory_slots: sync_state_inventory_slots::ServerStorage,
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
    /// See type docs.
    pub server_only: &'a mut ServerOnlyState,
    /// See type docs.
    pub sync_ctx: &'a ServerSyncCtx,

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
