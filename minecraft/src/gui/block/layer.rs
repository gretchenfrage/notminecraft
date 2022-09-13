use super::*;
    use std::iter::repeat;

    pub fn layer_gui_block<'a, I: GuiBlockSeq<'a, DimParentSets, DimParentSets>>(items: I) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
        LayerGuiBlock { items }
    }

    struct LayerGuiBlock<I> {
        items: I,
    }

    impl<'a, I: GuiBlockSeq<'a, DimParentSets, DimParentSets>> GuiBlock<'a, DimParentSets, DimParentSets> for LayerGuiBlock<I> {
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