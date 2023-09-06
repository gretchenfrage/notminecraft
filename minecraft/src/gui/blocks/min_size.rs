
use crate::gui::{
    DimConstraint,
    DimChildSets,
    GuiBlock,
    GuiGlobalContext,
    GuiVisitorTarget,
    SizedGuiBlock,
    GuiVisitor,
};
use super::axis_swap;
use vek::*;


pub fn min_width<
    'a,
    H: DimConstraint,
    I: GuiBlock<'a, DimChildSets, H>,
>(logical_min_width: f32, align: f32, inner: I) -> impl GuiBlock<'a, DimChildSets, H>
{
    MinWidth {
        logical_min_width,
        align,
        inner,
    }
}

pub fn min_height<
    'a,
    W: DimConstraint,
    I: GuiBlock<'a, W, DimChildSets>,
>(logical_min_height: f32, align: f32, inner: I) -> impl GuiBlock<'a, W, DimChildSets>
{
    axis_swap(min_width(logical_min_height, align, axis_swap(inner)))
}


pub fn min_size<
    'a,
    S: Into<Extent2<f32>>,
    A: Into<Vec2<f32>>,
    I: GuiBlock<'a, DimChildSets, DimChildSets>,
>(logical_min_size: S, align: A, inner: I) -> impl GuiBlock<'a, DimChildSets, DimChildSets>
{
    let logical_min_size = logical_min_size.into();
    let align = align.into();
    min_width(logical_min_size.w, align.x, min_height(logical_min_size.h, align.y, inner))
}


#[derive(Debug)]
struct MinWidth<I> {
    logical_min_width: f32,
    align: f32,
    inner: I,
}

impl<
    'a,
    H: DimConstraint,
    I: GuiBlock<'a, DimChildSets, H>,
> GuiBlock<'a, DimChildSets, H> for MinWidth<I> {
    type Sized = MinWidthSized<I::Sized>;

    fn size(
        self,
        ctx: &GuiGlobalContext<'a>,
        (): (),
        h_in: H::In,
        scale: f32,
    ) -> (f32, H::Out, Self::Sized) {
        let min_width = self.logical_min_width * scale;
        let (inner_w, h_out, inner_sized) = self.inner.size(ctx, (), h_in, scale);
        let w = f32::max(inner_w, min_width);
        let sized = MinWidthSized {
            x_translate: (w - inner_w) * self.align,
            inner: inner_sized,
        };
        (w, h_out, sized)
    }        
}

#[derive(Debug)]
struct MinWidthSized<I> {
    x_translate: f32,
    inner: I,
}

impl<'a, I: SizedGuiBlock<'a>> SizedGuiBlock<'a> for MinWidthSized<I> {
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
