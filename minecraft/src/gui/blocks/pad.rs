
use crate::gui::{
    GuiVisitor,
    GuiVisitorTarget,
    DimConstraint,
    DimChildSets,
    GuiBlock,
    SizedGuiBlock,
    GuiGlobalContext,
};
use super::axis_swap;


pub fn pad<'a, I: GuiBlock<'a, DimChildSets, DimChildSets>>(
    logical_pad_left: f32,
    logical_pad_right: f32,
    logical_pad_top: f32,
    logical_pad_bottom: f32,
    inner: I,
) -> impl GuiBlock<'a, DimChildSets, DimChildSets>
{
    h_pad(logical_pad_left, logical_pad_right,
        v_pad(logical_pad_top, logical_pad_bottom,
            inner,
        )
    )
}


/// Gui block with a child-set width that puts left and right padding around
/// its child (of constant pre-scale size), setting the parent's width to a
/// larger value. Passes through the height constraint.
pub fn h_pad<'a, H: DimConstraint, I: GuiBlock<'a, DimChildSets, H>>(
    logical_pad_left: f32,
    logical_pad_right: f32,
    inner: I,
) -> impl GuiBlock<'a, DimChildSets, H> {
    HPad {
        logical_pad_left,
        logical_pad_right,
        inner,
    }
}


pub fn v_pad<'a, W: DimConstraint, I: GuiBlock<'a, W, DimChildSets>>(
    logical_pad_top: f32,
    logical_pad_bottom: f32,
    inner: I,
) -> impl GuiBlock<'a, W, DimChildSets> {
    axis_swap(
        h_pad(
            logical_pad_top,
            logical_pad_bottom,
            axis_swap(inner),
        ),
    )
}


#[derive(Debug)]
struct HPad<I> {
    logical_pad_left: f32,
    logical_pad_right: f32,
    inner: I,
}

impl<
    'a,
    H: DimConstraint,
    I: GuiBlock<'a, DimChildSets, H>,
> GuiBlock<'a, DimChildSets, H> for HPad<I> {
    type Sized = HPadSized<I::Sized>;

    fn size(
        self,
        ctx: &GuiGlobalContext<'a>,
        (): (),
        h_in: H::In,
        scale: f32,
    ) -> (f32, H::Out, Self::Sized) {
        let pad_left = self.logical_pad_left * scale;
        let pad_right = self.logical_pad_right * scale;

        let (
            inner_w,
            h_out,
            inner_sized,
        ) = self.inner.size(ctx, (), h_in, scale);

        let w = pad_left + inner_w + pad_right;
        let sized = HPadSized {
            x_translate: pad_left,
            inner: inner_sized,
        };

        (w, h_out, sized)
    }
}

#[derive(Debug)]
struct HPadSized<I> {
    x_translate: f32,
    inner: I,
}

impl<'a, I: SizedGuiBlock<'a>> SizedGuiBlock<'a> for HPadSized<I> {
    fn visit_nodes<T: GuiVisitorTarget<'a>>(
        self,
        visitor: &mut GuiVisitor<'a, '_, T>,
        forward: bool,
    ) {
        let mut visitor = visitor.reborrow()
            .debug_tag("h_pad")
            .translate([self.x_translate, 0.0]);
        self.inner.visit_nodes(&mut visitor, forward);
    }
}
