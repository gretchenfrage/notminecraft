
use super::UiSize;
use graphics::frame_content::Canvas2;


pub struct UiHCenter<I> {
    pub inner: I,
    unscaled_inner_width: f32,
    x_translate: f32,
    width: f32,
    scale: f32,
}

pub struct UiHCenterConfig<I> {
    pub inner: I,
    pub unscaled_inner_width: f32,
}

impl<I> UiHCenter<I> {
    pub fn new(
        config: UiHCenterConfig<I>,
        width: f32,
        scale: f32,
    ) -> Self {
        UiHCenter {
            inner: config.inner,
            unscaled_inner_width: config.unscaled_inner_width,
            x_translate: (width - config.unscaled_inner_width * scale) / 2.0,
            width,
            scale,
        }
    }
    
    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }

    pub fn draw<'a, F>(&'a self, mut canvas: Canvas2<'a, '_>, draw_inner: F)
    where
        F: FnOnce(&'a I, Canvas2<'a, '_>),
    {
        let mut canvas = canvas.reborrow()
            .translate([self.x_translate, 0.0]);
        draw_inner(&self.inner, canvas);
    }

    pub fn set_width(&mut self, width: f32)
    {
        self.width = width;

        self.x_translate = (self.width - self.unscaled_inner_width * self.scale) / 2.0;
    }

    pub fn set_scale<P, F1, F2>(
        &mut self,
        scale: f32,
        passthrough: &mut P,
        mut set_inner_scale: F1,
        mut set_inner_width: F2,
    )
    where
        F1: FnOnce(&mut P, &mut I, f32),
        F2: FnOnce(&mut P, &mut I, f32),
    {
        self.scale = scale;

        self.x_translate = (self.width - self.unscaled_inner_width * self.scale) / 2.0;

        set_inner_scale(passthrough, &mut self.inner, self.scale);
        set_inner_width(passthrough, &mut self.inner, self.scale * self.unscaled_inner_width);
    }
}
