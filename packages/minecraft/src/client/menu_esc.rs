//! The escape menu.

use crate::{
    client::menu_mgr::*,
    gui::prelude::*,
};
use vek::*;


/// The escape menu.
#[derive(Debug)]
pub struct EscMenu {
    title: GuiTextBlock<true>,

}

impl EscMenu {
    pub fn new(ctx: &GuiGlobalContext) -> Self {
        let title = GuiTextBlock::new(&GuiTextBlockConfig {
            text: "Game menu",
            font: ctx.assets.font,
            logical_font_size: 16.0,
            color: Rgba::white(),
            h_align: HAlign::Center,
            v_align: VAlign::Bottom,
        });
        EscMenu { title }
    }

    pub fn gui<'a>(&'a mut self) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
        align(0.5,
            logical_size([400.0, 320.0],
                v_align(0.0,
                    v_stack(0.0, (
                        &mut self.title,
                        /*
                        logical_height(72.0, gap()),
                        resources.exit_menu_button.gui(on_exit_menu_click(&resources.effect_queue)),
                        logical_height(8.0, gap()),
                        resources.exit_game_button.gui(on_exit_game_click),
                        logical_height(8.0, gap()),
                        resources.open_to_lan_button.gui(on_open_to_lan_click(args.internal_server)),
                        logical_height(56.0 - 48.0, gap()),
                        resources.options_button.gui(on_options_click(&resources.effect_queue)),
                        */
                    ))
                )
            )
        )
    }

    pub fn on_key_press(&mut self, key: PhysicalKey, menu_setter: MenuSetter) {
        if key == KeyCode::Escape {
            menu_setter.clear_menu();
        }
    }
}
