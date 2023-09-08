
use crate::gui::{
    blocks::identity_maperator::IdentityMaperator,
    DimConstraint,
    GuiBlockSeq,
    GuiBlock,
    GuiGlobalContext,
    SizedGuiBlock,
    SizedGuiBlockSeq,
    GuiVisitorTarget,
    GuiVisitor,
    DimParentSets,
};
use std::iter::repeat;


pub fn before_after<'a, W, H, B, M, A>(
    before: B,
    middle: M,
    after: A,
) -> impl GuiBlock<'a, W, H>
where
    W: DimConstraint,
    H: DimConstraint,
    B: GuiBlockSeq<'a, DimParentSets, DimParentSets>,
    M: GuiBlock<'a, W, H>,
    A: GuiBlockSeq<'a, DimParentSets, DimParentSets>,
{
    BeforeAfter {
        before,
        middle,
        after,
    }
}

#[derive(Debug)]
struct BeforeAfter<B, M, A> {
    before: B,
    middle: M,
    after: A,
}

impl<
    'a,
    W: DimConstraint,
    H: DimConstraint,
    B: GuiBlockSeq<'a, DimParentSets, DimParentSets>,
    M: GuiBlock<'a, W, H>,
    A: GuiBlockSeq<'a, DimParentSets, DimParentSets>,
> GuiBlock<'a, W, H> for BeforeAfter<B, M, A> {
    type Sized = BeforeAfter<B::SizedSeq, M::Sized, A::SizedSeq>;

    fn size(
        self,
        ctx: &GuiGlobalContext<'a>,
        w_in: W::In,
        h_in: H::In,
        scale: f32,
    ) -> (W::Out, H::Out, Self::Sized)
    {
        let (w_out, h_out, middle) = self.middle.size(ctx, w_in, h_in, scale);
        let w = W::get(w_in, w_out);
        let h = H::get(h_in, h_out);
        let (_, _, before) = self.before.size_all(ctx, repeat(w), repeat(h), repeat(scale));
        let (_, _, after) = self.after.size_all(ctx, repeat(w), repeat(h), repeat(scale));
        (w_out, h_out, BeforeAfter {
            before,
            middle,
            after,
        })
    }
}

impl<
    'a,
    B: SizedGuiBlockSeq<'a>,
    M: SizedGuiBlock<'a>,
    A: SizedGuiBlockSeq<'a>,
> SizedGuiBlock<'a> for BeforeAfter<B, M, A> {
    fn visit_nodes<T: GuiVisitorTarget<'a>>(
        self,
        visitor: &mut GuiVisitor<'a, '_, T>,
        forward: bool,
    ) {
        if forward {
            self.before.visit_items_nodes(visitor, IdentityMaperator, true);
            self.middle.visit_nodes(visitor, true);
            self.after.visit_items_nodes(visitor, IdentityMaperator, true);
        } else {
            self.after.visit_items_nodes(visitor, IdentityMaperator, false);
            self.middle.visit_nodes(visitor, false);
            self.before.visit_items_nodes(visitor, IdentityMaperator, false);
        }
    }
}
