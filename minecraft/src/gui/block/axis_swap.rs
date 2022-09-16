
use graphics::modifier::Transform2;
use crate::gui::{
    GuiVisitor,
    GuiVisitorTarget,
    block::{
        DimConstraint,
        GuiBlock,
        SizedGuiBlock,
        GuiBlockSeq,
        SizedGuiBlockSeq,
        GuiVisitorIter,
    },
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
    AxisSwap { inner }
}

/// Sequence equivalent of `axis_swap`.
pub fn axis_swap_seq<
    'a,
    W: DimConstraint,
    H: DimConstraint,
    I: GuiBlockSeq<'a, W, H>,
>(inner: I) -> impl GuiBlockSeq<'a, H, W> {
    AxisSwap { inner }
}


struct AxisSwap<I> {
    inner: I,
}

impl<
    'a,
    W: DimConstraint,
    H: DimConstraint,
    I: GuiBlock<'a, H, W>,
> GuiBlock<'a, W, H> for AxisSwap<I> {
    type Sized = AxisSwap<I::Sized>;

    fn size(
        self,
        w_in: W::In,
        h_in: H::In,
        scale: f32,
    ) -> (W::Out, H::Out, Self::Sized)
    {
        let (
            inner_w_out,
            inner_h_out,
            inner_sized,
        ) = self.inner.size(h_in, w_in, scale);
        let sized = AxisSwap { inner: inner_sized };
        (inner_h_out, inner_w_out, sized)
    }
}

impl<'a, I: SizedGuiBlock<'a>> SizedGuiBlock<'a> for AxisSwap<I> {
    fn visit_nodes<T>(self, mut visitor: GuiVisitor<'_, T>)
    where
        T: GuiVisitorTarget<'a>,
    {
        self.inner.visit_nodes(visitor.reborrow()
            .modify(Transform2(Mat3::new(
                0.0, 1.0, 0.0,
                1.0, 0.0, 0.0,
                0.0, 0.0, 1.0,
            ))));
    }
}

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
        self.inner.len()
    }

    fn size_all<
        WInSeq: IntoIterator<Item=W::In>,
        HInSeq: IntoIterator<Item=H::In>,
        ScaleSeq: IntoIterator<Item=f32>,
    >(
        self,
        w_in_seq: WInSeq,
        h_in_seq: HInSeq,
        scale_seq: ScaleSeq,
    ) -> (Self::WOutSeq, Self::HOutSeq, Self::SizedSeq) {
        let (
            inner_w_out_seq,
            inner_h_out_seq,
            inner_sized_seq,
        ) = self.inner.size_all(h_in_seq, w_in_seq, scale_seq);
        let sized = AxisSwap { inner: inner_sized_seq };
        (inner_h_out_seq, inner_w_out_seq, sized)
    }
}

impl<'a, I: SizedGuiBlockSeq<'a>> SizedGuiBlockSeq<'a> for AxisSwap<I> {
    fn visit_items_nodes<It: GuiVisitorIter<'a>>(self, visitors: It) {
        self.inner
            .visit_items_nodes(AxisSwapGuiVisitorIter {
                inner: visitors,
            })
    }
}

pub struct AxisSwapGuiVisitorIter<I> {
    inner: I,
}

impl<'a, I: GuiVisitorIter<'a>> GuiVisitorIter<'a> for AxisSwapGuiVisitorIter<I> {
    type Target = I::Target;

    fn next<'b>(&'b mut self) -> GuiVisitor<'b, Self::Target> {
        self.inner
            .next()
            .modify(Transform2(Mat3::new(
                0.0, 1.0, 0.0,
                1.0, 0.0, 0.0,
                0.0, 0.0, 1.0,
            )))
    }
}
