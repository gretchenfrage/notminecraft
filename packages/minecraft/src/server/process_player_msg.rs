//! Process player messages.
//!
//! This is a "game logic" module moreso than a "system" module.

use crate::{
    server::{
        per_player::*,
        SyncWorld,
    },
    message::*,
    sync_state_inventory_slots,
    sync_state_entities::SteveEntityState,
};
use chunk_data::*;
use std::cmp::min;


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
        PlayerMsg::SpawnSteve(pos) => {
            let cc = (pos / CHUNK_EXTENT.map(|n| n as f32)).map(|n| n.floor() as i64);
            let ci = world.getter.get(cc).expect("TODO");
            world.chunk_steves.create_entity(
                cc,
                ci,
                SteveEntityState { vel: Default::default(), name: "Steve".into() },
                Default::default(),
                pos % CHUNK_EXTENT.map(|n| n as f32),
            );
        }
        PlayerMsg::ClearSteves => {
            let mut chunk_steves = world.chunk_steves.iter_move_batch();
            for (cc, ci) in world.sync_ctx.chunk_mgr.chunks().iter() {
                let mut steves = chunk_steves.get(cc, ci);
                while let Some(steve) = steves.next() {
                    steve.delete();
                }
            }
        }
        PlayerMsg::ClockDebug(time) => {
            use std::time::Instant;
            let now = Instant::now();
            debug!(
                "received message from(?) {:.6} ms ago",
                now.saturating_duration_since(world.sync_ctx.conn_mgr.derel_time(pk, time)).as_nanos() as f64 / 1_000_000.0,
            );
        }
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
            world.sync_ctx.conn_mgr.send(pk2, DownMsg::PreJoin(
                PreJoinDownMsg::SetPlayerCharState {
                    player_idx: DownPlayerIdx(
                        world.sync_ctx.conn_mgr.player_to_clientside(pk, pk2)
                    ),
                    pos,
                    yaw,
                    pitch,
                }
            ));
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
            world.sync_ctx.conn_mgr.send(pk, DownMsg::PostJoin(
                PostJoinDownMsg::InvalidateSyncMenu { up_msg_idx }
            ));
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

fn resolve_slot<'a>(
    item_slot: UpItemSlotRef,
    world: &'a mut SyncWorld,
    pk: JoinedPlayerKey,
) -> sync_state_inventory_slots::SyncWriteSlot<'a> {
    match item_slot {
        UpItemSlotRef::Inventory(idx) =>
            world.player_inventory_slots.get(pk).inventory_slot(idx.get()),
        UpItemSlotRef::Held => world.player_inventory_slots.get(pk).held_slot()
    }
}

impl Process for SyncMenuMsgTransferItems {
    // transfer items from one item slot to another
    fn process(self, world: &mut SyncWorld, pk: JoinedPlayerKey) {
        let SyncMenuMsgTransferItems { from, to, amount } = self;
        // clone the from slot, early-return if from slot is empty
        let mut from_stack_clone = match resolve_slot(from, world, pk).as_ref() {
            Some(from_stack_ref) => from_stack_ref.clone(),
            None => return,
        };
        // compute how much we transfer over
        let stack_limit = world.sync_ctx.game.items_max_count[from_stack_clone.iid].get();
        let mut to_stack_write = resolve_slot(to, world, pk);
        let to_can_take = to_stack_write.reborrow().as_ref()
            .map(|to_stack_ref|
                if to_stack_ref.iid == from_stack_clone.iid
                    && to_stack_ref.meta == from_stack_clone.meta
                    && to_stack_ref.damage == from_stack_clone.damage
                {
                    stack_limit.saturating_sub(to_stack_ref.count.get())
                } else { 0 }
            )
            .unwrap_or(stack_limit);
        if to_can_take > 0 {
            let amount_transfer = min(amount, min(to_can_take, from_stack_clone.count.get()));
            let from_final_amount = from_stack_clone.count.get() - amount_transfer;

            // do the movement now
            if let Some(mut to_stack_clone) = to_stack_write.reborrow().as_ref().cloned() {
                // case where they both start non-0 count
                // change to slot count
                to_stack_clone.count = (to_stack_clone.count.get() + amount_transfer)
                    .try_into().unwrap();
                to_stack_write.write(Some(to_stack_clone));

                let mut from_stack_write = resolve_slot(from, world, pk);
                if from_final_amount > 0 {
                    // change from slot count
                    from_stack_clone.count = from_final_amount.try_into().unwrap();
                    from_stack_write.write(Some(from_stack_clone));
                } else {
                    // from slot drops to 0
                    from_stack_write.write(None);
                }
            } else {
                if from_final_amount > 0 {
                    // case where some but not all of from slot is splitting off to fill empty to
                    let mut from_stack_clone_2 = from_stack_clone.clone();
                    from_stack_clone_2.count = amount_transfer.try_into().unwrap();
                    to_stack_write.write(Some(from_stack_clone_2));

                    from_stack_clone.count = from_final_amount.try_into().unwrap();
                    resolve_slot(from, world, pk).write(Some(from_stack_clone));
                } else {
                    // case where the entirety of from slot is moving to to slot
                    debug_assert_eq!(from_stack_clone.count.get(), amount_transfer);
                    to_stack_write.write(Some(from_stack_clone));

                    resolve_slot(from, world, pk).write(None);
                }
            }
        }
    }
}

impl Process for SyncMenuMsgSwapItemSlots {
    // swap item slots content
    fn process(self, world: &mut SyncWorld, pk: JoinedPlayerKey) {
        let SyncMenuMsgSwapItemSlots([a, b]) = self;
        let a_clone = resolve_slot(a, world, pk).as_ref().cloned();
        let mut b_write = resolve_slot(b, world, pk);
        let b_clone = b_write.reborrow().as_ref().cloned();
        b_write.write(a_clone);
        resolve_slot(a, world, pk).write(b_clone);
    }
}
