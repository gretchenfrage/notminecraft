
use graphics::modifier::Modifier2;
use crate::gui::{
    GuiVisitor,
    GuiVisitorTarget,
    DimConstraint,
    GuiBlock,
    SizedGuiBlock,
    GuiGlobalContext,
};


/// Gui block that simply applies some modifier to its child.
pub fn modify<
    'a,
    W: DimConstraint,
    H: DimConstraint,
    M: Into<Modifier2>,
    I: GuiBlock<'a, W, H>,
>(modifier: M, inner: I) -> impl GuiBlock<'a, W, H> {
    let modifier = modifier.into();
    Modify {
        modifier,
        inner,
    }
}


struct Modify<I> {
    modifier: Modifier2,
    inner: I,
}

impl<
    'a,
    W: DimConstraint,
    H: DimConstraint,
    I: GuiBlock<'a, W, H>,
> GuiBlock<'a, W, H> for Modify<I>
{
    type Sized = Modify<I::Sized>;

    fn size(
        self,
        ctx: &GuiGlobalContext,
        w_in: W::In,
        h_in: H::In,
        scale: f32,
    ) -> (W::Out, H::Out, Self::Sized)
    {
        let (
            w_out,
            h_out,
            inner_sized,
        ) = self.inner.size(ctx, w_in, h_in, scale);
        let sized = Modify {
            modifier: self.modifier,
            inner: inner_sized,
        };
        (w_out, h_out, sized)
    }
}

impl<'a, I: SizedGuiBlock<'a>> SizedGuiBlock<'a> for Modify<I> {
    fn visit_nodes<T: GuiVisitorTarget<'a>>(
        self,
        visitor: &mut GuiVisitor<'_, T>,
    ) {
        self.inner.visit_nodes(&mut visitor.reborrow()
            .debug_tag("modify")
            .modify(self.modifier));
    }
}
