//! Messages sent between client and server.
//!
//! See `server::conn_mgr` module for more detailed explanation of connection protocol.

use crate::game_binschema::GameBinschema;
use chunk_data::*;
use vek::*;


/// Message sent from client to server.
#[derive(Debug, GameBinschema)]
pub enum UpMsg {
    /// Part of connection initialization flow.
    ///
    /// Client sends this right after connecting, which triggers an `AcceptLogIn` message response
    /// if accepted.
    LogIn(UpMsgLogIn),
    /// Part of connection initialization flow.
    ///
    /// Adds the client fully to the game world, and triggers a `FinalizeJoinGame` message
    /// response to be sent.
    JoinGame,
    /// Manages client-to-server backpressure for loading additional chunks.
    ///
    /// Should be sent after `AddChunk` is fully processed. May include client-side asynchronous
    /// post-processing such as meshing the chunk. Can deduplicate if multiple by adding together.
    /// Protocol violation to send more than have received `AddChunk`.
    AcceptMoreChunks(u32),
    /// "Game logic" message from a joined player to the server.
    PlayerMsg(PlayerMsg),
}

/// Part of connection initialization flow.
#[derive(Debug, GameBinschema)]
pub struct UpMsgLogIn {
    pub username: String,
}

/// "Game logic" message from a joined player to the server.
#[derive(Debug, GameBinschema)]
pub enum PlayerMsg {
    /// Set own position and direction.
    SetCharState(PlayerMsgSetCharState),
    /// Set block at tile.
    SetTileBlock(PlayerMsgSetTileBlock),
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

/// Message sent from server to client.
#[derive(Debug, GameBinschema)]
pub enum DownMsg {
    /// Part of connection initialization flow.
    ///
    /// This places the client into a state of preparing to join the world. It will begin receiving
    /// messages to load parts of the world into the client. Once enough of the world is loaded,
    /// the client will receive a `ShouldJoinGame` message.
    AcceptLogIn,
    /// Load a player into the client.
    AddPlayer(DownMsgAddPlayer),
    /// Remove a loaded player from the client.
    RemovePlayer(DownMsgRemovePlayer),
    /// Load a chunk into the client. When done, send back an `AcceptMoreChunks` message.
    AddChunk(DownMsgAddChunk),
    /// Remove a loaded chunk from the client.
    RemoveChunk(DownMsgRemoveChunk),
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
    /// Acknowledge having fully processed messages from client up to and including message number
    /// `last_processed`, wherein the first up msg the client sends has a message number of 1.
    Ack { last_processed: u64 },
    /// Apply an edit to a loaded part of the world.
    ApplyEdit(Edit),
}

/// Part of connection initialization flow.
///
/// Contains any player-specific state to be loaded only onto that client.
#[derive(Debug, GameBinschema)]
pub struct DownMsgFinalizeJoinGame {
    /// The loaded player corresponding to the client itself.
    pub self_player_idx: DownPlayerIdx,
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
}

/// Remove a loaded chunk from the client.
#[derive(Debug, GameBinschema)]
pub struct DownMsgRemoveChunk {
    /// Follows a slab pattern.
    pub chunk_idx: DownChunkIdx,
}

/// Edit sent from the server to the client regarding some loaded state.
#[derive(Debug, GameBinschema)]
pub enum Edit {
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
}

/// Type safety wrapper around clientside player index in down msgs.
#[derive(Debug, GameBinschema, Copy, Clone)]
pub struct DownPlayerIdx(pub usize);

/// Type safety wrapper around clientside chunk index in down msgs.
#[derive(Debug, GameBinschema, Copy, Clone)]
pub struct DownChunkIdx(pub usize);
