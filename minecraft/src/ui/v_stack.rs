
use super::UiSize;
use graphics::frame_content::Canvas2;


#[derive(Debug, Clone)]
pub struct UiVStack {
    unscaled_gap: f32,
    size: UiSize,
    item_y_translates: Vec<f32>,
}

pub struct UiVStackConfig<F> {
    pub unscaled_gap: f32,
    pub num_items: usize,
    pub get_item_height: F,
}

impl UiVStack {
    pub fn new<F>(
        config: UiVStackConfig<F>,
        width: f32,
        scale: f32,
    ) -> Self
    where
        F: Fn(usize) -> f32,
    {
        let mut item_y_translates = Vec::new();
        let mut height = 0.0;

        for i in 0..config.num_items {
            if i > 0 {
                height += config.unscaled_gap * scale;
            }
            item_y_translates.push(height);
            height += (config.get_item_height)(i);
        }

        UiVStack {
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
        F: Fn(usize, Canvas2<'a, '_>),
    {
        for (i, &translate) in self.item_y_translates.iter().enumerate() {
            let mut canvas = canvas.reborrow()
                .translate([0.0, translate]);
            draw_item(i, canvas.reborrow());
        }
    }

    pub fn set_width<F>(&mut self, width: f32, mut set_item_width: F)
    where
        F: FnMut(usize, f32),
    {
        self.size.size.w = width;

        for i in 0..self.item_y_translates.len() {
            set_item_width(i, width);
        }
    }

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
}
