
use crate::gui::{
    GuiVisitor,
    GuiVisitorTarget,
    DimParentSets,
    GuiBlock,
    GuiBlockSeq,
    SizedGuiBlockFlatten,
    GuiVisitorMaperator,
    GuiGlobalContext,
};
use std::iter::repeat;


/// Gui block that superimposes its children over each other.
pub fn layer<
    'a,
    I: GuiBlockSeq<'a, DimParentSets, DimParentSets>,
>(items: I) -> impl GuiBlock<'a, DimParentSets, DimParentSets>
{
    Layer(items)
}


#[derive(Debug)]
struct Layer<I>(I);

impl<
    'a,
    I: GuiBlockSeq<'a, DimParentSets, DimParentSets>,
> GuiBlock<'a, DimParentSets, DimParentSets> for Layer<I>
{
    type Sized = SizedGuiBlockFlatten<I::SizedSeq, IdentityMaperator>;

    fn size(
        self,
        ctx: &GuiGlobalContext<'a>,
        w: f32,
        h: f32,
        scale: f32,
    ) -> ((), (), Self::Sized) {
        let w_in_seq = repeat(w);
        let h_in_seq = repeat(h);
        let scale_seq = repeat(scale);

        let (
            _,
            _,
            sized_seq,
        ) = self.0.size_all(ctx, w_in_seq, h_in_seq, scale_seq);

        let sized = SizedGuiBlockFlatten(sized_seq, IdentityMaperator);

        ((), (), sized)
    }
}

#[derive(Debug)]
pub struct IdentityMaperator;

impl<'a> GuiVisitorMaperator<'a> for IdentityMaperator {
    fn next<'b, T: GuiVisitorTarget<'a>>(
        &'b mut self,
        visitor: &'b mut GuiVisitor<'a, '_, T>,
    ) -> GuiVisitor<'a, 'b, T>
    {
        visitor.reborrow()
            .debug_tag("layer item")
    }
}
