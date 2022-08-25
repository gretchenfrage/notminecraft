
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


pub struct UiHCenterBlock<I> {
    size: Extent2<f32>,
    scale: f32,

    x_translate: f32,
    pub inner: I,
}

fn x_translate<I: UiBlock>(size: Extent2<f32>, inner: &I) -> f32 {
    (size.w - inner.width()) / 2.0
}

impl<I: UiBlock> UiHCenterBlock<I> {
    pub fn new<F>(
        create_inner: F,
        size: Extent2<f32>,
        scale: f32,
    ) -> Self
    where
        F: FnOnce(f32) -> I,
    {
        let inner = create_inner(scale);
        let x_translate = x_translate(size, &inner);

        UiHCenterBlock {
            size,
            scale,
            x_translate,
            inner,
        }
    }
}

impl<I: UiBlock> UiBlock for UiHCenterBlock<I> {
    type WidthChanged = False;
    type HeightChanged = False;

    fn draw<'a>(&'a self, mut canvas: Canvas2<'a, '_>) {
        self.inner.draw(canvas.reborrow()
            .translate([self.x_translate, 0.0]));
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

        let (
            inner_width_changed,
            _,
        ) = self.inner.set_scale(renderer, self.scale);
        if inner_width_changed.into() {
            self.x_translate = x_translate(self.size, &self.inner);
        }

        (False, False)
    }
}

impl<I: UiBlock> UiBlockSetWidth for UiHCenterBlock<I> {
    fn set_width(&mut self, _: &Renderer, width: f32) {
        self.size.w = width;

        self.x_translate = x_translate(self.size, &self.inner);
    }
}

impl<I: UiBlock> UiBlockSetHeight for UiHCenterBlock<I> {
    fn set_height(&mut self, _: &Renderer, height: f32) {
        self.size.h = height;

        self.x_translate = x_translate(self.size, &self.inner);
    }
}
