use super::*;
    use std::iter::repeat;

    pub fn v_stack_gui_block<'a, I: GuiBlockSeq<'a, DimParentSets, DimChildSets>>(unscaled_gap: f32, items: I) -> impl GuiBlock<'a, DimParentSets, DimChildSets> {
        VStackGuiBlock {
            unscaled_gap,
            items,
        }
    }

    struct VStackGuiBlock<I> {
        unscaled_gap: f32,
        items: I,
    }

    impl<'a, I: GuiBlockSeq<'a, DimParentSets, DimChildSets>> GuiBlock<'a, DimParentSets, DimChildSets> for VStackGuiBlock<I> {
        type Sized = SubmapIterSizedGuiBlock<VStackItemVisitorMapper<I::HOutSeq>, I::SizedSeq>;

        fn size(self, w: f32, (): (), scale: f32) -> ((), f32, Self::Sized) {
            let len = self.items.len();

            let gap = self.unscaled_gap * scale;

            let w_in_seq = repeat(w);
            let h_in_seq = repeat(());
            let scale_seq = repeat(scale);

            let (_, item_heights, sized_seq) = self.items.size_all(w_in_seq, h_in_seq, scale_seq);

            let mut height = 0.0;
            for i in 0..len {
                if i > 0 {
                    height += gap;
                }
                height += item_heights[i];
            }
            
            let sized = SubmapIterSizedGuiBlock::new(
                VStackItemVisitorMapper {
                    item_heights,
                    gap,
                    next_idx: 0,
                    next_y_translate: 0.0,
                },
                sized_seq,
            );

            ((), height, sized)
        }
    }

    struct VStackItemVisitorMapper<H> {
        item_heights: H,
        gap: f32,
        next_idx: usize,
        next_y_translate: f32,
    }

    impl<H: Index<usize, Output=f32>> GuiVisitorSubmapIterMapper for VStackItemVisitorMapper<H> {
        fn map_next<'a, 'b, T: GuiVisitorTarget<'a>>(&'b mut self, visitor: GuiVisitor<'b, T>) -> GuiVisitor<'b, T> {
            let visitor = visitor
                .translate([0.0, self.next_y_translate]);

            self.next_y_translate += self.item_heights[self.next_idx];
            self.next_y_translate += self.gap;

            self.next_idx += 1;

            visitor
        }
    }