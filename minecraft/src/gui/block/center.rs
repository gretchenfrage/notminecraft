
use crate::gui::{
    GuiVisitor,
    GuiVisitorTarget,
    block::{
        DimConstraint,
        DimParentSets,
        DimChildSets,
        GuiBlock,
        SizedGuiBlock,
    },
};


/// Gui block that horizontally centers its child.
///
/// `frac` is the fraction through self's horizontal length to put the
/// horizontal center of child. Set to `0.5` to actually center.
pub fn h_center<'a, H: DimConstraint, I: GuiBlock<'a, DimChildSets, H>>(frac: f32, inner: I) -> impl GuiBlock<'a, DimParentSets, H> {
    HCenter {
        frac,
        inner,
    }
}


struct HCenter<I> {
    frac: f32,
    inner: I,
}

impl<'a, H: DimConstraint, I: GuiBlock<'a, DimChildSets, H>> GuiBlock<'a, DimParentSets, H> for HCenter<I> {
    type Sized = HCenterSized<I::Sized>;

    fn size(self, w: f32, h_in: H::In, scale: f32) -> ((), H::Out, Self::Sized) {
        let (inner_w, h_out, inner_sized) = self.inner.size((), h_in, scale);
        let sized = HCenterSized {
            x_translate: (w - inner_w) * self.frac,
            inner: inner_sized,
        };
        ((), h_out, sized)
    }
}


struct HCenterSized<I> {
    x_translate: f32,
    inner: I,
}

impl<'a, I: SizedGuiBlock<'a>> SizedGuiBlock<'a> for HCenterSized<I> {
    fn visit_nodes<T: GuiVisitorTarget<'a>>(self, mut visitor: GuiVisitor<'_, T>) {
        self.inner.visit_nodes(visitor.reborrow()
            .translate([self.x_translate, 0.0]));
    }
}


// ==== TODO dedupe somehow ====


/// Gui block that vertically centers its child.
///
/// `frac` is the fraction through self's vertical length to put the
/// vertical center of child. Set to `0.5` to actually center.
pub fn v_center<'a, W: DimConstraint, I: GuiBlock<'a, W, DimChildSets>>(frac: f32, inner: I) -> impl GuiBlock<'a, W, DimParentSets> {
    VCenter {
        frac,
        inner,
    }
}

struct VCenter<I> {
    frac: f32,
    inner: I,
}

impl<'a, W: DimConstraint, I: GuiBlock<'a, W, DimChildSets>> GuiBlock<'a, W, DimParentSets> for VCenter<I> {
    type Sized = VCenterSized<I::Sized>;

    fn size(self, w_in: W::In, h: f32, scale: f32) -> (W::Out, (), Self::Sized) {
        let (w_out, inner_h, inner_sized) = self.inner.size(w_in, (), scale);
        let sized = VCenterSized {
            y_translate: (h - inner_h) * self.frac,
            inner: inner_sized,
        };
        (w_out, (), sized)
    }
}


struct VCenterSized<I> {
    y_translate: f32,
    inner: I,
}

impl<'a, I: SizedGuiBlock<'a>> SizedGuiBlock<'a> for VCenterSized<I> {
    fn visit_nodes<T: GuiVisitorTarget<'a>>(self, mut visitor: GuiVisitor<'_, T>) {
        self.inner.visit_nodes(visitor.reborrow()
            .translate([0.0, self.y_translate]));
    }
}
