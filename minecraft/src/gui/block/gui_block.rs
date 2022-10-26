
use crate::gui::{
    context::GuiGlobalContext,
    node::{
        GuiNode,
        GuiVisitorTarget,
        GuiVisitor,
    },
    block::dim_constraint::DimConstraint,
};


/// GUI block, often borrowing from some part of a `&'a mut FrameState`, not
/// yet sized or positioned.
pub trait GuiBlock<'a, W: DimConstraint, H: DimConstraint> {
    /// Sized version of self.
    type Sized: SizedGuiBlock<'a>;

    /// Compute the size of this block. Position is not yet knowable.
    fn size(
        self,
        ctx: &GuiGlobalContext,
        w_in: W::In,
        h_in: H::In,
        scale: f32,
    ) -> (W::Out, H::Out, Self::Sized);
}

/// GUI block after being fixed to a particular size. Not yet positioned.
///
/// Auto-impl'd for `GuiNode` impls. A `SizedGuiBlock<'a>` is isomorphic to a
/// `() => [GuiNode<'a>]` function, we simply find it makes the API nicer for
/// it to be done in this way. As such, a `GuiNode => [GuiNode]` conversion is
/// quite natural.
pub trait SizedGuiBlock<'a> {
    /// Visit this block's nodes and subnodes in order. The visitor carries
    /// with it position data. Further transformations may be applied to the
    /// visitor before passing it to `visit_nodes` calls on child
    /// `SizedGuiBlock`s.
    fn visit_nodes<T: GuiVisitorTarget<'a>>(
        self,
        visitor: GuiVisitor<T>,
    );
}


impl<'a, N: GuiNode<'a>> SizedGuiBlock<'a> for N {
    fn visit_nodes<T: GuiVisitorTarget<'a>>(self, visitor: GuiVisitor<'_, '_, T>) {
        visitor.visit_node(self);
    }
}
