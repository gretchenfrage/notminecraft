//! Sync state module for the item slots in each players inventory.

use crate::{
    server::{
        ServerSyncCtx,
        per_player::*,
    },
    item::*,
    message::*,
};

/// Both server-side per-player and client-side state for item slots in the player's inventory.
pub struct PlayerInventorySlots {
    pub inventory_slots: [Option<ItemStack>; 36],
    pub held_slot: Option<ItemStack>,
}

/// Auto-syncing writer for this sync state. Analogous to
/// `&mut PerJoinedPlayer<PlayerInventorySlots>`.
pub struct SyncWrite<'a> {
    ctx: &'a ServerSyncCtx,
    state: &'a mut PerJoinedPlayer<PlayerInventorySlots>,
}

impl<'a> SyncWrite<'a> {
    /// Construct manually (with respect to synchronization logic).
    pub fn new_manual(
        ctx: &'a ServerSyncCtx,
        state: &'a mut PerJoinedPlayer<PlayerInventorySlots>,
    ) -> Self {
        SyncWrite { ctx, state }
    }

    /// Get state as a read-only reference.
    pub fn as_ref(&self) -> &PerJoinedPlayer<PlayerInventorySlots> {
        &self.state
    }

    /// Narrow in on a specific player.
    pub fn get(&mut self, pk: JoinedPlayerKey) -> SyncWritePlayer {
        SyncWritePlayer {
            ctx: self.ctx,
            state: &mut self.state[pk],
            pk,
        }
    }
}

/// Auto-syncing writer for this sync state for a player. Analogous to `&mut PlayerInventorySlots`.
pub struct SyncWritePlayer<'a> {
    ctx: &'a ServerSyncCtx,
    state: &'a mut PlayerInventorySlots,
    pk: JoinedPlayerKey,
}

impl<'a> SyncWritePlayer<'a> {
    /// Convert a `&'a2 mut SyncWritePlayer<'_>` to a `SyncWritePlayer<'a2>`.
    pub fn reborrow<'a2>(&'a2 mut self) -> SyncWritePlayer<'a2> {
        SyncWritePlayer {
            ctx: &self.ctx,
            state: &mut self.state,
            pk: self.pk,
        }
    }

    /// Convert into state as a read-only reference.
    pub fn as_ref(self) -> &'a PlayerInventorySlots {
        self.state
    }

    /// Narrow in on a specific inventory slot.
    pub fn inventory_slot(self, idx: usize) -> SyncWriteSlot<'a> {
        SyncWriteSlot {
            ctx: self.ctx,
            state: &mut self.state.inventory_slots[idx],
            pk: self.pk,
            slot_ref: DownItemSlotRef::Inventory(idx.try_into().unwrap()),
        }
    }

    /// Narrow in on the held slot.
    pub fn held_slot(self) -> SyncWriteSlot<'a> {
        SyncWriteSlot {
            ctx: self.ctx,
            state: &mut self.state.held_slot,
            pk: self.pk,
            slot_ref: DownItemSlotRef::Held,
        }
    }
}

/// Auto-syncing writer for this sync state for a slot. Analogous to `&mut Option<ItemStack>`.
pub struct SyncWriteSlot<'a> {
    ctx: &'a ServerSyncCtx,
    state: &'a mut Option<ItemStack>,
    pk: JoinedPlayerKey,
    slot_ref: DownItemSlotRef,
}

impl<'a> SyncWriteSlot<'a> {
    /// Convert a `&'a2 mut SyncWriteSlot<'_>` to a `SyncWriteSlot<'a2>`.
    pub fn reborrow<'a2>(&'a2 mut self) -> SyncWriteSlot<'a2> {
        SyncWriteSlot {
            ctx: &self.ctx,
            state: &mut self.state,
            pk: self.pk,
            slot_ref: self.slot_ref,
        }
    }

    /// Convert into state as a read-only reference.
    pub fn as_ref(self) -> Option<&'a ItemStack> {
        self.state.as_ref()
    }

    /// Set the item slot's content.
    pub fn write(&mut self, content: Option<ItemStack>) {
        // send update to client
        self.ctx.conn_mgr
            .send(self.pk, DownMsg::PostJoin(PostJoinDownMsg::SetItemSlot {
                item_slot: self.slot_ref,
                slot_content: content.clone(),
            }));

        // TODO mark player state as unsaved

        // edit server's in-memory representation
        *self.state = content;
    }
}
