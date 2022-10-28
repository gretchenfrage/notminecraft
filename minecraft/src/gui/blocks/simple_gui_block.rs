

use crate::gui::{
	GuiNode,
	GuiBlock,
	DimParentSets,
	GuiGlobalContext,
};
use vek::*;


/// Utility for GUI blocks with parent-set size and trivial sizing logic.
///
/// A blanket impl makes it so that:
/// `SimpleGuiBlock<I>: GuiNode` -> `I: GuiBlock`.
///
/// Due to the orphan rule, it would not be useful to expose this to other
/// crates.
pub struct SimpleGuiBlock<I> {
	pub inner: I,
	pub size: Extent2<f32>,
	pub scale: f32,
}

impl<'a, I> GuiBlock<'a, DimParentSets, DimParentSets> for I
where
	SimpleGuiBlock<I>: GuiNode<'a>,
{
	type Sized = SimpleGuiBlock<I>;

	fn size(
		self,
		_: &GuiGlobalContext,
		w: f32,
		h: f32,
		scale: f32,
	) -> ((), (), Self::Sized)
    {
    	let sized = SimpleGuiBlock {
    		inner: self,
    		size: Extent2 { w, h },
    		scale,
    	};
    	((), (), sized)
    }
}


/// Macro to implement `GuiNode::blocks_cursor` on some `SimpleGuiBlock` type
/// with logic that blocks the cursor if it's over the block's rectangular
/// area.
macro_rules! simple_blocks_cursor_impl {
	()=>{
		fn blocks_cursor(&self, pos: Vec2<f32>) -> bool {
			pos.x >= 0.0
				&& pos.y >= 0.0
				&& pos.x <= self.size.w
				&& pos.y <= self.size.h
		}
	};
}

pub(crate) use simple_blocks_cursor_impl;
