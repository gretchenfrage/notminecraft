//! Messages sent between client and server.
//!
//! See `server::conn_mgr` module for more detailed explanation of connection protocol.

use crate::{
    game_binschema::GameBinschema,
    util_usize_lt::UsizeLt,
    util_time::ServerRelTime,
    item::*,
    sync_state_entities::*,
};
use chunk_data::*;
use uuid::Uuid;
use vek::*;


/// Message sent from client to server.
#[derive(Debug, GameBinschema)]
pub enum UpMsg {
    /// Part of connection initialization flow.
    ///
    /// Client sends this right after connecting, which triggers an `AcceptLogIn` message response
    /// if accepted.
    LogIn(UpMsgLogIn),
    /// Message that client may merely need to be logged in to send, rather than fully joined.
    PreJoin(PreJoinUpMsg),
    /// Part of connection initialization flow.
    ///
    /// Adds the client fully to the game world, and triggers a `FinalizeJoinGame` message
    /// response to be sent.
    JoinGame,
    /// "Game logic" message from a joined player to the server.
    PlayerMsg(PlayerMsg),
}

/// Part of connection initialization flow.
#[derive(Debug, GameBinschema)]
pub struct UpMsgLogIn {
    pub username: String,
}

/// Message that client may merely need to be logged in to send, rather than fully joined.
#[derive(Debug, GameBinschema)]
pub enum PreJoinUpMsg {
    /// Manages client-to-server backpressure for loading additional chunks.
    ///
    /// Should be sent after `AddChunk` is fully processed. "Fully processed" may include client-
    /// side asynchronous post-processing such as meshing the chunk. Can deduplicate if multiple by
    /// adding together. Protocol violation to send more than have received `AddChunk`.
    AcceptMoreChunks(u32),
}

/// "Game logic" message from a joined player to the server.
#[derive(Debug, GameBinschema)]
pub enum PlayerMsg {
    /// Set own position and direction.
    SetCharState(PlayerMsgSetCharState),
    /// Set block at tile.
    SetTileBlock(PlayerMsgSetTileBlock),
    /// Open a game menu in a way that's synced with the server.
    OpenSyncMenu(PlayerMsgOpenSyncMenu),
    /// Close the currently open sync menu.
    CloseSyncMenu(PlayerMsgCloseSyncMenu),
    /// Player message to be processed by the currently open sync menu.
    SyncMenuMsg(SyncMenuMsg),
    ClockDebug(ServerRelTime),
}

/// Set own position and direction.
#[derive(Debug, GameBinschema)]
pub struct PlayerMsgSetCharState {
    pub pos: Vec3<f32>,
    pub yaw: f32,
    pub pitch: f32,
}

/// Set block at tile.
#[derive(Debug, GameBinschema)]
pub struct PlayerMsgSetTileBlock {
    pub gtc: Vec3<i64>,
    pub bid_meta: ErasedBidMeta,
}

/// Open a game menu in a way that's synced with the server.
#[derive(Debug, GameBinschema, Copy, Clone, PartialEq)]
pub enum PlayerMsgOpenSyncMenu {
    Inventory,
}

/// Close the currently open sync menu.
#[derive(Debug, GameBinschema)]
pub struct PlayerMsgCloseSyncMenu;

/// Reference to an item slot transmitted from client to server.
///
/// This means it may be relative to the sync menu the client has open.
#[derive(Debug, GameBinschema, Copy, Clone)]
pub enum UpItemSlotRef {
    /// Item in the player's open inventory.
    Inventory(UsizeLt<36>),
    /// The held item.
    Held,
}

/// Player message to be processed by the currently open sync menu.
#[derive(Debug, GameBinschema)]
pub enum SyncMenuMsg {
    TransferItems(SyncMenuMsgTransferItems),
    SwapItemSlots(SyncMenuMsgSwapItemSlots),
}

/// Attempt to move the given number of items from one slot to another.
#[derive(Debug, GameBinschema)]
pub struct SyncMenuMsgTransferItems {
    /// Item slot transferring from.
    pub from: UpItemSlotRef,
    /// Item slot transferring to.
    pub to: UpItemSlotRef,
    /// Number of items to transfer.
    pub amount: u8,
}

/// Attempt to swap the contents of two item slots.
#[derive(Debug, GameBinschema)]
pub struct SyncMenuMsgSwapItemSlots(pub [UpItemSlotRef; 2]);

/// Message sent from server to client.
#[derive(Debug, GameBinschema)]
pub enum DownMsg {
    /// Part of connection initialization flow.
    ///
    /// This places the client into a state of preparing to join the world. It will begin receiving
    /// messages to load parts of the world into the client. Once enough of the world is loaded,
    /// the client will receive a `ShouldJoinGame` message.
    AcceptLogIn,
    /// Message that client can process once logged in but possibly still before joining game.
    PreJoin(PreJoinDownMsg),
    /// Part of connection initialization flow.
    ///
    /// When the client receives this, when ready, the client should send a `JoinGame` message.
    ShouldJoinGame,
    /// Part of connection initialization flow.
    ///
    /// When the client receives this, the server has fully added the player to the game world, and
    /// the client should begin displaying the world to the user and allowing the user to interact
    /// with the world in ways that trigger player msgs.
    FinalizeJoinGame(DownMsgFinalizeJoinGame),
    /// Message that is only valid to send to client once it has fully joined the game.
    PostJoin(PostJoinDownMsg),
}

/// Message that client can process once logged in but possibly still before joining game.
#[derive(Debug, GameBinschema)]
pub enum PreJoinDownMsg {
    /// Load a player into the client.
    AddPlayer(DownMsgAddPlayer),
    /// Remove a loaded player from the client.
    RemovePlayer(DownMsgRemovePlayer),
    /// Load a chunk into the client. When done, send back an `AcceptMoreChunks` message.
    AddChunk(DownMsgAddChunk),
    /// Remove a loaded chunk from the client.
    RemoveChunk(DownMsgRemoveChunk),
    SetTileBlock {
        chunk_idx: DownChunkIdx,
        lti: u16,
        bid_meta: ErasedBidMeta,
    },
    SetPlayerCharState {
        player_idx: DownPlayerIdx,
        pos: Vec3<f32>,
        yaw: f32,
        pitch: f32,
    },
    /// Add a new entity to a chunk.
    ///
    /// Should push the entity to the chunk's entity vector of the given entity's entity type.
    AddEntity {
        /// Chunk that shall own the entity.
        chunk_idx: DownChunkIdx,
        /// Entity to be added.
        entity: EntityData<AnyEntityState>,
    },
    /// Remove an existing entity.
    ///
    /// Should swap-remove the entity from the chunk's entity vector of the specified entity type.
    RemoveEntity {
        /// Chunk that owns the entity.
        chunk_idx: DownChunkIdx,
        /// Which entity vector the entity exists in.
        entity_type: EntityType,
        /// Index of the entity to remove within its vector.
        vector_idx: usize,
    },
    /// Move an existing entity to a different chunk.
    ///
    /// Should swap-remove the entity from the old chunk's entity vector of the specified entity
    /// type, then push it to the corresponding entity vector of the new chunk.
    ChangeEntityOwningChunk {
        /// Chunk that currently owns the entity.
        old_chunk_idx: DownChunkIdx,
        /// Which entity vector in the old and new chunk the entity exists in.
        entity_type: EntityType,
        /// Entity's current index in its old chunk's entity vector.
        vector_idx: usize,
        /// Chunk that the entity will be moved into.
        new_chunk_idx: DownChunkIdx,
    },
    /// Apply an edit to an existing entity.
    EditEntity {
        /// Chunk that owns the entity.
        chunk_idx: DownChunkIdx,
        /// Index of the entity within the chunk's entity vector of the entity type that the edit
        /// value applies to.
        vector_idx: usize,
        /// Edit to apply.
        edit: AnyEntityEdit,
    },
}

/// Message that is only valid to send to client once it has fully joined the game.
#[derive(Debug, GameBinschema)]
pub enum PostJoinDownMsg {
    /// Acknowledge having fully processed messages from client up to and including message number
    /// `last_processed`, wherein the first up msg the client sends has a message number of 1.
    Ack { last_processed: u64 },
    /// Invalidate the open sync menu opened with the given up msg index.
    ///
    /// The client unilaterally determines what sync menu it thinks it has open, and so may send
    /// other messages to be processed within the context of the sync menu it thinks it has open.
    /// However, the server has the ability to "invalidate" the client opening or having open some
    /// sync menu, both immediately upon the client opening it so as to reject the opening of it,
    /// and also later on if some conditions occur which makes forces it to close. Messages
    /// received from the client which are to be processed within the context of the currently open
    /// sync menu are generally just ignored if the currently open sync menu is invalidated.
    InvalidateSyncMenu { up_msg_idx: u64 },
    SetItemSlot {
        item_slot: DownItemSlotRef,
        slot_content: Option<ItemStack>,
    }
}

/// Part of connection initialization flow.
///
/// Contains any player-specific state to be loaded only onto that client.
#[derive(Debug, GameBinschema)]
pub struct DownMsgFinalizeJoinGame {
    /// The loaded player corresponding to the client itself.
    pub self_player_idx: DownPlayerIdx,
    pub inventory_slots: [Option<ItemStack>; 36],
    pub held_slot: Option<ItemStack>,
}

/// Load a player into the client.
///
/// Contains any player-specific state to be loaded onto all clients.
#[derive(Debug, GameBinschema)]
pub struct DownMsgAddPlayer {
    /// Follows a slab pattern.
    pub player_idx: DownPlayerIdx,
    pub username: String,
    pub pos: Vec3<f32>,
    pub pitch: f32,
    pub yaw: f32,
}

/// Remove a loaded player from the client.
#[derive(Debug, GameBinschema)]
pub struct DownMsgRemovePlayer {
    /// Follows a slab pattern.
    pub player_idx: DownPlayerIdx,
}

/// Load a chunk into the client.
///
/// Contains any chunk state to be loaded onto the client.
#[derive(Debug, GameBinschema)]
pub struct DownMsgAddChunk {
    /// Follows a slab pattern.
    pub chunk_idx: DownChunkIdx,
    pub cc: Vec3<i64>,
    pub chunk_tile_blocks: ChunkBlocks,
    //pub steves: Vec<DownEntity<SteveEntityState>>,
    //pub pigs: Vec<DownEntity<PigEntityState>>,
}
/*
/// Entity state for tranmission in a down msg.
#[derive(Debug, GameBinschema, Copy, Clone)]
pub struct DownEntity<T> {
    /// Entity's stable UUID.
    pub entity_uuid: Uuid,
    /// Entity's position relative to chunk that owns it.
    pub rel_pos: Vec3<f32>,
    /// Entity type-specific state.
    pub state: T,
}

// TODO: generally speaking organize this better

#[derive(Debug, GameBinschema)]
pub enum AnyDownEntity {
    Steve(DownEntity<SteveEntityState>),
    Pig(DownEntity<PigEntityState>),
}

/// Edit sent from server to client applicable to a single entity.
///
/// This value must convey the information of what type of entity this edit applies to.
#[derive(Debug, GameBinschema)]
pub enum EntityEdit {
    SetStevePosVel(EntityEditSetStevePosVel),
    SetSteveName(EntityEditSetSteveName),
    SetPigPosVel(EntityEditSetPigPosVel),
    SetPigColor(EntityEditSetPigColor),
}

#[derive(Debug, GameBinschema)]
pub struct EntityEditSetStevePosVel {
    pub rel_pos: Vec3<f32>,
    pub vel: Vec3<f32>,
}

#[derive(Debug, GameBinschema)]
pub struct EntityEditSetSteveName {
    pub name: String,
}

#[derive(Debug, GameBinschema)]
pub struct EntityEditSetPigPosVel {
    pub rel_pos: Vec3<f32>,
    pub vel: Vec3<f32>,
}

#[derive(Debug, GameBinschema)]
pub struct EntityEditSetPigColor {
    pub color: Rgb<f32>,
}
*/
/// Remove a loaded chunk from the client.
#[derive(Debug, GameBinschema)]
pub struct DownMsgRemoveChunk {
    /// Follows a slab pattern.
    pub chunk_idx: DownChunkIdx,
}

/// Type safety wrapper around clientside player index in down msgs.
#[derive(Debug, GameBinschema, Copy, Clone)]
pub struct DownPlayerIdx(pub usize);

/// Type safety wrapper around clientside chunk index in down msgs.
#[derive(Debug, GameBinschema, Copy, Clone)]
pub struct DownChunkIdx(pub usize);

/// Reference to an item slot transmitted from server to client.
#[derive(Debug, GameBinschema, Copy, Clone)]
pub enum DownItemSlotRef {
    /// The held item.
    Held,
    /// Item in the player's open inventory.
    Inventory(UsizeLt<36>),
}
