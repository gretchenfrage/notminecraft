
use super::*;
use crate::{
    client::network::*,
    item::*,
    message::*,
};
use std::cmp::min;


/// Reasonable-defaults `ItemGridClickLogic` implementation.
pub fn item_grid_default_click_logic<'a, F>(
    connection: &'a Connection,
    held_slot: &'a Option<ItemStack>,
    idx_to_up_ref: F,
) -> impl ItemGridClickLogic<Option<ItemStack>> + 'a
where
    F: FnOnce(usize) -> UpItemSlotRef + 'a,
{
    ItemGridDefaultClickLogic { connection, held_slot, idx_to_up_ref }
}

struct ItemGridDefaultClickLogic<'a, F> {
    connection: &'a Connection,
    held_slot: &'a Option<ItemStack>,
    idx_to_up_ref: F,
}

impl<
    'a,
    F: FnOnce(usize) -> UpItemSlotRef,
> ItemGridClickLogic<Option<ItemStack>> for ItemGridDefaultClickLogic<'a, F> {
    fn handle_click(
        self,
        item_slot_idx: usize,
        item_slot: &Option<ItemStack>,
        button: MouseButton,
        game: &Arc<GameData>,
    ) {
        let target = (self.idx_to_up_ref)(item_slot_idx);
        let msg = if button == MouseButton::Left {
            // left click
            match (self.held_slot.as_ref(), item_slot.as_ref()) {
                (Some(held_stack), Some(item_stack)) => {
                    // both slots occupied
                    let stackable = held_stack.iid == item_stack.iid
                        && held_stack.meta == item_stack.meta
                        && held_stack.damage == item_stack.damage;
                    if stackable {
                        let stack_limit = game.items_max_count[item_stack.iid].get();
                        let max_deposit = min(
                            stack_limit.saturating_sub(item_stack.count.get()),
                            held_stack.count.get(),
                        );
                        if max_deposit > 0 {
                            // try to deposit in items
                            Some(SyncMenuMsg::TransferItems(SyncMenuMsgTransferItems {
                                from: UpItemSlotRef::Held,
                                to: target,
                                amount: max_deposit,
                            }))
                        } else {
                            // cannot deposit items
                            let max_withdrawal = min(
                                stack_limit.saturating_sub(held_stack.count.get()),
                                item_stack.count.get(),
                            );
                            if max_withdrawal > 0 {
                                // try to withdrawal out items
                                Some(SyncMenuMsg::TransferItems(SyncMenuMsgTransferItems {
                                    from: target,
                                    to: UpItemSlotRef::Held,
                                    amount: max_withdrawal,
                                }))
                            } else {
                                // cannot deposit nor withdrawal
                                None
                            }
                        }
                    } else {
                        // not stackable with each other, so swap them
                        Some(SyncMenuMsg::SwapItemSlots(SyncMenuMsgSwapItemSlots(
                            [UpItemSlotRef::Held, target]
                        )))
                    }
                }
                (Some(held_stack), None) => {
                    // only held occupied, so try to deposit it all (unless it's overstacked)
                    Some(SyncMenuMsg::TransferItems(SyncMenuMsgTransferItems {
                        from: UpItemSlotRef::Held,
                        to: target,
                        amount: min(
                            held_stack.count.get(),
                            game.items_max_count[held_stack.iid].get(),
                        ),
                    }))
                }
                (None, Some(item_stack)) => {
                    // only target occupied, so try to withdraw it all (unless it's overstacked)
                    Some(SyncMenuMsg::TransferItems(SyncMenuMsgTransferItems {
                        from: target,
                        to: UpItemSlotRef::Held,
                        amount: min(
                            item_stack.count.get(),
                            game.items_max_count[item_stack.iid].get()
                        ),
                    }))
                }
                (None, None) => {
                    // neither occupied, so nothing can be done
                    None
                }
            }
        } else if button == MouseButton::Right {
            // right click
            match (self.held_slot.as_ref(), item_slot.as_ref()) {
                (Some(held_stack), Some(item_stack)) => {
                    // both slots occupied
                    let stackable = held_stack.iid == item_stack.iid
                        && held_stack.meta == item_stack.meta
                        && held_stack.damage == item_stack.damage;
                    if stackable {
                        // try to deposit in one, unless at stack limit
                        let stack_limit = game.items_max_count[item_stack.iid].get();
                        if stack_limit > item_stack.count.get() {
                            Some(SyncMenuMsg::TransferItems(SyncMenuMsgTransferItems {
                                from: UpItemSlotRef::Held,
                                to: target,
                                amount: 1,
                            }))
                        } else {
                            None
                        }
                    } else {
                        // not stackable with each other, so swap them
                        Some(SyncMenuMsg::SwapItemSlots(SyncMenuMsgSwapItemSlots(
                            [UpItemSlotRef::Held, target]
                        )))
                    }
                }
                (Some(_held_stack), None) => {
                    // only held occupied, so try to deposit in one
                    Some(SyncMenuMsg::TransferItems(SyncMenuMsgTransferItems {
                        from: UpItemSlotRef::Held,
                        to: target,
                        amount: 1,
                    }))
                }
                (None, Some(item_stack)) => {
                    // only target occupied, so try to withdrawal half (unless it's overstacked)
                    Some(SyncMenuMsg::TransferItems(SyncMenuMsgTransferItems {
                        from: target,
                        to: UpItemSlotRef::Held,
                        amount: min(
                            // round up
                            item_stack.count.get() / 2
                                + if item_stack.count.get() % 2 == 0 { 0 } else { 1 },
                            game.items_max_count[item_stack.iid].get()
                        ),
                    }))
                }
                (None, None) => {
                    // neither occupied, so nothing can be done
                    None
                }
            }
        } else {
            None
        };
        if let Some(msg) = msg {
            self.connection.send(UpMsg::PlayerMsg(PlayerMsg::SyncMenuMsg(msg)));
        }
    }
}
