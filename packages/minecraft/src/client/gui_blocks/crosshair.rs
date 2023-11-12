
use crate::gui::prelude::*;
use graphics::{
    prelude::*,
    frame_content::{
        DrawObj2,
        DrawInvert,
    },
};


/// GUI block for rendering the crosshair.
#[derive(Debug)]
pub struct Crosshair;

impl<'a> GuiNode<'a> for SimpleGuiBlock<Crosshair> {
    never_blocks_cursor_impl!();

    fn draw(self, ctx: GuiSpatialContext, canvas: &mut Canvas2) {
        canvas.reborrow()
            .scale(self.size)
            .draw(DrawObj2::Invert(DrawInvert {
                image: ctx.assets().hud_crosshair.clone(),
                tex_index: 0,
                tex_start: 0.0.into(),
                tex_extent: 1.0.into(),
            }));
    }
}
