use super::*;

    pub fn h_stable_unscaled_dim_size_gui_block<'a, H: DimConstraint, I: GuiBlock<'a, DimParentSets, H>>(unscaled_dim_size: f32, inner: I) -> impl GuiBlock<'a, DimChildSets, H> {
        HStableUnscaledDimSizeGuiBlock {
            unscaled_dim_size,
            inner,
        }
    }

    struct HStableUnscaledDimSizeGuiBlock<I> {
        unscaled_dim_size: f32,
        inner: I,
    }

    impl<'a, H: DimConstraint, I: GuiBlock<'a, DimParentSets, H>> GuiBlock<'a, DimChildSets, H> for HStableUnscaledDimSizeGuiBlock<I> {
        type Sized = I::Sized;

        fn size(self, (): (), h_in: H::In, scale: f32) -> (f32, H::Out, Self::Sized) {
            let w = self.unscaled_dim_size * scale;
            let ((), h_out, sized) = self.inner.size(w, h_in, scale);
            (w, h_out, sized)
        }        
    }


    // ==== TODO dedupe somehow ====


    pub fn v_stable_unscaled_dim_size_gui_block<'a, W: DimConstraint, I: GuiBlock<'a, W, DimParentSets>>(unscaled_dim_size: f32, inner: I) -> impl GuiBlock<'a, W, DimChildSets> {
        VStableUnscaledDimSizeGuiBlock {
            unscaled_dim_size,
            inner,
        }
    }

    struct VStableUnscaledDimSizeGuiBlock<I> {
        unscaled_dim_size: f32,
        inner: I,
    }

    impl<'a, W: DimConstraint, I: GuiBlock<'a, W, DimParentSets>> GuiBlock<'a, W, DimChildSets> for VStableUnscaledDimSizeGuiBlock<I> {
        type Sized = I::Sized;

        fn size(self, w_in: W::In, (): (), scale: f32) -> (W::Out, f32, Self::Sized) {
            let h = self.unscaled_dim_size * scale;
            let (w_out, (), sized) = self.inner.size(w_in, h, scale);
            (w_out, h, sized)
        }        
    }