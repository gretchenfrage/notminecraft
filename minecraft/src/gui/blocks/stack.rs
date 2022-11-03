
use crate::gui::{
    GuiVisitor,
    GuiVisitorTarget,
    GuiGlobalContext,
    GuiVisitorMaperator,
    SizedGuiBlockFlatten,
    DimParentSets,
    DimChildSets,
    GuiBlock,
    GuiBlockSeq,
};
use super::{
    axis_swap,
    axis_swap_seq,
};
use std::{
    iter::repeat,
    ops::Index,
    fmt::Debug,
};


/// Gui block that arranges a sequence of blocks in a vertical stack with a
/// consistent pre-scale gap size.
pub fn v_stack<'a, I: GuiBlockSeq<'a, DimParentSets, DimChildSets>>(
    logical_gap: f32,
    items: I,
) -> impl GuiBlock<'a, DimParentSets, DimChildSets> {
    VStack {
        logical_gap,
        items,
    }
}

/// Gui block that arranges a sequence of blocks in a horizontal stack with a
/// consistent pre-scale gap size.
pub fn h_stack<'a, I: GuiBlockSeq<'a, DimChildSets, DimParentSets>>(
    logical_gap: f32,
    items: I,
) -> impl GuiBlock<'a, DimChildSets, DimParentSets> {
    axis_swap(v_stack(logical_gap, axis_swap_seq(items)))
}


#[derive(Debug)]
struct VStack<I> {
    logical_gap: f32,
    items: I,
}

impl<
    'a,
    I: GuiBlockSeq<'a, DimParentSets, DimChildSets>,
> GuiBlock<'a, DimParentSets, DimChildSets> for VStack<I>
{
    type Sized = SizedGuiBlockFlatten<
        I::SizedSeq,
        VStackMaperator<I::HOutSeq>,
    >;

    fn size(
        self,
        ctx: &GuiGlobalContext<'a>,
        w: f32,
        (): (),
        scale: f32,
    ) -> ((), f32, Self::Sized)
    {
        let len = self.items.len();

        let scaled_gap = self.logical_gap * scale;

        let w_in_seq = repeat(w);
        let h_in_seq = repeat(());
        let scale_seq = repeat(scale);

        let (
            _,
            item_heights,
            sized_seq,
        ) = self.items.size_all(ctx, w_in_seq, h_in_seq, scale_seq);

        let mut height = 0.0;
        for i in 0..len {
            if i > 0 {
                height += scaled_gap;
            }
            height += item_heights[i];
        }
        
        let maperator = VStackMaperator {
            item_heights,
            scaled_gap,
            next_idx: 0,
            next_y_translate: 0.0,
        };

        ((), height, SizedGuiBlockFlatten(sized_seq, maperator))
    }
}

#[derive(Debug)]
struct VStackMaperator<H> {
    item_heights: H,
    scaled_gap: f32,
    next_idx: usize,
    next_y_translate: f32,
}

impl<
    'a,
    H: Index<usize, Output=f32> + Debug,
> GuiVisitorMaperator<'a> for VStackMaperator<H>
{
    fn next<'b, T: GuiVisitorTarget<'a>>(
        &'b mut self,
        visitor: &'b mut GuiVisitor<'a, '_, T>,
    ) -> GuiVisitor<'a, 'b, T>
    {
        let y_translate = self.next_y_translate;

        self.next_y_translate += self.item_heights[self.next_idx];
        self.next_y_translate += self.scaled_gap;
        self.next_idx += 1;

        visitor.reborrow()
            .translate([0.0, y_translate])
    }
}
