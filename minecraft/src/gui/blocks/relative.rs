
use crate::gui::{
    blocks::identity_maperator::IdentityMaperator,
    DimConstraint,
    GuiBlockSeq,
    DimChildSets,
    GuiBlock,
    GuiGlobalContext,
    SizedGuiBlock,
    SizedGuiBlockSeq,
    GuiVisitorTarget,
    GuiVisitor,
    DebugHack,
};
use std::{
    iter::repeat,
    fmt::{self, Formatter, Debug},
};


pub fn relative<'a, W, H, B, I, A>(
    before: B,
    item: I,
    after: A,
) -> impl GuiBlock<'a, W, H>
where
    W: DimConstraint,
    H: DimConstraint,
    B: GuiBlockSeq<'a, DimChildSets, DimChildSets>,
    I: GuiBlock<'a, W, H>,
    A: GuiBlockSeq<'a, DimChildSets, DimChildSets>,
    for<'d> DebugHack<'d, B>: Debug,
    for<'d> DebugHack<'d, A>: Debug,
    for<'d> DebugHack<'d, B::SizedSeq>: Debug,
    for<'d> DebugHack<'d, A::SizedSeq>: Debug,
{
    Relative {
        before,
        item,
        after,
    }
}


struct Relative<B, I, A> {
    before: B,
    item: I,
    after: A,
}

impl<
    'a,
    W: DimConstraint,
    H: DimConstraint,
    B: GuiBlockSeq<'a, DimChildSets, DimChildSets>,
    I: GuiBlock<'a, W, H>,
    A: GuiBlockSeq<'a, DimChildSets, DimChildSets>,
> GuiBlock<'a, W, H> for Relative<B, I, A>
where
    for<'d> DebugHack<'d, B>: Debug,
    for<'d> DebugHack<'d, A>: Debug,
    for<'d> DebugHack<'d, B::SizedSeq>: Debug,
    for<'d> DebugHack<'d, A::SizedSeq>: Debug,
{
    type Sized = Relative<B::SizedSeq, I::Sized, A::SizedSeq>;

    fn size(
        self,
        ctx: &GuiGlobalContext<'a>,
        w_in: W::In,
        h_in: H::In,
        scale: f32,
    ) -> (W::Out, H::Out, Self::Sized)
    {
        let (
            w_out,
            h_out,
            item,
        ) = self.item.size(ctx, w_in, h_in, scale);
        let (
            _,
            _,
            before,
        ) = self.before.size_all(ctx, repeat(()), repeat(()), repeat(scale));
        let (
            _,
            _,
            after,
        ) = self.after.size_all(ctx, repeat(()), repeat(()), repeat(scale));
        (w_out, h_out, Relative {
            before,
            item,
            after,
        })
    }
}

impl<
    'a,
    B: SizedGuiBlockSeq<'a>,
    I: SizedGuiBlock<'a>,
    A: SizedGuiBlockSeq<'a>,
> SizedGuiBlock<'a> for Relative<B, I, A>
where
    for<'d> DebugHack<'d, B>: Debug,
    for<'d> DebugHack<'d, A>: Debug,
{
    fn visit_nodes<T: GuiVisitorTarget<'a>>(
        self,
        visitor: &mut GuiVisitor<'a, '_, T>,
        forward: bool,
    ) {
        if forward {
            self.before.visit_items_nodes(visitor, IdentityMaperator, true);
            self.item.visit_nodes(visitor, true);
            self.after.visit_items_nodes(visitor, IdentityMaperator, true);
        } else {
            self.after.visit_items_nodes(visitor, IdentityMaperator, false);
            self.item.visit_nodes(visitor, false);
            self.before.visit_items_nodes(visitor, IdentityMaperator, false);
        }
    }
}

impl<B, I: Debug, A> Debug for Relative<B, I, A>
where
    for<'d> DebugHack<'d, B>: Debug,
    for<'d> DebugHack<'d, A>: Debug,
{
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Relative")
            .field("before", &DebugHack(&self.before))
            .field("item", &self.item)
            .field("after", &DebugHack(&self.after))
            .finish()
    }
}
