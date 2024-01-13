
use crate::gui::prelude::*;
use std::time::Duration;


pub fn fade<'a, W, H, I>(
    fade_at: Duration,
    fade_for_secs: f32,
    inner: I,
) -> impl GuiBlock<'a, W, H>
where
    W: DimConstraint,
    H: DimConstraint,
    I: GuiBlock<'a, W, H>,
{
    Fade {
        fade_at,
        fade_for_secs,
        inner,
    }
}


struct Fade<I> {
    fade_at: Duration,
    fade_for_secs: f32,
    inner: I,
}

impl<
    'a,
    W: DimConstraint,
    H: DimConstraint,
    I: GuiBlock<'a, W, H>,
> GuiBlock<'a, W, H> for Fade<I> {
    type Sized = FadeSized<I::Sized>;

    fn size(
        self,
        ctx: &GuiGlobalContext<'a>,
        w_in: W::In,
        h_in: H::In,
        scale: f32,
    ) -> (W::Out, H::Out, Self::Sized) {
        let alpha = if ctx.time_since_epoch > self.fade_at {
            let secs_faded_for = (ctx.time_since_epoch - self.fade_at).as_secs_f32();
            if secs_faded_for < self.fade_for_secs {
                1.0 - secs_faded_for / self.fade_for_secs
            } else {
                0.0
            }
        } else {
            1.0
        };


        let (w_out, h_out, inner_sized) = self.inner.size(ctx, w_in, h_in, scale);
        (w_out, h_out, FadeSized {
            alpha,
            inner: inner_sized,
        })
    }
}

struct FadeSized<I> {
    alpha: f32,
    inner: I,
}

impl<'a, I: SizedGuiBlock<'a>> SizedGuiBlock<'a> for FadeSized<I> {
    fn visit_nodes<T: GuiVisitorTarget<'a>>(
        self,
        visitor: &mut GuiVisitor<'a, '_, T>,
        forward: bool,
    ) {
        let mut visitor = visitor.reborrow()
            .color([1.0, 1.0, 1.0, self.alpha]);
        self.inner.visit_nodes(&mut visitor, forward);
    }
}
