
use crate::{
    asset::Assets,
    gui::{
        *,
        blocks::{
            *,
            mc::*,
        },
    },
    util::hex_color::hex_color,
    client_server::client::Client,
};
use graphics::{
    Renderer,
    frame_content::{
        HAlign,
        VAlign,
    },
};
use rand::thread_rng;
use vek::*;


#[derive(Debug)]
pub struct MultiplayerMenu {
    title_text: GuiTextBlock,
    //info_text_1: GuiTextBlock,
    //info_text_2: GuiTextBlock,
}

impl MultiplayerMenu {
    pub fn new(ctx: &GuiGlobalContext) -> Self
    {
        let title_text = GuiTextBlock::new(&GuiTextBlockConfig {
            text: &ctx.assets.lang.multiplayer_title,
            font: ctx.assets.font,
            logical_font_size: 16.0,
            color: hex_color(0xE0E0E0FF),
            h_align: HAlign::Center,
            v_align: VAlign::Top,
            wrap: true,
        });
        MultiplayerMenu {
            title_text,
        }
    }

    fn gui<'a>(
        &'a mut self,
        ctx: &'a GuiWindowContext,
    ) -> impl GuiBlock<'a, DimParentSets, DimParentSets>
    {
        layer((
            modify(Rgba::new(0.25, 0.25, 0.25, 1.0),
                tile_image(&ctx.assets().menu_bg, 64.0)
            ),
            align(0.5,
                logical_width(500.0,
                    v_stack(0.0, (
                        logical_height(10.0, &mut self.title_text),
                    ))
                )
            )
        ))
    }
}


impl GuiStateFrame for MultiplayerMenu {
    impl_visit_nodes!();
}
