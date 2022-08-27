

use super::{
    False,
    UiBlock,
    UiBlockSetWidth,
    UiBlockSetHeight,
    UiBlockItems,
    UiBlockItemsSetWidth,
    UiBlockItemsSetHeight,
};
use graphics::{
    Renderer,
    frame_content::Canvas2,
};
use vek::*;


pub struct UiLayerBlock<I> {
    size: Extent2<f32>,
    scale: f32,

    pub items: I,
}


impl<I> UiLayerBlock<I> {
    pub fn new<F>(
        create_items: F,
        size: Extent2<f32>,
        scale: f32,
    ) -> Self
    where
        F: FnOnce(Extent2<f32>, f32) -> I,
    {
        let items = create_items(size, scale);
        UiLayerBlock {
            items,
            size,
            scale,
        }
    }
}


impl<
    I: UiBlockItems<
        WidthChanged=False,
        HeightChanged=False,
    >
> UiBlock for UiLayerBlock<I>
{
    type WidthChanged = False;
    type HeightChanged = False;

    fn draw<'a>(&'a self, mut canvas: Canvas2<'a, '_>) {
        for i in 0..self.items.len() {
            self.items.draw(i, canvas.reborrow());
        }
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

        for i in 0..self.items.len() {
            let (False, False) = self.items.set_scale(i, renderer, scale);
        }

        (False, False)
    }
}

impl<
    I: UiBlockItems + UiBlockItemsSetWidth,
> UiBlockSetWidth for UiLayerBlock<I>
{
    fn set_width(&mut self, renderer: &Renderer, width: f32) {
        self.size.w = width;

        for i in 0..self.items.len() {
            self.items.set_width(i, renderer, width);
        }
    }
}

impl<
    I: UiBlockItems + UiBlockItemsSetHeight,
> UiBlockSetHeight for UiLayerBlock<I>
{
    fn set_height(&mut self, renderer: &Renderer, height: f32) {
        self.size.h = height;

        for i in 0..self.items.len() {
            self.items.set_height(i, renderer, height);
        }
    }
}
