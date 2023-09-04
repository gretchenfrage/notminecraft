
use crate::{
    asset::Assets,
    gui::prelude::*,
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
    title_text: GuiTextBlock<true>,
    info1_text: GuiTextBlock<false>,
    info2_text: GuiTextBlock<false>,
    ipinfo_text: GuiTextBlock<false>,
    connect_button: MenuButton,
    cancel_button: MenuButton,
}

impl MultiplayerMenu {
    pub fn new(ctx: &GuiGlobalContext) -> Self
    {
        let title_text = GuiTextBlock::new(&GuiTextBlockConfig {
            text: &ctx.assets.lang.multiplayer_title,
            font: ctx.assets.font,
            logical_font_size: 16.0,
            color: Rgba::white(),
            h_align: HAlign::Center,
            v_align: VAlign::Top,
        });
        let info1_text = GuiTextBlock::new(&GuiTextBlockConfig {
            text: &ctx.assets.lang.multiplayer_info1,
            font: ctx.assets.font,
            logical_font_size: 16.0,
            color: hex_color(0xa0a0a0ff),
            h_align: HAlign::Left,
            v_align: VAlign::Top,
        });
        let info2_text = GuiTextBlock::new(&GuiTextBlockConfig {
            text: &ctx.assets.lang.multiplayer_info2,
            font: ctx.assets.font,
            logical_font_size: 16.0,
            color: hex_color(0xa0a0a0ff),
            h_align: HAlign::Left,
            v_align: VAlign::Top,
        });
        let ipinfo_text = GuiTextBlock::new(&GuiTextBlockConfig {
            text: &ctx.assets.lang.multiplayer_ipinfo,
            font: ctx.assets.font,
            logical_font_size: 16.0,
            color: hex_color(0xa0a0a0ff),
            h_align: HAlign::Left,
            v_align: VAlign::Top,
        });
        let connect_button = menu_button(&ctx.assets.lang.multiplayer_connect)
            .build(&ctx.assets);
        let cancel_button = menu_button(&ctx.assets.lang.gui_cancel)
            .build(&ctx.assets);
        MultiplayerMenu {
            title_text,
            info1_text,
            info2_text,
            ipinfo_text,
            connect_button,
            cancel_button,
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
                logical_size([562.0, 400.0],
                    v_align(0.0,
                        v_stack(0.0, (
                            &mut self.title_text,
                            logical_height(64.0,
                                gap()
                            ),
                            h_align(0.0,
                                &mut self.info1_text,
                            ),
                            logical_height(2.0,
                                gap()
                            ),
                            h_align(0.0,
                                &mut self.info2_text,
                            ),
                            logical_height(38.0,
                                gap(),
                            ),
                            h_align(0.0,
                                &mut self.ipinfo_text,
                            ),
                            logical_height(26.0,
                                gap(),
                            ),
                            h_align(0.5,
                                logical_size([404.0, 44.0],
                                    DebugRed
                                )
                            ),
                            logical_height(58.0,
                                gap(),
                            ),
                            h_align(0.5,
                                logical_width(400.0,
                                    self.connect_button.gui(on_connect_click)
                                )
                            ),
                            logical_height(9.0,
                                gap(),
                            ),
                            h_align(0.5,
                                logical_width(400.0,
                                    self.cancel_button.gui(on_cancel_click)
                                )
                            ),
                        ))
                    )
                )
            )
        ))
    }
}

impl GuiStateFrame for MultiplayerMenu {
    impl_visit_nodes!();
}

fn on_connect_click(ctx: &GuiGlobalContext) {

}

fn on_cancel_click(ctx: &GuiGlobalContext) {

}



use graphics::prelude::*;

#[derive(Debug)]
struct DebugRed;

impl<'a> GuiNode<'a> for SimpleGuiBlock<DebugRed> {
    never_blocks_cursor_impl!();

    fn draw(self, ctx: GuiSpatialContext<'a>, canvas: &mut Canvas2) {
        canvas.reborrow()
            .color(Rgba::red())
            .draw_solid(self.size);
    }
}
