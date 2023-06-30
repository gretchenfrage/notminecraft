
use crate::gui::{
    GuiVisitor,
    GuiVisitorTarget,
    GuiGlobalContext,
    GuiVisitorMaperator,
    DirMaperators,
    SizedGuiBlockFlatten,
    DimParentSets,
    DimChildSets,
    DimConstraint,
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


pub fn v_stack_auto<'a, I: GuiBlockSeq<'a, DimChildSets, DimChildSets>>(
    logical_gap: f32,
    items: I,
) -> impl GuiBlock<'a, DimChildSets, DimChildSets> {
    VStack {
        logical_gap,
        items,
    }
}

pub fn h_stack_auto<'a, I: GuiBlockSeq<'a, DimChildSets, DimChildSets>>(
    logical_gap: f32,
    items: I,
) -> impl GuiBlock<'a, DimChildSets, DimChildSets> {
    axis_swap(v_stack_auto(logical_gap, axis_swap_seq(items)))
}


/// For abstracting over:
/// - Stack of elements with parent-set width, that sets them all to the same.
/// - Stack of elements with child-set width, which sets its own width to their
///   max.
trait WidthLogic: DimConstraint {
    fn w_out<Seq>(num_items: usize, item_w_outs: Seq) -> Self::Out
    where
        Seq: Index<usize, Output=Self::Out> + Debug;
}

impl WidthLogic for DimParentSets {
    fn w_out<Seq>(_num_items: usize, _: Seq) -> ()
    where
        Seq: Index<usize, Output=()> + Debug
    {}
}

impl WidthLogic for DimChildSets {
    fn w_out<Seq>(num_items: usize, item_widths: Seq) -> f32
    where
        Seq: Index<usize, Output=f32> + Debug
    {
        (0..num_items)
            .map(|i| item_widths[i])
            .max_by(|a, b| a.total_cmp(&b))
            .unwrap_or(0.0)
    }
}


#[derive(Debug)]
struct VStack<I> {
    logical_gap: f32,
    items: I,
}

impl<
    'a,
    W: DimConstraint + WidthLogic,
    I: GuiBlockSeq<'a, W, DimChildSets>,
> GuiBlock<'a, W, DimChildSets> for VStack<I>
{
    type Sized = SizedGuiBlockFlatten<
        I::SizedSeq,
        VStackDirMaperators<I::HOutSeq>,
    >;

    fn size(
        self,
        ctx: &GuiGlobalContext<'a>,
        w_in: W::In,
        (): (),
        scale: f32,
    ) -> (W::Out, f32, Self::Sized)
    {
        let len = self.items.len();

        let scaled_gap = self.logical_gap * scale;

        let w_in_seq = repeat(w_in);
        let h_in_seq = repeat(());
        let scale_seq = repeat(scale);

        let (
            item_w_outs,
            item_heights,
            sized_seq,
        ) = self.items.size_all(ctx, w_in_seq, h_in_seq, scale_seq);

        let w_out = W::w_out(len, item_w_outs);

        let mut height = 0.0;
        for i in 0..len {
            if i > 0 {
                height += scaled_gap;
            }
            height += item_heights[i];
        }
        
        let maperator = VStackDirMaperators {
            item_heights,
            scaled_gap,
            len,
        };

        (w_out, height, SizedGuiBlockFlatten(sized_seq, maperator))
    }
}


#[derive(Debug)]
struct VStackDirMaperators<H> {
    item_heights: H,
    scaled_gap: f32,
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
            scaled_gap: self.scaled_gap,
            next_y_translate: 0.0,
            next_idx: 0,
        }
    }

    fn reverse(self) -> Self::Reverse {
        let mut next_y_translate = 0.0;
        let mut next_idx = 0;

        while next_idx < self.len {
            next_y_translate += self.item_heights[next_idx];
            next_y_translate += self.scaled_gap;
            next_idx += 1;
        }

        VStackMaperatorReverse {
            item_heights: self.item_heights,
            scaled_gap: self.scaled_gap,
            prev_idx: next_idx,
            prev_y_translate: next_y_translate,
        }
    }
}


#[derive(Debug)]
struct VStackMaperatorForward<H> {
    item_heights: H,
    scaled_gap: f32,
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
        self.next_y_translate += self.scaled_gap;
        self.next_idx += 1;

        visitor.reborrow()
            .translate([0.0, y_translate])
    }
}


#[derive(Debug)]
struct VStackMaperatorReverse<H> {
    item_heights: H,
    scaled_gap: f32,
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
        self.prev_y_translate -= self.scaled_gap;
        self.prev_y_translate -= self.item_heights[self.prev_idx];

        visitor.reborrow()
            .translate([0.0, self.prev_y_translate])
    }
}
