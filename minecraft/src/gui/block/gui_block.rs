
use crate::gui::{
    context::GuiGlobalContext,
    node::{
        GuiVisitorTarget,
        GuiVisitor,
    },
    block::dim_constraint::DimConstraint,
};


pub trait GuiBlock<'a, W: DimConstraint, H: DimConstraint> {
    type Sized: SizedGuiBlock<'a>;

    fn size(
        self,
        ctx: &GuiGlobalContext,
        w_in: W::In,
        h_in: H::In,
        scale: f32,
    ) -> (W::Out, H::Out, Self::Sized);
}

pub trait SizedGuiBlock<'a> {
    fn visit_nodes<T: GuiVisitorTarget<'a>>(
        self,
        visitor: GuiVisitor<T>,
    );
}
