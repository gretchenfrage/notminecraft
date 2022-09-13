use super::*;

    pub fn h_margin_gui_block<'a, H: DimConstraint, I: GuiBlock<'a, DimParentSets, H>>(unscaled_margin_min: f32, unscaled_margin_max: f32, inner: I) -> impl GuiBlock<'a, DimParentSets, H> {
        HMarginGuiBlock {
            unscaled_margin_min,
            unscaled_margin_max,
            inner,
        }
    }

    struct HMarginGuiBlock<I> {
        unscaled_margin_min: f32,
        unscaled_margin_max: f32,
        inner: I,
    }

    impl<'a, H: DimConstraint, I: GuiBlock<'a, DimParentSets, H>> GuiBlock<'a, DimParentSets, H> for HMarginGuiBlock<I> {
        type Sized = HMarginSizedGuiBlock<I::Sized>;

        fn size(self, w: f32, h_in: H::In, scale: f32) -> ((), H::Out, Self::Sized) {
            let margin_min = self.unscaled_margin_min * scale;
            let margin_max = self.unscaled_margin_max * scale;

            let inner_w = f32::max(w - margin_min - margin_max, 0.0);
            let x_translate = (w - inner_w) / 2.0;

            let ((), h_out, inner_sized) = self.inner.size(inner_w, h_in, scale);

            let sized = HMarginSizedGuiBlock {
                x_translate,
                inner: inner_sized,
            };

            ((), h_out, sized)
        }
    }

    struct HMarginSizedGuiBlock<I> {
        x_translate: f32,
        inner: I,
    }

    impl<'a, I: SizedGuiBlock<'a>> SizedGuiBlock<'a> for HMarginSizedGuiBlock<I> {
        fn visit_nodes<T: GuiVisitorTarget<'a>>(self, mut visitor: GuiVisitor<'_, T>) {
            self.inner.visit_nodes(visitor.reborrow()
                .translate([self.x_translate, 0.0]));
        }
    }


    // ==== TODO dedupe somehow ====


    pub fn v_margin_gui_block<'a, W: DimConstraint, I: GuiBlock<'a, W, DimParentSets>>(unscaled_margin_min: f32, unscaled_margin_max: f32, inner: I) -> impl GuiBlock<'a, W, DimParentSets> {
        VMarginGuiBlock {
            unscaled_margin_min,
            unscaled_margin_max,
            inner,
        }
    }

    struct VMarginGuiBlock<I> {
        unscaled_margin_min: f32,
        unscaled_margin_max: f32,
        inner: I,
    }

    impl<'a, W: DimConstraint, I: GuiBlock<'a, W, DimParentSets>> GuiBlock<'a, W, DimParentSets> for VMarginGuiBlock<I> {
        type Sized = VMarginSizedGuiBlock<I::Sized>;

        fn size(self, w_in: W::In, h: f32, scale: f32) -> (W::Out, (), Self::Sized) {
            let margin_min = self.unscaled_margin_min * scale;
            let margin_max = self.unscaled_margin_max * scale;

            let inner_h = f32::max(h - margin_min - margin_max, 0.0);
            let y_translate = (h - inner_h) / 2.0;

            let (w_out, (), inner_sized) = self.inner.size(w_in, inner_h, scale);

            let sized = VMarginSizedGuiBlock {
                y_translate,
                inner: inner_sized,
            };

            (w_out, (), sized)
        }
    }

    struct VMarginSizedGuiBlock<I> {
        y_translate: f32,
        inner: I,
    }

    impl<'a, I: SizedGuiBlock<'a>> SizedGuiBlock<'a> for VMarginSizedGuiBlock<I> {
        fn visit_nodes<T: GuiVisitorTarget<'a>>(self, mut visitor: GuiVisitor<'_, T>) {
            self.inner.visit_nodes(visitor.reborrow()
                .translate([0.0, self.y_translate]));
        }
    }