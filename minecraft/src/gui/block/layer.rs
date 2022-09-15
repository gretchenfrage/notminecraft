
use crate::gui::{
    GuiVisitor,
    GuiVisitorTarget,
    block::{
        DimParentSets,
        GuiBlock,
        GuiBlockSeq,
        SubmapIterSizedGuiBlock,
        GuiVisitorSubmapIterMapper,
    },
};
use std::iter::repeat;


/// Gui block that superimposes its children over each other.
pub fn layer<'a, I: GuiBlockSeq<'a, DimParentSets, DimParentSets>>(items: I) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
    Layer { items }
}

struct Layer<I> {
    items: I,
}

impl<'a, I: GuiBlockSeq<'a, DimParentSets, DimParentSets>> GuiBlock<'a, DimParentSets, DimParentSets> for Layer<I> {
    type Sized = SubmapIterSizedGuiBlock<LayerItemVisitorMapper, I::SizedSeq>;

    fn size(self, w: f32, h: f32, scale: f32) -> ((), (), Self::Sized) {
        let w_in_seq = repeat(w);
        let h_in_seq = repeat(h);
        let scale_seq = repeat(scale);

        let (_, _, sized_seq) = self.items.size_all(w_in_seq, h_in_seq, scale_seq);

        let sized = SubmapIterSizedGuiBlock::new(LayerItemVisitorMapper, sized_seq);

        ((), (), sized)
    }
}

struct LayerItemVisitorMapper;

impl GuiVisitorSubmapIterMapper for LayerItemVisitorMapper {
    fn map_next<'a, 'b, T: GuiVisitorTarget<'a>>(&'b mut self, visitor: GuiVisitor<'b, T>) -> GuiVisitor<'b, T> {
        visitor
    }
}