
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
    HAlign::<_, true> {
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


/// Gui block that aligns its child's top-left corner.
pub fn align_start<
    'a,
    A: Into<Vec2<f32>>,
    I: GuiBlock<'a, DimChildSets, DimChildSets>,
>(frac: A, inner: I) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
    let Vec2 { x, y } = frac.into();
    h_align_start(x,
        v_align_start(y,
            inner,
        )
    )
}


/// Gui block that aligns its child's left edge.
pub fn h_align_start<
    'a,
    H: DimConstraint,
    I: GuiBlock<'a, DimChildSets, H>,
>(frac: f32, inner: I) -> impl GuiBlock<'a, DimParentSets, H> {
    HAlign::<_, false> {
        frac,
        inner,
    }
}


/// Gui block that aligns its child's top edge.
pub fn v_align_start<
    'a,
    W: DimConstraint,
    I: GuiBlock<'a, W, DimChildSets>,
>(frac: f32, inner: I) -> impl GuiBlock<'a, W, DimParentSets> {
    axis_swap(h_align_start(frac, axis_swap(inner)))
}


#[derive(Debug)]
struct HAlign<I, const ALIGN_CENTER: bool> {
    frac: f32,
    inner: I,
}

impl<
    'a,
    H: DimConstraint,
    I: GuiBlock<'a, DimChildSets, H>,
    const ALIGN_CENTER: bool,
> GuiBlock<'a, DimParentSets, H> for HAlign<I, ALIGN_CENTER> {
    type Sized = HAlignSized<I::Sized>;

    fn size(self, ctx: &GuiGlobalContext<'a>, w: f32, h_in: H::In, scale: f32) -> ((), H::Out, Self::Sized) {
        let (inner_w, h_out, inner_sized) = self.inner.size(ctx, (), h_in, scale);
        let sized = HAlignSized {
            x_translate: if ALIGN_CENTER { w - inner_w } else { w } * self.frac,
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
