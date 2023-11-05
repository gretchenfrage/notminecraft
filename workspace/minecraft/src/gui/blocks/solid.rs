
use crate::gui::{
    GuiNode,
    DimParentSets,
    GuiSpatialContext,
    GuiBlock,
};
use super::simple_gui_block::{
    SimpleGuiBlock,
    never_blocks_cursor_impl,
};
use graphics::prelude::*;
use vek::*;


/// Gui block which display a solid colored rectangle.
pub fn solid<'a>(color: impl Into<Rgba<f32>>) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
    GuiSolidBlock(color.into())
}


#[derive(Debug)]
struct GuiSolidBlock(Rgba<f32>);

impl<'a> GuiNode<'a> for SimpleGuiBlock<GuiSolidBlock> {
    never_blocks_cursor_impl!();

    fn draw(self, _: GuiSpatialContext, canvas: &mut Canvas2) {
        canvas.reborrow()
            .color(self.inner.0)
            .draw_solid(self.size);
    }
}
