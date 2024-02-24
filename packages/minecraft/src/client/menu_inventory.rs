//! The escape menu.

use crate::{
    client::{
        menu_mgr::*,
        item_grid::*,
    },
    gui::prelude::*,
    util_hex_color::*,
    util_array::*,
    message::*,
};


/// The inventory menu.
#[derive(Debug)]
pub struct InventoryMenu {
    crafting: GuiTextBlock<false>,
    hotbar_slot_text_caches: [ItemSlotTextCache; 9],
    non_hotbar_slot_text_caches: [ItemSlotTextCache; 27],
    held_slot_text_cache: ItemSlotTextCacheNonhoverable,
}

impl InventoryMenu {
    pub fn new(ctx: &GuiGlobalContext) -> Self {
        let crafting = GuiTextBlock::new(&GuiTextBlockConfig {
            text: "Crafting",
            font: ctx.assets.font,
            logical_font_size: 16.0,
            color: hex_color(0x404040FF),
            h_align: HAlign::Left,
            v_align: VAlign::Bottom,
            shadow: false,
        });
        InventoryMenu {
            crafting,
            hotbar_slot_text_caches: array_default(),
            non_hotbar_slot_text_caches: array_default(),
            held_slot_text_cache: Default::default(),
        }
    }

    pub fn gui<'a>(
        &'a mut self,
        ctx: &GuiGlobalContext<'a>,
        client: MenuGuiClientBorrows<'a>,
    ) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
        align(0.5,
            logical_size([176.0 * 2.0, 166.0 * 2.0],
                layer((
                    &ctx.assets.gui_inventory,
                    margin(172.0, 0.0, 0.0, 166.0 * 2.0 - 46.0,
                        align([0.0, 1.0],
                            &mut self.crafting
                        )
                    ),
                    margin(7.0 * 2.0, 0.0, 141.0 * 2.0, 0.0,
                        align(0.0,
                            item_grid_gui_block(
                                &client.inventory_slots.inventory_slots[..9],
                                ItemGridDefaultLayout::new(9),
                                item_grid_default_render_logic(
                                    client.item_mesh,
                                    &client.inventory_slots.held_slot,
                                    self.hotbar_slot_text_caches.iter_mut(),
                                ),
                                item_grid_default_click_logic(
                                    client.connection,
                                    &client.inventory_slots.held_slot,
                                    |i| UpItemSlotRef::Inventory(i.try_into().unwrap()),
                                ),
                            )
                        )
                    ),
                    margin(7.0 * 2.0, 0.0, 83.0 * 2.0, 0.0,
                        align(0.0,
                            item_grid_gui_block(
                                &client.inventory_slots.inventory_slots[9..],
                                ItemGridDefaultLayout::new(9),
                                item_grid_default_render_logic(
                                    client.item_mesh,
                                    &client.inventory_slots.held_slot,
                                    self.non_hotbar_slot_text_caches.iter_mut(),
                                ),
                                item_grid_default_click_logic(
                                    client.connection,
                                    &client.inventory_slots.held_slot,
                                    |i| UpItemSlotRef::Inventory((i + 9).try_into().unwrap()),
                                ),
                            )
                        )
                    ),
                    self.held_slot_text_cache.held_item_gui_block(
                        client.item_mesh,
                        &client.inventory_slots.held_slot,
                    ),
                ))
            )
        )
    }

    pub fn on_key_press(&mut self, key: PhysicalKey, menu_setter: MenuSetter) {
        if key == KeyCode::Escape || key == KeyCode::KeyE {
            menu_setter.clear_menu();
        }
    }
}
