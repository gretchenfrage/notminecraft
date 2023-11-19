
use crate::{
    item::*,
    gui::prelude::*,
    game_data::{
        content::chest::ChestBlockMeta,
        *,
    },
    client::{
        connection::Connection,
        PredictionToMake,
    },
    message::*,
};
use chunk_data::*;
use std::{
    sync::Arc,
    collections::VecDeque,
    cell::RefCell,
};


pub trait ItemSlotClickLogic {
    fn on_click(
        self,
        slot_idx: usize,
        slot: &ItemSlot,
        button: MouseButton,
        game: &Arc<GameData>,
    );
}

#[derive(Debug, Copy, Clone)]
pub struct NoninteractiveItemSlotClickLogic;

impl ItemSlotClickLogic for NoninteractiveItemSlotClickLogic {
    fn on_click(
        self,
        _slot_idx: usize,
        _slot: &ItemSlot,
        _button: MouseButton,
        _game: &Arc<GameData>,
    ) {}
}

#[derive(Debug)]
pub struct MultiplayerItemSlotClickLogic<'a> {
    pub slot_offset: usize,
    pub connection: &'a Connection,
    pub open_menu_msg_idx: u64,
    pub predictions_to_make: &'a RefCell<VecDeque<PredictionToMake>>,
    pub idx_space: MultiplayerItemSlotIdxSpace<'a>,
}

#[derive(Debug, Clone)]
pub enum MultiplayerItemSlotIdxSpace<'a> {
    Inventory,
    Chest {
        ci: usize,
        lti: u16,
        meta: &'a ChestBlockMeta,
    },
}

impl<'a> ItemSlotClickLogic for MultiplayerItemSlotClickLogic<'a> {
    fn on_click(
        self,
        slot_idx: usize,
        _slot: &ItemSlot,
        button: MouseButton,
        game: &Arc<GameData>,
    ) {
        /*
        if button == MouseButton::Middle {
            let slot_idx = slot_idx + self.slot_offset;
            let stack = ItemStack::new(game.content.stone.iid_stone, ());

            self.connection.send(up::ItemSlotAdd {
                slot: slot_idx,
                open_menu_msg_idx: self.open_menu_msg_idx,
                stack: stack.clone(),
            });

            let edit = match self.idx_space {
                MultiplayerItemSlotIdxSpace::Inventory => edit::InventorySlot {
                    slot_idx: slot_idx,
                    edit: inventory_slot_edit::SetInventorySlot {
                        slot_val: Some(stack),
                    }.into(),
                }.into(),
                MultiplayerItemSlotIdxSpace::Chest { ci, lti, meta } => {
                    let mut meta2 = meta.clone();
                    meta2.slots[slot_idx - 36] = Some(stack.clone());
                    edit::Tile {
                        ci,
                        lti,
                        edit: tile_edit::SetTileBlock {
                            bid_meta: ErasedBidMeta::new(
                                game.content.chest.bid_chest,
                                meta2,
                            ),
                        }.into(),
                    }.into()
                },
            };
            self.predictions_to_make.borrow_mut().push_back(PredictionToMake {
                edit,
                up_msg_idx: self.connection.up_msg_idx(),
            });
        }
        */
    }
}


/*
#[derive(Debug, Copy, Clone)]
pub struct StorageItemSlotClickLogic<H> {
    pub held: H,
}

impl<H: BorrowItemSlot> ItemSlotClickLogic for StorageItemSlotClickLogic<H> {
    fn on_click(
        mut self,
        _slot_idx: usize,
        slot_mut: & mut ItemSlot,
        button: MouseButton,
        game: &Arc<GameData>,
    ) {
        // borrow
        let mut held_guard = self.held.borrow();
        let held_mut = H::deref(&mut held_guard);

        if button == MouseButton::Left {
            // left click
            // take ownership of both stacks, remember to put them back if we want to
            match (held_mut.take(), slot_mut.take()) {
                (Some(mut held), Some(mut slot)) => {
                    // both held and slot have stack
                    if held.iid == slot.iid
                        && held.meta == slot.meta
                        && held.damage == slot.damage
                    {
                        // stacks have same item

                        // number of items to transfer from held to slot
                        let transfer = u8::min(
                            // number of items in held
                            held.count.get(),
                            // number of additional items slot could receive
                            game.items_max_count[slot.iid].get().saturating_sub(slot.count.get()),
                        );

                        // add to slot, give back ownership
                        slot.count = (slot.count.get() + transfer).try_into().unwrap();
                        *slot_mut = Some(slot);

                        // subtract from held, give back ownership or leave it none
                        if let Ok(held_new_count) = (held.count.get() - transfer).try_into() {
                            held.count = held_new_count;
                            *held_mut = Some(held)
                        }
                    } else {
                        // stacks have different items
                        // swap them
                        *held_mut = Some(slot);
                        *slot_mut = Some(held);
                    }
                }
                (opt_held, opt_slot) => {
                    // otherwise, swap them (regardless of further specifics)
                    *held_mut = opt_slot;
                    *slot_mut = opt_held;
                }
            }
        } else if button == MouseButton::Right {
            // right click
            // take ownership of both stacks, remember to put them back if we want to
            match (held_mut.take(), slot_mut.take()) {
                (Some(mut held), Some(mut slot)) => {
                    // both held and slot have stack
                    if held.iid == slot.iid
                        && held.meta == slot.meta
                        && held.damage == slot.damage
                    {
                        // stacks have same item
                        if let Some(slot_new_count) = slot.count.get()
                            .checked_add(1)
                            .filter(|&n| n <= game.items_max_count[held.iid].get())
                        {
                            // slot has room for another item
                            
                            // add to slot, give back ownership
                            slot.count = slot_new_count.try_into().unwrap();
                            *slot_mut = Some(slot);

                            // subtract from held, give back ownership or leave it none
                            if let Ok(held_new_count) = (held.count.get() - 1).try_into() {
                                held.count = held_new_count;
                                *held_mut = Some(held)
                            }
                        } else {
                            // slot is full
                            // give back ownership of both without modifying
                            *held_mut = Some(held);
                            *slot_mut = Some(slot);
                        }
                    } else {
                        // stacks have different items
                        // swap them
                        *held_mut = Some(slot);
                        *slot_mut = Some(held);
                    }
                }
                (Some(mut held), None) => {
                    // only held has stack

                    // put one item in slot
                    *slot_mut = Some(ItemStack {
                        iid: held.iid,
                        meta: held.meta.clone(),
                        count: 1.try_into().unwrap(),
                        damage: held.damage,
                    });

                    // subtract from held, give back ownership or leave it none
                    if let Ok(held_new_count) = (held.count.get() - 1).try_into() {
                        held.count = held_new_count;
                        *held_mut = Some(held);
                    }

                }
                (None, Some(mut slot)) => {
                    // only slot has stack

                    // amount to leave = half, round down
                    let slot_new_count = slot.count.get() / 2;
                    // amount to take = half, round up
                    let held_new_count = slot.count.get() - slot_new_count;

                    // put in held
                    *held_mut = Some(ItemStack {
                        iid: slot.iid,
                        meta: slot.meta.clone(),
                        count: held_new_count.try_into().unwrap(),
                        damage: slot.damage,
                    });

                    // subtract from slot, give back ownership or leave it none
                    if let Ok(slot_new_count) = slot_new_count.try_into() {
                        slot.count = slot_new_count;
                        *slot_mut = Some(slot)
                    }
                }
                (None, None) => {} // both are empty, nothing to do
            }
        }
    }
}
*/
