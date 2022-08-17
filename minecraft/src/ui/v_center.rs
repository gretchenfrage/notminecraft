
use super::{
    UiSize,
    UiPosInputEvent,
};
use graphics::frame_content::Canvas2;
use vek::*;


pub struct UiVCenter<I> {
    pub inner: I,
    fraction_down: f32,
    inner_height: f32,
    y_translate: f32,
    height: f32,
    scale: f32,
}

pub struct UiVCenterConfig<F1, F2> {
    pub create_inner: F1,
    pub get_inner_height: F2,
    pub fraction_down: f32,
}

impl<I> UiVCenter<I> {
    pub fn new<F1, F2>(
        config: UiVCenterConfig<F1, F2>,
        height: f32,
        scale: f32,
    ) -> Self
    where
        F1: FnOnce(
            // scale
            f32,
        ) -> I,
        F2: Fn(&I) -> f32,
    {
        let inner = (config.create_inner)(scale);
        let inner_height = (config.get_inner_height)(&inner);
        UiVCenter {
            inner,
            fraction_down: config.fraction_down,
            inner_height,
            y_translate: (height - inner_height) * config.fraction_down,
            height,
            scale,
        }
    }

    pub fn height(&self) -> f32 {
        self.height
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }

    pub fn draw<'a, F>(&'a self, mut canvas: Canvas2<'a, '_>, draw_inner: F)
    where
        F: FnOnce(&'a I, Canvas2<'a, '_>),
    {
        let mut canvas = canvas.reborrow()
            .translate([0.0, self.y_translate]);
        draw_inner(&self.inner, canvas);
    }

    pub fn set_height(&mut self, height: f32)
    {
        self.height = height;

        //self.y_translate = (self.height - self.inner_height) * self.fraction_down;
        self.y_translate = (self.height /*- self.inner_height TODO whatevers*/) * (self.fraction_down + 0.1 /* TODO whaaaatevers */) - self.inner_height /* TODO whaaaateversssssss */;
    }

    pub fn set_scale<F1, F2>(
        &mut self,
        scale: f32,
        mut set_inner_scale: F1,
        get_inner_height: F2,
    )
    where
        F1: FnOnce(&mut I, f32),
        F2: Fn(&I) -> f32,
    {
        self.scale = scale;

        set_inner_scale(&mut self.inner, self.scale);

        self.inner_height = get_inner_height(&self.inner);
        debug!(inner_height=%self.inner_height);
        self.y_translate = (self.height /*- self.inner_height TODO whatevers*/) * (self.fraction_down - 0.2 /* TODO whaaaatevers */) - self.inner_height /* TODO whaaaateversssssss */;
    }

    pub fn on_pos_input_event<F>(
        &mut self,
        event: UiPosInputEvent,
        inner_on_pos_input_event: F,
    )
    where
        F: Fn(&mut I, UiPosInputEvent)
    {
        inner_on_pos_input_event(
            &mut self.inner,
            event.map_pos(|v| v - Vec2::new(0.0, self.y_translate)),
        )
    }
}
