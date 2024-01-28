
use crate::gui::prelude::*;
use graphics::prelude::*;
use std::fmt::{self, Formatter, Debug};
use vek::*;


/// Gui state for a loading screen.
pub struct LoadingMenu {
    loading_text: GuiTextBlock<true>,
    cancel_button: MenuButton,
    oneshot: Box<dyn LoadingOneshot>,
}

/// Loading menu queries this every frame to see if loading is done. If dropped should cancel the
/// loading.
pub trait LoadingOneshot {
    fn poll(&mut self) -> Option<Box<dyn GuiStateFrameObj>>;
}

impl LoadingMenu {
    pub fn new(ctx: &GuiGlobalContext, oneshot: Box<dyn LoadingOneshot>) -> Self {
        let loading_text = GuiTextBlock::new(&GuiTextBlockConfig {
            text: "Loading...",
            font: ctx.assets.font,
            logical_font_size: 16.0,
            color: Rgba::white(),
            h_align: HAlign::Center,
            v_align: VAlign::Top,
        });
        let cancel_button = menu_button(&ctx.assets.lang.gui_cancel)
            .build(&ctx.assets);
        LoadingMenu {
            loading_text,
            cancel_button,
            oneshot,
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
                        &mut self.loading_text,
                        logical_height(64.0, gap()),
                        h_align(0.5,
                            logical_width(400.0,
                                self.cancel_button.gui(on_cancel_click)
                            )
                        ),
                    ))
                )
            )
        ))
    }
}

impl GuiStateFrame for LoadingMenu {
    impl_visit_nodes!();

    fn update(&mut self, ctx: &GuiWindowContext, _: f32) {
        if let Some(loaded) = self.oneshot.poll() {
            ctx.global().pop_state_frame();
            ctx.global().push_state_frame_obj(loaded);
        }
    }
}

fn on_cancel_click(ctx: &GuiGlobalContext) {
    ctx.pop_state_frame();
}


impl Debug for LoadingMenu {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("LoadingMenu { .. }")
    }
}
