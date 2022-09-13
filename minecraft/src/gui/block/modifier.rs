use super::*;

    pub fn modifier_gui_block<'a, W: DimConstraint, H: DimConstraint, M: Into<Modifier2>, I: GuiBlock<'a, W, H>>(modifier: M, inner: I) -> impl GuiBlock<'a, W, H> {
        let modifier = modifier.into();
        ModifierGuiBlock {
            modifier,
            inner,
        }
    }

    struct ModifierGuiBlock<I> {
        modifier: Modifier2,
        inner: I,
    }

    impl<'a, W: DimConstraint, H: DimConstraint, I: GuiBlock<'a, W, H>> GuiBlock<'a, W, H> for ModifierGuiBlock<I> {
        type Sized = ModifierSizedGuiBlock<I::Sized>;

        fn size(self, w_in: W::In, h_in: H::In, scale: f32) -> (W::Out, H::Out, Self::Sized) {
            let (w_out, h_out, inner_sized) = self.inner.size(w_in, h_in, scale);
            let sized = ModifierSizedGuiBlock {
                modifier: self.modifier,
                inner: inner_sized,
            };
            (w_out, h_out, sized)
        }
    }

    struct ModifierSizedGuiBlock<I> {
        modifier: Modifier2,
        inner: I,
    }

    impl<'a, I: SizedGuiBlock<'a>> SizedGuiBlock<'a> for ModifierSizedGuiBlock<I> {
        fn visit_nodes<T: GuiVisitorTarget<'a>>(self, mut visitor: GuiVisitor<'_, T>) {
            self.inner.visit_nodes(visitor.reborrow()
                .modify(self.modifier));
        }
    }