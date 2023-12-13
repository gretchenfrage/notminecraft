
use super::{
    Server,
    OnReceived,
    OpenGameMenu,
    per_connection::*,
};
use crate::{
    message::*,
    item::*,
};
use anyhow::*;


impl OnReceived<up::OpenGameMenu> for Server {
    type Ck = ClientConnKey;

    fn on_received(&mut self, msg: up::OpenGameMenu, ck: ClientConnKey) -> Result<()> {
        let up::OpenGameMenu { menu } = msg;
        
        let valid = match &menu {
            &GameMenu::Inventory => true,
            &GameMenu::Chest { gtc: _ } => true, // TODO validation logic
        };
        let open_menu_msg_idx = self.last_processed[ck].num;
        if !valid {
            self.connections[ck].send(down::CloseGameMenu { open_menu_msg_idx });
        }
        self.open_game_menu[ck] = Some(OpenGameMenu {
            menu,
            open_menu_msg_idx,
            valid,
        });

        Ok(())
    }
}

impl OnReceived<up::CloseGameMenu> for Server {
    type Ck = ClientConnKey;

    fn on_received(&mut self, msg: up::CloseGameMenu, ck: ClientConnKey) -> Result<()> {
        let up::CloseGameMenu {} = msg;

        self.open_game_menu[ck] = None;

        Ok(())
    }
}

impl OnReceived<up::GameMenuAction> for Server {
    type Ck = ClientConnKey;

    fn on_received(&mut self, msg: up::GameMenuAction, ck: ClientConnKey) -> Result<()> {
        let up::GameMenuAction { action } = msg;
        match action {
            GameMenuAction::TransferItems { from, to, amount } => {
                if let Some(stack_1) = get_server_item_slot(self, from, ck)
                    .and_then(|slot| slot.take())
                {
                    // if slot 1 found and contains items take those items
                    if let Some(slot_2) = get_server_item_slot(self, to, ck) {
                        // if slot 2 found
                        if let Some(stack_2_mut) = slot_2.as_mut() {
                            // if slot 2 has content

                            // determine how many actual item we'll transfer
                            // use the requested count as a starting point
                            let mut amount_to_transfer: u8 = amount;
                            // but don't transfer more than was initially present
                            amount_to_transfer = amount_to_transfer.min(stack_1.count.get());
                            // if they're mutually not stackable don't transfer anything
                            if stack_1.iid != stack_2_mut.iid
                                || stack_1.meta != stack_2_mut.meta
                                || stack_1.damage != stack_2_mut.damage
                            {
                                amount_to_transfer = 0;
                            }
                            // and don't fill the target past the item's stack limit
                            amount_to_transfer = amount_to_transfer.min(
                                self.game.items_max_count[stack_1.iid].get()
                                    .saturating_sub(stack_2_mut.count.get())
                            );
                            
                            // then modify their counts
                            stack_2_mut.count = (stack_2_mut.count.get() + amount_to_transfer)
                                .try_into().unwrap();

                            if let Some(remaining) = (stack_1.count.get() - amount_to_transfer)
                                .try_into().ok()
                            {
                                let slot_1_mut = get_server_item_slot(self, from, ck).unwrap();
                                let mut stack_1 = stack_1;
                                stack_1.count = remaining;
                                *slot_1_mut = Some(stack_1);
                            }
                        } else {
                            // but if slot 2 is empty, put slot 1's content in slot 2
                            *slot_2 = Some(stack_1);
                        }
                    } else {
                        // but if slot 2 not found, put slot 1's content back
                        *get_server_item_slot(self, from, ck).unwrap() = Some(stack_1);
                    }
                }
            }
            GameMenuAction::SwapItemSlots([slot_ref_1, slot_ref_2]) => {
                if let Some(slot_1) = get_server_item_slot(self, slot_ref_1, ck) {
                    // if slot 1 found, take its content
                    let slot_1_content = slot_1.take();
                    if let Some(slot_2) = get_server_item_slot(self, slot_ref_2, ck) {
                        // then if slot 2 found, replace its content with slot 1's content
                        let slot_2_content = slot_2.take();
                        *slot_2 = slot_1_content;
                        // and put slot 2's content into slot 1
                        *get_server_item_slot(self, slot_ref_1, ck).unwrap() = slot_2_content;
                    } else {
                        // but if slot 2 not found, put slot 1's content back
                        *get_server_item_slot(self, slot_ref_1, ck).unwrap() = slot_1_content;
                    }
                }
            }
        }
        Ok(())
    }
}

fn get_server_item_slot(
    server: &mut Server,
    slot_ref: ItemSlotReference,
    ck: ClientConnKey,
) -> Option<&mut ItemSlot> {
    let open_menu = &server.open_game_menu[ck]
        .as_ref()
        .filter(|open_menu| open_menu.valid)?
        .menu;
    match slot_ref {
        ItemSlotReference::Held => Some(&mut server.held[ck]),
        ItemSlotReference::Inventory(i) => Some(&mut server.inventory_slots[ck][i.get()]),
        ItemSlotReference::Armor(_i) => None,
        ItemSlotReference::InventoryCrafting(_i) => None,
        ItemSlotReference::InventoryCraftingOutput => None,
        ItemSlotReference::Chest(i) => {
            if let &GameMenu::Chest { gtc } = open_menu { Some(gtc) } else { None }
                .and_then(|gtc| server.chunk_mgr.getter().gtc_get(gtc))
                .and_then(|tile| tile
                    .get(&mut server.tile_blocks)
                    .try_meta(server.game.content.chest.bid_chest))
                .map(|chest_meta| &mut chest_meta.slots[i.get()])
        }
    }
} 
