
use super::{
    False,
    UiBlock,
    UiBlockSetWidth,
    UiBlockSetHeight,
};
use graphics::{
    Renderer,
    frame_content::Canvas2,
};
use vek::*;


pub struct UiStableUnscaledWidthBlock<I> {
    unscaled_width: f32,

    size: Extent2<f32>,
    scale: f32,

    pub inner: I,
}

impl<I> UiStableUnscaledWidthBlock<I> {
    pub fn new<F>(
        unscaled_width: f32,
        create_inner: F,
        height: f32,
        scale: f32,
    ) -> Self
    where
        F: FnOnce(Extent2<f32>, f32) -> I,
    {
        let size = Extent2 {
            w: unscaled_width * scale,
            h: height,
        };
        let inner = create_inner(size, scale);
        
        UiStableUnscaledWidthBlock {
            unscaled_width,
            size,
            scale,
            inner,
        }
    }
}

impl<
    I: UiBlock<WidthChanged=False> + UiBlockSetWidth
> UiBlock for UiStableUnscaledWidthBlock<I> {
    type WidthChanged = bool;
    type HeightChanged = <I as UiBlock>::HeightChanged;

    fn draw<'a>(&'a self, mut canvas: Canvas2<'a, '_>) {
        self.inner.draw(canvas.reborrow());
    }

    fn width(&self) -> f32 {
        self.size.w
    }

    fn height(&self) -> f32 {
        self.size.h
    }

    fn scale(&self) -> f32 {
        self.scale
    }

    fn set_scale(&mut self, renderer: &Renderer, scale: f32) -> (
        Self::WidthChanged,
        Self::HeightChanged,
    )
    {
        self.scale = scale;

        self.size.w = self.unscaled_width * self.scale;

        let (
            False,
            height_changed,
        ) = self.inner.set_scale(renderer, self.scale);
        self.inner.set_width(renderer, self.size.w);

        (true, height_changed)
    }
}

impl<I: UiBlockSetHeight> UiBlockSetHeight for UiStableUnscaledWidthBlock<I> {
    fn set_height(&mut self, renderer: &Renderer, height: f32) {
        self.inner.set_height(renderer, height);
    }
}


// ==== TODO dedupe this somehow ====


pub struct UiStableUnscaledHeightBlock<I> {
    unscaled_height: f32,

    size: Extent2<f32>,
    scale: f32,

    pub inner: I,
}

impl<I> UiStableUnscaledHeightBlock<I> {
    pub fn new<F>(
        unscaled_height: f32,
        create_inner: F,
        width: f32,
        scale: f32,
    ) -> Self
    where
        F: FnOnce(Extent2<f32>, f32) -> I,
    {
        let size = Extent2 {
            w: width,
            h: unscaled_height * scale,
        };
        let inner = create_inner(size, scale);
        
        UiStableUnscaledHeightBlock {
            unscaled_height,
            size,
            scale,
            inner,
        }
    }
}

impl<
    I: UiBlock<HeightChanged=False> + UiBlockSetHeight
> UiBlock for UiStableUnscaledHeightBlock<I> {
    type WidthChanged = <I as UiBlock>::WidthChanged;
    type HeightChanged = bool;

    fn draw<'a>(&'a self, mut canvas: Canvas2<'a, '_>) {
        self.inner.draw(canvas.reborrow());
    }

    fn width(&self) -> f32 {
        self.size.w
    }

    fn height(&self) -> f32 {
        self.size.h
    }

    fn scale(&self) -> f32 {
        self.scale
    }

    fn set_scale(&mut self, renderer: &Renderer, scale: f32) -> (
        Self::WidthChanged,
        Self::HeightChanged,
    )
    {
        self.scale = scale;

        self.size.h = self.unscaled_height * self.scale;

        let (
            width_changed,
            False,
        ) = self.inner.set_scale(renderer, self.scale);
        self.inner.set_height(renderer, self.size.h);

        (width_changed, true)
    }
}

impl<I: UiBlockSetWidth> UiBlockSetWidth for UiStableUnscaledHeightBlock<I> {
    fn set_width(&mut self, renderer: &Renderer, width: f32) {
        self.inner.set_width(renderer, width);
    }
}
