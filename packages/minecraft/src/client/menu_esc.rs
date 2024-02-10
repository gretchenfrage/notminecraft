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
    exit_menu: MenuButton,
    exit_game: MenuButton,
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
            shadow: true,
        });
        let exit_menu = menu_button("Back to game").build(ctx.assets);
        let exit_game = menu_button("Save and quit to title").build(ctx.assets);
        EscMenu { title, exit_menu, exit_game }
    }

    pub fn gui<'a>(
        &'a mut self,
        menu_setter: MenuSetter<'a>,
    ) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
        align(0.5,
            logical_size([400.0, 320.0],
                v_align(0.0,
                    v_stack(0.0, (
                        &mut self.title,
                        logical_height(72.0, gap()),
                        self.exit_menu.gui(move |_| menu_setter.clear_menu()),
                        logical_height(8.0, gap()),
                        self.exit_game.gui(|ctx| ctx.event_loop.borrow_mut().pop_state_frame()),
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
