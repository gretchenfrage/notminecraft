
use graphics::modifier::Transform2;
use crate::gui::{
    GuiVisitor,
    GuiVisitorTarget,
    DimConstraint,
    GuiBlock,
    SizedGuiBlock,
    GuiBlockSeq,
    SizedGuiBlockSeq,
    GuiVisitorMaperator,
    GuiGlobalContext,
};
use vek::*;


/// Gui block that swaps the x and y axis of its child, mirroring it across the
/// down-right diagonal and swapping the `W`/`H` dimensional constraint types.
pub fn axis_swap<
    'a,
    W: DimConstraint,
    H: DimConstraint,
    I: GuiBlock<'a, W, H>,
>(inner: I) -> impl GuiBlock<'a, H, W> {
    AxisSwap(inner)
}

/// Like `axis_swap` but for each element of a `GuiBlockSeq`.
pub fn axis_swap_seq<
    'a,
    W: DimConstraint,
    H: DimConstraint,
    I: GuiBlockSeq<'a, W, H>,
>(inner: I) -> impl GuiBlockSeq<'a, H, W> {
    AxisSwap(inner)
}


#[derive(Debug)]
struct AxisSwap<I>(I);


impl<
    'a,
    W: DimConstraint,
    H: DimConstraint,
    I: GuiBlock<'a, H, W>,
> GuiBlock<'a, W, H> for AxisSwap<I> {
    type Sized = AxisSwap<I::Sized>;

    fn size(
        self,
        ctx: &GuiGlobalContext<'a>,
        w_in: W::In,
        h_in: H::In,
        scale: f32,
    ) -> (W::Out, H::Out, Self::Sized)
    {
        let (
            inner_w_out,
            inner_h_out,
            inner_sized,
        ) = self.0.size(ctx, h_in, w_in, scale);
        (inner_h_out, inner_w_out, AxisSwap(inner_sized))
    }
}

impl<'a, I: SizedGuiBlock<'a>> SizedGuiBlock<'a> for AxisSwap<I> {
    fn visit_nodes<T>(self, visitor: &mut GuiVisitor<'a, '_, T>, forward: bool)
    where
        T: GuiVisitorTarget<'a>,
    {
        let mut visitor = visitor.reborrow()
            .debug_tag("axis_swap")
            .modify(Transform2(Mat3::new(
                0.0, 1.0, 0.0,
                1.0, 0.0, 0.0,
                0.0, 0.0, 1.0,
            )));
        self.0.visit_nodes(&mut visitor, forward);
    }
}


#[derive(Debug)]
struct AxisSwap<I>(I);


impl<
    'a,
    W: DimConstraint,
    H: DimConstraint,
    I: GuiBlockSeq<'a, H, W>,
> GuiBlockSeq<'a, W, H> for AxisSwap<I> {
    type SizedSeq = AxisSwap<I::SizedSeq>;
    type WOutSeq = I::HOutSeq;
    type HOutSeq = I::WOutSeq;

    fn len(&self) -> usize {
        self.0.len()
    }

    fn size_all<
        WInSeq: IntoIterator<Item=W::In>,
        HInSeq: IntoIterator<Item=H::In>,
        ScaleSeq: IntoIterator<Item=f32>,
    >(
        self,
        ctx: &GuiGlobalContext<'a>,
        w_in_seq: WInSeq,
        h_in_seq: HInSeq,
        scale_seq: ScaleSeq,
    ) -> (Self::WOutSeq, Self::HOutSeq, Self::SizedSeq) {
        let (
            inner_w_out_seq,
            inner_h_out_seq,
            inner_sized_seq,
        ) = self.0.size_all(ctx, h_in_seq, w_in_seq, scale_seq);
        (inner_h_out_seq, inner_w_out_seq, AxisSwap(inner_sized_seq))
    }
}

impl<'a, I: SizedGuiBlockSeq<'a>> SizedGuiBlockSeq<'a> for AxisSwap<I> {
    fn visit_items_nodes<T, M>(
        self,
        visitor: &mut GuiVisitor<'a, '_, T>,
        maperator: M,
        forward: bool,
    )
    where
        T: GuiVisitorTarget<'a>,
        M: GuiVisitorMaperator<'a>,
    {
        self.0
            .visit_items_nodes(visitor, AxisSwap(maperator), forward)
    }
}

impl<'a, I: GuiVisitorMaperator<'a>> GuiVisitorMaperator<'a> for AxisSwap<I> {
    fn next<'b, T: GuiVisitorTarget<'a>>(
        &'b mut self,
        visitor: &'b mut GuiVisitor<'a, '_, T>,
    ) -> GuiVisitor<'a, 'b, T>
    {
        self.0.next(visitor)
            .modify(Transform2(Mat3::new(
                0.0, 1.0, 0.0,
                1.0, 0.0, 0.0,
                0.0, 0.0, 1.0,
            )))
    }
}
