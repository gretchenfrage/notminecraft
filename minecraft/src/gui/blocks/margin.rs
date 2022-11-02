
use crate::gui::{
    GuiVisitor,
    GuiVisitorTarget,
    DimConstraint,
    DimParentSets,
    GuiBlock,
    SizedGuiBlock,
    GuiGlobalContext,
};
use super::axis_swap;


/// Gui block with a parent-set size that puts margins around its left, right,
/// top, and bottom sides (of constant pre-scale size), setting the child's
/// size to smaller values.
pub fn margin<'a, I: GuiBlock<'a, DimParentSets, DimParentSets>>(
    logical_margin_left: f32,
    logical_margin_right: f32,
    logical_margin_top: f32,
    logical_margin_bottom: f32,
    inner: I,
) -> impl GuiBlock<'a, DimParentSets, DimParentSets>
{
    h_margin(logical_margin_left, logical_margin_right,
        v_margin(logical_margin_top, logical_margin_bottom,
            inner,
        )
    )
}


/// Gui block with a parent-set width that puts a left and right margin around
/// its child (of constant pre-scale size), setting the child's width to a
/// smaller value. Passes through the height constraint.
pub fn h_margin<'a, H: DimConstraint, I: GuiBlock<'a, DimParentSets, H>>(
    logical_margin_left: f32,
    logical_margin_right: f32,
    inner: I,
) -> impl GuiBlock<'a, DimParentSets, H> {
    HMargin {
        logical_margin_left,
        logical_margin_right,
        inner,
    }
}


/// Gui block with a parent-set height that puts a top and bottom margin around
/// its child (of constant pre-scale size), setting the child's height to a
/// smaller value. Passes through the width constraint.
pub fn v_margin<'a, W: DimConstraint, I: GuiBlock<'a, W, DimParentSets>>(
    logical_margin_top: f32,
    logical_margin_bottom: f32,
    inner: I,
) -> impl GuiBlock<'a, W, DimParentSets> {
    axis_swap(
        h_margin(
            logical_margin_top,
            logical_margin_bottom,
            axis_swap(inner),
        ),
    )
}


struct HMargin<I> {
    logical_margin_left: f32,
    logical_margin_right: f32,
    inner: I,
}

impl<
    'a,
    H: DimConstraint,
    I: GuiBlock<'a, DimParentSets, H>,
> GuiBlock<'a, DimParentSets, H> for HMargin<I> {
    type Sized = HMarginSized<I::Sized>;

    fn size(
        self,
        ctx: &GuiGlobalContext,
        w: f32,
        h_in: H::In,
        scale: f32,
    ) -> ((), H::Out, Self::Sized) {
        let margin_min = self.logical_margin_left * scale;
        let margin_max = self.logical_margin_right * scale;

        let inner_w = f32::max(w - margin_min - margin_max, 0.0);
        let x_translate = (w - inner_w) / 2.0;

        let (
            (),
            h_out,
            inner_sized,
        ) = self.inner.size(ctx, inner_w, h_in, scale);

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
    fn visit_nodes<T: GuiVisitorTarget<'a>>(
        self,
        visitor: &mut GuiVisitor<'_, T>,
    ) {
        self.inner.visit_nodes(&mut visitor.reborrow()
            .translate([self.x_translate, 0.0]));
    }
}
