
use crate::gui::{
    GuiVisitor,
    GuiVisitorTarget,
    GuiGlobalContext,
    GuiVisitorMaperator,
    DirMaperators,
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
/// consistent ~~pre-scale~~ gap size.
pub fn v_stack<'a, I: GuiBlockSeq<'a, DimParentSets, DimChildSets>>(
    gap: f32,
    items: I,
) -> impl GuiBlock<'a, DimParentSets, DimChildSets> {
    VStack {
        gap,
        items,
    }
}

/// Gui block that arranges a sequence of blocks in a horizontal stack with a
/// consistent ~~pre-scale~~ gap size.
pub fn h_stack<'a, I: GuiBlockSeq<'a, DimChildSets, DimParentSets>>(
    gap: f32,
    items: I,
) -> impl GuiBlock<'a, DimChildSets, DimParentSets> {
    axis_swap(v_stack(gap, axis_swap_seq(items)))
}


#[derive(Debug)]
struct VStack<I> {
    gap: f32,
    items: I,
}

impl<
    'a,
    I: GuiBlockSeq<'a, DimParentSets, DimChildSets>,
> GuiBlock<'a, DimParentSets, DimChildSets> for VStack<I>
{
    type Sized = SizedGuiBlockFlatten<
        I::SizedSeq,
        VStackDirMaperators<I::HOutSeq>,
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
                height += self.gap;
            }
            height += item_heights[i];
        }
        
        let maperator = VStackDirMaperators {
            item_heights,
            gap: self.gap,
            len,
        };

        ((), height, SizedGuiBlockFlatten(sized_seq, maperator))
    }
}


#[derive(Debug)]
struct VStackDirMaperators<H> {
    item_heights: H,
    gap: f32,
    len: usize
}

impl<
    'a,
    H: Index<usize, Output=f32> + Debug,
> DirMaperators<'a> for VStackDirMaperators<H> {
    type Forward = VStackMaperatorForward<H>;
    type Reverse = VStackMaperatorReverse<H>;

    fn forward(self) -> Self::Forward {
        VStackMaperatorForward {
            item_heights: self.item_heights,
            gap: self.gap,
            next_y_translate: 0.0,
            next_idx: 0,
        }
    }

    fn reverse(self) -> Self::Reverse {
        let mut next_y_translate = 0.0;
        let mut next_idx = 0;

        while next_idx < self.len {
            next_y_translate += self.item_heights[next_idx];
            next_y_translate += self.gap;
            next_idx += 1;
        }

        VStackMaperatorReverse {
            item_heights: self.item_heights,
            gap: self.gap,
            prev_idx: next_idx,
            prev_y_translate: next_y_translate,
        }
    }
}


#[derive(Debug)]
struct VStackMaperatorForward<H> {
    item_heights: H,
    gap: f32,
    next_y_translate: f32,
    next_idx: usize,
}

impl<
    'a,
    H: Index<usize, Output=f32> + Debug,
> GuiVisitorMaperator<'a> for VStackMaperatorForward<H>
{
    fn next<'b, T: GuiVisitorTarget<'a>>(
        &'b mut self,
        visitor: &'b mut GuiVisitor<'a, '_, T>,
    ) -> GuiVisitor<'a, 'b, T>
    {
        let y_translate = self.next_y_translate;

        self.next_y_translate += self.item_heights[self.next_idx];
        self.next_y_translate += self.gap;
        self.next_idx += 1;

        visitor.reborrow()
            .translate([0.0, y_translate])
    }
}


#[derive(Debug)]
struct VStackMaperatorReverse<H> {
    item_heights: H,
    gap: f32,
    prev_idx: usize,
    prev_y_translate: f32,
}

impl<
    'a,
    H: Index<usize, Output=f32> + Debug,
> GuiVisitorMaperator<'a> for VStackMaperatorReverse<H>
{
    fn next<'b, T: GuiVisitorTarget<'a>>(
        &'b mut self,
        visitor: &'b mut GuiVisitor<'a, '_, T>,
    ) -> GuiVisitor<'a, 'b, T>
    {
        self.prev_idx -= 1;
        self.prev_y_translate -= self.gap;
        self.prev_y_translate -= self.item_heights[self.prev_idx];

        visitor.reborrow()
            .translate([0.0, self.prev_y_translate])
    }
}
