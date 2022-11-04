
use crate::gui::{
    GuiVisitor,
    GuiVisitorTarget,
    DimConstraint,
    DimParentSets,
    DimChildSets,
    GuiBlock,
    SizedGuiBlock,
    GuiGlobalContext,
};
use super::axis_swap;
use vek::*;


pub fn align<
    'a,
    A: Into<Vec2<f32>>,
    I: GuiBlock<'a, DimChildSets, DimChildSets>,
>(frac: A, inner: I) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
    let Vec2 { x, y } = frac.into();
    h_align(x,
        v_align(y,
            inner,
        )
    )
}

/// Gui block that horizontally aligns its child.
///
/// `frac` is the fraction through self's horizontal length to put the
/// horizontal center of child. Set to `0.5` to center.
pub fn h_align<
    'a,
    H: DimConstraint,
    I: GuiBlock<'a, DimChildSets, H>,
>(frac: f32, inner: I) -> impl GuiBlock<'a, DimParentSets, H> {
    HAlign {
        frac,
        inner,
    }
}

/// Gui block that vertically aligns its child.
///
/// `frac` is the fraction through self's vertical length to put the
/// vertical center of child. Set to `0.5` to center.
pub fn v_align<
    'a,
    W: DimConstraint,
    I: GuiBlock<'a, W, DimChildSets>,
>(frac: f32, inner: I) -> impl GuiBlock<'a, W, DimParentSets> {
    axis_swap(h_align(frac, axis_swap(inner)))
}


#[derive(Debug)]
struct HAlign<I> {
    frac: f32,
    inner: I,
}

impl<'a, H: DimConstraint, I: GuiBlock<'a, DimChildSets, H>> GuiBlock<'a, DimParentSets, H> for HAlign<I> {
    type Sized = HAlignSized<I::Sized>;

    fn size(self, ctx: &GuiGlobalContext<'a>, w: f32, h_in: H::In, scale: f32) -> ((), H::Out, Self::Sized) {
        let (inner_w, h_out, inner_sized) = self.inner.size(ctx, (), h_in, scale);
        let sized = HAlignSized {
            x_translate: (w - inner_w) * self.frac,
            inner: inner_sized,
        };
        ((), h_out, sized)
    }
}


#[derive(Debug)]
struct HAlignSized<I> {
    x_translate: f32,
    inner: I,
}

impl<'a, I: SizedGuiBlock<'a>> SizedGuiBlock<'a> for HAlignSized<I> {
    fn visit_nodes<T: GuiVisitorTarget<'a>>(
        self,
        visitor: &mut GuiVisitor<'a, '_, T>,
        forward: bool,
    ) {
        let mut visitor = visitor.reborrow()
            .translate([self.x_translate, 0.0]);
        self.inner.visit_nodes(&mut visitor, forward);
    }
}
