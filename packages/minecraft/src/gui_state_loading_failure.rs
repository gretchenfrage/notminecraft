
use crate::gui::prelude::*;
use std::fmt::{self, Formatter, Debug};
use vek::*;
use anyhow::Error;


/// Gui state for an loading failure screen.
pub struct LoadingFailureMenu {
    error_text: GuiTextBlock<true>,
    done_button: MenuButton,
}

impl LoadingFailureMenu {
    pub fn new(ctx: &GuiGlobalContext, error: Error) -> Self {
        let error_text = GuiTextBlock::new(&GuiTextBlockConfig {
            text: &error.to_string(),
            font: ctx.assets.font,
            logical_font_size: 16.0,
            color: Rgba::white(),
            h_align: HAlign::Center,
            v_align: VAlign::Top,
            shadow: true,
        });
        let done_button = menu_button("Done").build(&ctx.assets);
        LoadingFailureMenu {
            error_text,
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
                logical_width(562.0,
                    v_stack(0.0, (
                        &mut self.error_text,
                        logical_height(64.0, gap()),
                        h_align(0.5,
                            logical_width(400.0,
                                self.done_button.gui(on_done_click)
                            )
                        ),
                    ))
                )
            )
        ))
    }
}

impl GuiStateFrame for LoadingFailureMenu {
    impl_visit_nodes!();
}

fn on_done_click(ctx: &GuiGlobalContext) {
    ctx.pop_state_frame();
}

impl Debug for LoadingFailureMenu {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("LoadingFailureMenu { .. }")
    }
}
