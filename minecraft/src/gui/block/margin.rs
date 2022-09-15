
use crate::gui::{
    GuiVisitor,
    GuiVisitorTarget,
    block::{
        axis_swap,
        DimConstraint,
        DimParentSets,
        GuiBlock,
        SizedGuiBlock,
    },
};


pub fn h_margin<'a, H: DimConstraint, I: GuiBlock<'a, DimParentSets, H>>(
    unscaled_margin_low: f32,
    unscaled_margin_high: f32,
    inner: I,
) -> impl GuiBlock<'a, DimParentSets, H> {
    HMargin {
        unscaled_margin_low,
        unscaled_margin_high,
        inner,
    }
}


pub fn v_margin<'a, W: DimConstraint, I: GuiBlock<'a, W, DimParentSets>>(
    unscaled_margin_low: f32,
    unscaled_margin_high: f32,
    inner: I,
) -> impl GuiBlock<'a, W, DimParentSets> {
    axis_swap(
        h_margin(
            unscaled_margin_low,
            unscaled_margin_high,
            axis_swap(inner),
        ),
    )
}


struct HMargin<I> {
    unscaled_margin_low: f32,
    unscaled_margin_high: f32,
    inner: I,
}

impl<
    'a,
    H: DimConstraint,
    I: GuiBlock<'a, DimParentSets, H>,
> GuiBlock<'a, DimParentSets, H> for HMargin<I> {
    type Sized = HMarginSized<I::Sized>;

    fn size(self, w: f32, h_in: H::In, scale: f32) -> ((), H::Out, Self::Sized) {
        let margin_min = self.unscaled_margin_low * scale;
        let margin_max = self.unscaled_margin_high * scale;

        let inner_w = f32::max(w - margin_min - margin_max, 0.0);
        let x_translate = (w - inner_w) / 2.0;

        let ((), h_out, inner_sized) = self.inner.size(inner_w, h_in, scale);

        let sized = HMarginSized {
            x_translate,
            inner: inner_sized,
        };

        ((), h_out, sized)
    }
}

struct HMarginSized<I> {
    x_translate: f32,
    inner: I,
}

impl<'a, I: SizedGuiBlock<'a>> SizedGuiBlock<'a> for HMarginSized<I> {
    fn visit_nodes<T: GuiVisitorTarget<'a>>(self, mut visitor: GuiVisitor<'_, T>) {
        self.inner.visit_nodes(visitor.reborrow()
            .translate([self.x_translate, 0.0]));
    }
}