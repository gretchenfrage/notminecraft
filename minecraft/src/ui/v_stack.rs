
use super::UiSize;
use graphics::frame_content::Canvas2;


#[derive(Debug, Clone)]
pub struct UiVStack<I> {
    pub items: I,
    unscaled_gap: f32,
    size: UiSize,
    item_y_translates: Vec<f32>,
}

pub struct UiVStackConfig<F1, F2> {
    pub create_items: F1,
    pub unscaled_gap: f32,
    pub num_items: usize,
    pub get_item_height: F2,
}

impl<I> UiVStack<I> {
    pub fn new<F1, F2>(
        mut config: UiVStackConfig<F1, F2>,
        width: f32,
        scale: f32,
    ) -> Self
    where
        F1: FnMut(
            // width
            f32,
            // scale
            f32,
        ) -> I,
        F2: Fn(&I, usize) -> f32,
    {
        let items = (config.create_items)(width, scale);

        let mut item_y_translates = Vec::new();
        let mut height = 0.0;

        for i in 0..config.num_items {
            if i > 0 {
                height += config.unscaled_gap * scale;
            }
            item_y_translates.push(height);
            height += (config.get_item_height)(&items, i);
        }

        UiVStack {
            items: items,
            unscaled_gap: config.unscaled_gap,
            size: UiSize {
                size: [width, height].into(),
                scale,
            },
            item_y_translates,
        }
    }

    pub fn size(&self) -> UiSize {
        self.size
    }

    pub fn draw<'a, F>(&'a self, mut canvas: Canvas2<'a, '_>, draw_item: F)
    where
        F: Fn(&'a I, usize, Canvas2<'a, '_>),
    {
        for (i, &translate) in self.item_y_translates.iter().enumerate() {
            let mut canvas = canvas.reborrow()
                .translate([0.0, translate]);
            draw_item(&self.items, i, canvas.reborrow());
        }
    }

    pub fn set_width<F>(&mut self, width: f32, mut set_item_width: F)
    where
        F: FnMut(&mut I, usize, f32),
    {
        self.size.size.w = width;

        for i in 0..self.item_y_translates.len() {
            set_item_width(&mut self.items, i, width);
        }
    }
    
    pub fn set_scale<F1, F2>(
        &mut self,
        scale: f32,
        mut set_item_scale: F1,
        get_item_height: F2,
    )
    where
        F1: FnMut(&mut I, usize, f32),
        F2: Fn(&I, usize) -> f32,
    {
        self.size.scale = scale;

        let mut height = 0.0;
        for (i, translate) in self.item_y_translates.iter_mut().enumerate() {
            if i > 0 {
                height += self.unscaled_gap * self.size.scale;
            }
            *translate = height;
            set_item_scale(&mut self.items, i, self.size.scale);
            height += get_item_height(&self.items, i);
        }
    }
    /*
    pub fn set_scale<P, F1, F2>(
        &mut self,
        scale: f32,
        passthrough: &mut P,
        mut set_item_scale: F1,
        get_item_height: F2,
    )
    where
        F1: FnMut(&mut P, usize, f32),
        F2: Fn(&P, usize) -> f32,
    {
        self.size.scale = scale;

        let mut height = 0.0;
        for (i, translate) in self.item_y_translates.iter_mut().enumerate() {
            if i > 0 {
                height += self.unscaled_gap * self.size.scale;
            }
            *translate = height;
            set_item_scale(passthrough, i, self.size.scale);
            height += get_item_height(passthrough, i);
        }
    }
    */
}
