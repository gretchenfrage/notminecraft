//! The escape menu.

use crate::{
    client::{
        menu_mgr::*,
        item_grid::*,
    },
    gui::prelude::*,
    util_hex_color::*,
};


/// The inventory menu.
#[derive(Debug)]
pub struct InventoryMenu {
    crafting: GuiTextBlock<false>,
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
        InventoryMenu { crafting }
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
                    margin(7.0 * 2.0, 0.0, 83.0 * 2.0, 0.0,
                        align(0.0,
                            item_grid_gui_block(
                                &client.inventory_slots.inventory_slots[9..],
                                ItemGridDefaultLayout::new(9),
                                ItemGridDefaultRenderLogic {
                                    item_mesh: client.item_mesh,
                                },
                                ItemGridDefaultClickLogic {},
                            )
                        )
                    ),
                    margin(7.0 * 2.0, 0.0, 141.0 * 2.0, 0.0,
                        align(0.0,
                            item_grid_gui_block(
                                &client.inventory_slots.inventory_slots[..9],
                                ItemGridDefaultLayout::new(9),
                                ItemGridDefaultRenderLogic {
                                    item_mesh: client.item_mesh,
                                },
                                ItemGridDefaultClickLogic {},
                            )
                        )
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
