
use crate::gui::{
    DimConstraint,
    DimParentSets,
    DimChildSets,
    GuiBlock,
    GuiGlobalContext,
};
use super::axis_swap;
use vek::*;


/// Gui block that keeps its child's width at a stable pre-scale size. Passes
/// through the height.
pub fn logical_width<
    'a,
    H: DimConstraint,
    I: GuiBlock<'a, DimParentSets, H>,
>(logical_width: f32, inner: I) -> impl GuiBlock<'a, DimChildSets, H>
{
    LogicalWidth {
        logical_width,
        inner,
    }
}

/// Gui block that keeps its child's height at a stable pre-scale size. Passes
/// through the width.
pub fn logical_height<
    'a,
    W: DimConstraint,
    I: GuiBlock<'a, W, DimParentSets>,
>(logical_height: f32, inner: I) -> impl GuiBlock<'a, W, DimChildSets>
{
    axis_swap(logical_width(logical_height, axis_swap(inner)))
}


pub fn logical_size<
    'a,
    S: Into<Extent2<f32>>,
    I: GuiBlock<'a, DimParentSets, DimParentSets>,
>(logical_size: S, inner: I) -> impl GuiBlock<'a, DimChildSets, DimChildSets>
{
    let size = logical_size.into();
    logical_width(size.w, logical_height(size.h, inner))
}


#[derive(Debug)]
struct LogicalWidth<I> {
    logical_width: f32,
    inner: I,
}

impl<
    'a,
    H: DimConstraint,
    I: GuiBlock<'a, DimParentSets, H>,
> GuiBlock<'a, DimChildSets, H> for LogicalWidth<I> {
    type Sized = I::Sized;

    fn size(
        self,
        ctx: &GuiGlobalContext,
        (): (),
        h_in: H::In,
        scale: f32,
    ) -> (f32, H::Out, Self::Sized) {
        let w = self.logical_width * scale;
        let ((), h_out, sized) = self.inner.size(ctx, w, h_in, scale);
        (w, h_out, sized)
    }        
}
