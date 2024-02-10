//! The escape menu.

use crate::{
    client::menu_mgr::*,
    gui::prelude::*,
    util_hex_color::*,
};
use vek::*;


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
        menu_setter: MenuSetter<'a>,
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
