//! Submodule for highly minecraft-specific gui blocks.

mod splash_text;
mod title;
mod on_click;
mod button_bg;
mod menu_button;
mod game_gui_macro;

pub use self::{
    splash_text::GuiSplashText,
    title::{
        GuiTitleBlock,
        GuiTitleNode,
    },
    on_click::{
        on_left_click,
        on_right_click,
        on_middle_click,
        on_click,
        on_any_click,
        click_sound,
    },
    button_bg::button_bg,
    menu_button::{
        MenuButton,
        MenuButtonBuilder,
        menu_button,
    },
    game_gui_macro::{
        game_gui,
        item_grid,
    },
};
