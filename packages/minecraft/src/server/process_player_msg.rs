//! Process player messages.
//!
//! This is a "game logic" module moreso than a "system" module.

use crate::{
    server::{
        per_player::*,
        SyncWorld,
    },
    message::*,
};


/// Per-player optional state for tracking sync menu they have open.
#[derive(Debug)]
pub struct PlayerOpenSyncMenu {
    /// Whether the server has not invalidated this open menu.
    pub valid: bool,
    /// The open sync menu.
    pub menu: PlayerMsgOpenSyncMenu,
    /// The up msg index that opened the sync menu.
    pub up_msg_idx: u64,
}


/// Process a player message from a joined player.
pub fn process_player_msg(world: &mut SyncWorld, pk: JoinedPlayerKey, msg: PlayerMsg) {
    match msg {
        PlayerMsg::SetCharState(inner) => inner.process(world, pk),
        PlayerMsg::SetTileBlock(inner) => inner.process(world, pk),
        PlayerMsg::OpenSyncMenu(inner) => inner.process(world, pk),
        PlayerMsg::CloseSyncMenu(inner) => inner.process(world, pk),
        PlayerMsg::SyncMenuMsg(inner) => inner.process(world, pk),
    }
}

// internal trait for player msg variants
trait Process {
    fn process(self, world: &mut SyncWorld, pk: JoinedPlayerKey);
}

impl Process for PlayerMsgSetCharState {
    // set char state
    fn process(self, world: &mut SyncWorld, pk: JoinedPlayerKey) {
        // as a temporary situation we kind of handle this manually
        let PlayerMsgSetCharState { pos, yaw, pitch } = self;

        world.server_only.player_pos[pk] = pos;
        world.server_only.player_yaw[pk] = yaw;
        world.server_only.player_pitch[pk] = pitch;

        for pk2 in world.sync_ctx.conn_mgr.players().iter() {
            world.sync_ctx.conn_mgr.send(pk2, DownMsg::PreJoin(PreJoinDownMsg::ApplyEdit(
                Edit::SetPlayerCharState {
                    player_idx: DownPlayerIdx(
                        world.sync_ctx.conn_mgr.player_to_clientside(pk, pk2)
                    ),
                    pos,
                    yaw,
                    pitch,
                }
            )));
        }
    }
}

impl Process for PlayerMsgSetTileBlock {
    // set tile block
    fn process(self, world: &mut SyncWorld, _pk: JoinedPlayerKey) {
        let PlayerMsgSetTileBlock { gtc, bid_meta } = self;

        if let Some(tile) = world.getter.gtc_get(gtc) {
            tile.get(&mut world.tile_blocks).erased_set(bid_meta);
        }
    }
}

impl Process for PlayerMsgOpenSyncMenu {
    // open a sync menu
    fn process(self, world: &mut SyncWorld, pk: JoinedPlayerKey) {
        let valid = match &self {
            PlayerMsgOpenSyncMenu::Inventory => true,
        };
        let up_msg_idx = world.sync_ctx.conn_mgr.last_processed(pk);
        world.server_only.player_open_sync_menu[pk] =
            Some(PlayerOpenSyncMenu { valid, menu: self, up_msg_idx });
        if !valid {
            world.sync_ctx.conn_mgr.send(pk, DownMsg::InvalidateSyncMenu { up_msg_idx });
        }
    }
}

impl Process for PlayerMsgCloseSyncMenu {
    // close the open sync menu
    fn process(self, world: &mut SyncWorld, pk: JoinedPlayerKey) {
        let PlayerMsgCloseSyncMenu = self;
        world.server_only.player_open_sync_menu[pk] = None;
    }
}

impl Process for SyncMenuMsg {
    // branching for sync menu messages
    fn process(self, world: &mut SyncWorld, pk: JoinedPlayerKey) {
        match self {
            SyncMenuMsg::TransferItems(inner) => inner.process(world, pk),
            SyncMenuMsg::SwapItemSlots(inner) => inner.process(world, pk),
        }
    }
}

impl Process for SyncMenuMsgTransferItems {
    // transfer items from one item slot to another
    fn process(self, world: &mut SyncWorld, pk: JoinedPlayerKey) {
        let SyncMenuMsgTransferItems { from, to, amount } = self;
        // TODO
    }
}

impl Process for SyncMenuMsgSwapItemSlots {
    // swap item slots content
    fn process(self, world: &mut SyncWorld, pk: JoinedPlayerKey) {
        let SyncMenuMsgSwapItemSlots([a, b]) = self;
        // TODO
    }
}
