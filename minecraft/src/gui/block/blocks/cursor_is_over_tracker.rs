
use crate::gui::{
    GuiContext,
    block::{
        DimParentSets,
        GuiBlock,
        SimpleGuiNode,
    },
};
use vek::*;


/// Gui block that sets an output variable to whether the cursor is over it.
///
/// Does not render, does not clip cursor.
pub fn cursor_is_over_tracker<'a, 'v>(var: &'v mut bool) -> impl GuiBlock<'a, DimParentSets, DimParentSets> + 'v {
    CursorIsOverTracker { var }
}


struct CursorIsOverTracker<'v> {
    var: &'v mut bool,
}

impl<'a, 'v> SimpleGuiNode<'a> for CursorIsOverTracker<'v> {
    fn clips_cursor(&self, _size: Extent2<f32>, _scale: f32, _pos: Vec2<f32>) -> bool { false }

    // TODO what if they change? do we want it like, on update, instead?

    fn on_cursor_change(self, size: Extent2<f32>, _scale: f32, ctx: &GuiContext) {
        let cursor_is_over = ctx
            .cursor
            .map(|cursor| cursor.unclipped
                && cursor.pos.x >= 0.0
                && cursor.pos.y >= 0.0
                && cursor.pos.x <= size.w
                && cursor.pos.y <= size.h
            )
            .unwrap_or(false);
        *self.var = cursor_is_over;
    }
}
