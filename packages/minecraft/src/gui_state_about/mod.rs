
use crate::{
    gui::prelude::*,
    util_hex_color::hex_color,
};
use graphics::prelude::*;
use vek::*;


#[derive(Debug)]
pub struct AboutMenu {
    title_text: GuiTextBlock<true>,
    info_text: GuiTextBlock<true>,
    done_button: MenuButton,
}

impl AboutMenu {
    pub fn new(ctx: &GuiGlobalContext) -> Self
    {
        let title_text = GuiTextBlock::new(&GuiTextBlockConfig {
            text: "About",
            font: ctx.assets.font,
            logical_font_size: 16.0,
            color: Rgba::white(),
            h_align: HAlign::Center,
            v_align: VAlign::Top,
            shadow: true,
        });
        let info_text = GuiTextBlock::new(&GuiTextBlockConfig {
            text: include_str!("about.txt"),
            font: ctx.assets.font,
            logical_font_size: 16.0,
            color: hex_color(0xa0a0a0ff),
            h_align: HAlign::Left,
            v_align: VAlign::Top,
            shadow: true,
        });
        let done_button = menu_button(&ctx.assets.lang.gui_done)
            .build(&ctx.assets);
        AboutMenu {
            title_text,
            info_text,
            done_button,
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
                            logical_height(64.0, gap()),
                            &mut self.info_text,
                            logical_height(58.0, gap()),
                            h_align(0.5,
                                logical_width(400.0,
                                    self.done_button.gui(on_done_click)
                                )
                            ),
                        ))
                    )
                )
            )
        ))
    }
}

impl GuiStateFrame for AboutMenu {
    impl_visit_nodes!();
}

fn on_done_click(ctx: &GuiGlobalContext) {
    ctx.pop_state_frame();
}
