
use crate::{
    asset::Assets,
    util::hex_color::hex_color,
    gui::{
        GuiBlock,
        DimParentSets,
        DimChildSets,
        GuiGlobalContext,
        blocks::{
            *,
            mc::*,
        },
    },
};
use graphics::frame_content::{
    HAlign,
    VAlign,
};


pub fn menu_button(text: &str) -> MenuButtonBuilder {
    MenuButtonBuilder {
        text,
    }
}


#[derive(Debug)]
pub struct MenuButtonBuilder<'a> {
    pub text: &'a str,
}

impl<'a> MenuButtonBuilder<'a> {
    pub fn build(self, assets: &Assets) -> MenuButton {
        let text = GuiTextBlock::new(&GuiTextBlockConfig {
            text: self.text,
            font: assets.font,
            logical_font_size: 16.0,
            color: hex_color(0xE0E0E0FF),
            h_align: HAlign::Center,
            v_align: VAlign::Center,
            wrap: false,
        });
        MenuButton {
            text,
        }
    }
}


#[derive(Debug)]
pub struct MenuButton {
    text: GuiTextBlock,
}

impl MenuButton {
    pub fn gui<'a, F>(
        &'a mut self,
        on_click: F,
    ) -> impl GuiBlock<'a, DimParentSets, DimChildSets>
    where
        F: for<'r, 's> FnOnce(&'r GuiGlobalContext<'s>),
    {
        logical_height(40.0,
            layer((
                button_bg(),
                &mut self.text,
                click_sound(),
                on_left_click(move |ctx| on_click(ctx.global)),
            )),
        )
    }
}
