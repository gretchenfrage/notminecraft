
use crate::gui::{
	block::{
		dim_constraint::DimConstraint,
		gui_block::{
			GuiBlock,
			SizedGuiBlock,
		},
	},
	context::GuiGlobalContext,
	node::{
		GuiVisitor,
		GuiVisitorTarget,
	},
};


#[derive(Debug)]
pub enum GuiEither<A, B> {
	A(A),
	B(B),
}

impl<
	'a,
	W: DimConstraint,
	H: DimConstraint,
	A: GuiBlock<'a, W, H>,
	B: GuiBlock<'a, W, H>,
> GuiBlock<'a, W, H> for GuiEither<A, B> {
	type Sized = GuiEither<A::Sized, B::Sized>;

	fn size(
	 	self,
		ctx: &GuiGlobalContext<'a>,
		w_in: W::In,
		h_in: H::In,
		scale: f32
	) -> (W::Out, H::Out, Self::Sized) {
	 	match self {
	 		GuiEither::A(a) => {
	 			let (w_out, h_out, sized) = a.size(
		 			ctx,
		 			w_in,
		 			h_in,
		 			scale,
		 		);
		 		(w_out, h_out, GuiEither::A(sized))
		 	}
		 	GuiEither::B(b) => {
	 			let (w_out, h_out, sized) = b.size(
		 			ctx,
		 			w_in,
		 			h_in,
		 			scale,
		 		);
		 		(w_out, h_out, GuiEither::B(sized))
		 	}
	 	}
	 }
}

impl<
	'a,
	A: SizedGuiBlock<'a>,
	B: SizedGuiBlock<'a>,
> SizedGuiBlock<'a> for GuiEither<A, B> {
	fn visit_nodes<T: GuiVisitorTarget<'a>>(
		self,
		visitor: &mut GuiVisitor<'a, '_, T>,
		forward: bool
	) {
		match self {
			GuiEither::A(a) => a.visit_nodes(visitor, forward),
			GuiEither::B(b) => b.visit_nodes(visitor, forward),
		}
	}
}

