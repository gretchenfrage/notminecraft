//! Module for applying edits to the client world.

use crate::{
    message::*,
    block_update_queue::BlockUpdateQueue,
    item::*,
    util::sparse_vec::SparseVec,
};
use chunk_data::*;
use std::{
    mem::replace,
    fmt::Debug,
};
use vek::*;


/// Parts of the client world that may be mutated by applying an edit to it.
#[derive(Debug)]
pub struct EditWorld<'a> {
    pub chunks: &'a LoadedChunks,
    pub getter: &'a Getter<'a>,
    pub ci_reverse_lookup: &'a SparseVec<Vec3<i64>>,
    pub tile_blocks: &'a mut PerChunk<ChunkBlocks>,
    pub block_updates: &'a mut BlockUpdateQueue,
    pub inventory_slots: &'a mut [ItemSlot; 36],
}

/// Trait implemented for edit variants. Directly used by the prediction manager.
pub trait EditVariant: Debug + Into<Edit> {
    /// Key type for this edit's scope within scope variant.
    type Key;

    /// Get key.
    fn key(&self, world: &EditWorld) -> Self::Key;

    /// Apply the edit to the client world. Return the reverser.
    fn apply(self, world: &mut EditWorld) -> Edit;
}

impl EditVariant for edit::Tile {
    type Key = TileKey;

    fn key(&self, world: &EditWorld) -> Self::Key {
        let &edit::Tile { ci, lti, edit: _ } = self;
        TileKey {
            cc: world.ci_reverse_lookup[ci],
            ci,
            lti,
        }
    }

    fn apply(mut self, world: &mut EditWorld) -> Edit {
        let tile = self.key(world);
        self.edit = match self.edit {
            TileEdit::SetTileBlock(tile_edit::SetTileBlock {
                bid_meta
            }) => {
                let old_bid_meta = tile
                    .get(&mut *world.tile_blocks)
                    .erased_replace(bid_meta);
                let gtc = tile.gtc();
                for z in -1..=1 {
                    for y in -1..=1 {
                        for x in -1..=1 {
                            world.block_updates.enqueue(gtc + Vec3 { x, y, z }, world.getter);
                        }
                    }
                }

                tile_edit::SetTileBlock {
                    bid_meta: old_bid_meta,
                }.into()
            }
        };
        self.into()
    }
}

impl EditVariant for edit::InventorySlot {
    type Key = usize;

    fn key(&self, _: &EditWorld) -> Self::Key {
        let &edit::InventorySlot { slot_idx, edit: _ } = self;
        slot_idx
    }

    fn apply(mut self, world: &mut EditWorld) -> Edit {
        let slot = &mut world.inventory_slots[self.slot_idx];
        self.edit = match self.edit {
            InventorySlotEdit::SetInventorySlot(inventory_slot_edit::SetInventorySlot {
                slot_val,
            }) => {
                let old_slot_val = replace(slot, slot_val);
                inventory_slot_edit::SetInventorySlot {
                    slot_val: old_slot_val,
                }.into()
            }
        };
        self.into()
    }
}
