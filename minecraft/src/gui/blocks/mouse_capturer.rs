
use crate::gui::{
    blocks::simple_gui_block::{
        SimpleGuiBlock,
        simple_blocks_cursor_impl,
    },
    GuiNode,
    GuiSpatialContext,
    MouseButton,
    DimParentSets,
    GuiBlock,
};


/// GUI block that captures the mouse when clicked.
pub fn mouse_capturer<'a>() -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
    MouseCapturer
}

#[derive(Debug)]
struct MouseCapturer;

impl<'a> GuiNode<'a> for SimpleGuiBlock<MouseCapturer> {
    simple_blocks_cursor_impl!();

    fn on_cursor_click(
        self,
        ctx: GuiSpatialContext,
        hits: bool,
        _button: MouseButton,
    ) {
        // capture mouse on click
        if !hits { return };
        ctx.global.capture_mouse();
    }
}
