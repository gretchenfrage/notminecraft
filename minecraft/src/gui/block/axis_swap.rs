
use graphics::modifier::Transform2;
use crate::gui::{
    GuiVisitor,
    GuiVisitorTarget,
    block::{
        DimConstraint,
        GuiBlock,
        SizedGuiBlock,
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
