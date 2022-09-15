
use crate::gui::block::{
    axis_swap,
    DimConstraint,
    DimParentSets,
    DimChildSets,
    GuiBlock,
};


/// Gui block that keeps its child's width at a stable pre-scale size. Passes
/// through the height.
pub fn h_stable_unscaled_size<
    'a,
    H: DimConstraint,
    I: GuiBlock<'a, DimParentSets, H>,
>(unscaled_dim_size: f32, inner: I) -> impl GuiBlock<'a, DimChildSets, H>
{
    HStableUnscaledSize {
        unscaled_dim_size,
        inner,
    }
}

/// Gui block that keeps its child's height at a stable pre-scale size. Passes
/// through the width.
pub fn v_stable_unscaled_size<
    'a,
    W: DimConstraint,
    I: GuiBlock<'a, W, DimParentSets>,
>(unscaled_dim_size: f32, inner: I) -> impl GuiBlock<'a, W, DimChildSets>
{
    axis_swap(h_stable_unscaled_size(unscaled_dim_size, axis_swap(inner)))
}


struct HStableUnscaledSize<I> {
    unscaled_dim_size: f32,
    inner: I,
}

impl<
    'a,
    H: DimConstraint,
    I: GuiBlock<'a, DimParentSets, H>,
> GuiBlock<'a, DimChildSets, H> for HStableUnscaledSize<I> {
    type Sized = I::Sized;

    fn size(self, (): (), h_in: H::In, scale: f32) -> (f32, H::Out, Self::Sized) {
        let w = self.unscaled_dim_size * scale;
        let ((), h_out, sized) = self.inner.size(w, h_in, scale);
        (w, h_out, sized)
    }        
}
