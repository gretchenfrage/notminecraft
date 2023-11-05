
use crate::gui::{
    GuiVisitor,
    GuiVisitorTarget,
    DimConstraint,
    GuiBlock,
    SizedGuiBlock,
    GuiGlobalContext,
};
use vek::*;


pub fn logical_translate<
    'a,
    W: DimConstraint,
    H: DimConstraint,
    V: Into<Vec2<f32>>,
    I: GuiBlock<'a, W, H>,
>(logical_translate: V, inner: I) -> impl GuiBlock<'a, W, H> {
    let translate = logical_translate.into();
    LogicalTranslate {
        translate,
        inner,
    }
}


#[derive(Debug)]
struct LogicalTranslate<I> {
    translate: Vec2<f32>,
    inner: I,
}

impl<
    'a,
    W: DimConstraint,
    H: DimConstraint,
    I: GuiBlock<'a, W, H>,
> GuiBlock<'a, W, H> for LogicalTranslate<I>
{
    type Sized = LogicalTranslate<I::Sized>;

    fn size(
        self,
        ctx: &GuiGlobalContext<'a>,
        w_in: W::In,
        h_in: H::In,
        scale: f32,
    ) -> (W::Out, H::Out, Self::Sized)
    {
        let (
            w_out,
            h_out,
            inner_sized,
        ) = self.inner.size(ctx, w_in, h_in, scale);
        let sized = LogicalTranslate {
            translate: self.translate * scale,
            inner: inner_sized,
        };
        (w_out, h_out, sized)
    }
}

impl<'a, I: SizedGuiBlock<'a>> SizedGuiBlock<'a> for LogicalTranslate<I> {
    fn visit_nodes<T: GuiVisitorTarget<'a>>(
        self,
        visitor: &mut GuiVisitor<'a, '_, T>,
        forward: bool,
    ) {
        let mut visitor = visitor.reborrow()
            .debug_tag("logical translate")
            .translate(self.translate);
        self.inner.visit_nodes(&mut visitor, forward);
    }
}
