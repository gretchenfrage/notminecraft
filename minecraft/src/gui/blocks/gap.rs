
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


/// Gui block which contains nothing.
///
/// Can be used to put a gap between elements.
pub fn gap<'a>() -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
    GuiGapBlock
}


#[derive(Debug)]
struct GuiGapBlock;

impl<'a> GuiNode<'a> for SimpleGuiBlock<GuiGapBlock> {
    never_blocks_cursor_impl!();
}
