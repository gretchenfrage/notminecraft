
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


struct Layer<I>(I);

impl<
    'a,
    I: GuiBlockSeq<'a, DimParentSets, DimParentSets>,
> GuiBlock<'a, DimParentSets, DimParentSets> for Layer<I>
{
    //type Sized = SubmapIterSizedGuiBlock<LayerItemVisitorMapper, I::SizedSeq>;
    type Sized = SizedGuiBlockFlatten<I::SizedSeq, IdentityMaperator>;

    fn size(
        self,
        ctx: &GuiGlobalContext,
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

pub struct IdentityMaperator;

impl<'a> GuiVisitorMaperator<'a> for IdentityMaperator {
    fn next<'b, T: GuiVisitorTarget<'a>>(
        &'b mut self,
        visitor: &'b mut GuiVisitor<T>,
    ) -> GuiVisitor<'b, T>
    {
        visitor.reborrow()
    }
}
