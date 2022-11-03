

use crate::gui::{
	GuiNode,
	GuiBlock,
	DimParentSets,
	GuiGlobalContext,
};
use std::fmt::Debug;
use vek::*;


/// Utility for GUI blocks with parent-set size and trivial sizing logic.
///
/// A blanket impl makes it so that:
/// `SimpleGuiBlock<I>: GuiNode` -> `I: GuiBlock`.
///
/// Due to the orphan rule, it would not be useful to expose this to other
/// crates.
#[derive(Debug)]
pub struct SimpleGuiBlock<I> {
	pub inner: I,
	pub size: Extent2<f32>,
	pub scale: f32,
}

impl<'a, I> GuiBlock<'a, DimParentSets, DimParentSets> for I
where
	I: Debug,
	SimpleGuiBlock<I>: GuiNode<'a>,
{
	type Sized = SimpleGuiBlock<I>;

	fn size(
		self,
		_: &GuiGlobalContext<'a>,
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
		fn blocks_cursor(&self, ctx: GuiSpatialContext) -> bool {
			ctx.cursor_in_area(0.0, self.size)
		}
	};
}

pub(crate) use simple_blocks_cursor_impl;
