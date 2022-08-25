
use super::{
    False,
    UiBlock,
    UiBlockSetWidth,
    UiBlockItems,
    UiBlockItemsSetWidth,
};
use graphics::{
    Renderer,
    frame_content::Canvas2,
};
use vek::*;


pub struct UiVStackBlock<I> {
    unscaled_gap: f32,

    size: Extent2<f32>,
    scale: f32,

    item_y_translates: Vec<f32>,
    pub items: I,
}


fn compute_layout<I: UiBlockItems>(
    unscaled_gap: f32,
    scale: f32,
    items: &I,
    item_y_translates: &mut Vec<f32>,
) -> f32
{
    let mut height = 0.0;

    for i in 0..items.len() {
        if i > 0 {
            height += unscaled_gap * scale;
        }
        item_y_translates.push(height);
        height += items.height(i);
    }

    height
}

impl<I: UiBlockItems> UiVStackBlock<I> {
    pub fn new<F>(
        unscaled_gap: f32,
        create_items: F,
        width: f32,
        scale: f32,
    ) -> Self
    where
        F: FnOnce(f32, f32) -> I, // (width, scale) -> I
    {
        let items = create_items(width, scale);

        let mut item_y_translates = Vec::new();
        let height = compute_layout(
            unscaled_gap,
            scale,
            &items,
            &mut item_y_translates,
        );

        UiVStackBlock {
            unscaled_gap,
            size: [width, height].into(),
            scale,
            item_y_translates,
            items,
        }
    }
}

impl<I: UiBlockItems<WidthChanged=False>> UiBlock for UiVStackBlock<I> {
    type WidthChanged = False;
    type HeightChanged = bool;

    fn draw<'a>(&'a self, mut canvas: Canvas2<'a, '_>) {
        for i in 0..self.items.len() {
            self.items.draw(i, canvas.reborrow()
                .translate([0.0, self.item_y_translates[i]]));
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
            self.items.set_scale(i, renderer, scale);
        }

        self.item_y_translates.clear();
        let height = compute_layout(
            self.unscaled_gap,
            self.scale,
            &self.items,
            &mut self.item_y_translates,
        );
        self.size.h = height;


        (False, true)
    }
}

impl<
    I: UiBlockItems + UiBlockItemsSetWidth,
> UiBlockSetWidth for UiVStackBlock<I>
{
    fn set_width(&mut self, renderer: &Renderer, width: f32) {
        for i in 0..self.items.len() {
            self.items.set_width(i, renderer, width);
        }
    }
}
