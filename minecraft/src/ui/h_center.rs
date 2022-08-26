
use super::{
    UiSize,
    UiPosInputEvent,
};
use graphics::frame_content::Canvas2;
use vek::*;


pub struct UiHCenterBlock<I> {
    pub inner: I,
    unscaled_inner_width: f32,
    x_translate: f32,
    width: f32,
    scale: f32,

    debug_dot: Option<Vec2<f32>>,
}

pub struct UiHCenterBlockConfig<F> {
    pub create_inner: F,
    pub unscaled_inner_width: f32,
}

impl<I> UiHCenterBlock<I> {
    pub fn new<F>(
        config: UiHCenterBlockConfig<F>,
        width: f32,
        scale: f32,
    ) -> Self
    where
        F: FnOnce(
            // width
            f32,
            // scale
            f32,
        ) -> I,
    {
        let inner_width = config.unscaled_inner_width * scale;
        let inner = (config.create_inner)(inner_width, scale);
        UiHCenterBlock {
            inner,
            unscaled_inner_width: config.unscaled_inner_width,
            x_translate: (width - inner_width) / 2.0,
            width,
            scale,

            debug_dot: None,
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
        draw_inner(&self.inner, canvas.reborrow());

        if let Some(pos) = self.debug_dot {
            let dot_size = Extent2 { w: 10.0, h: 10.0 };
            canvas.reborrow()
                .translate(pos)
                .translate(-dot_size / 2.0)
                .color(Rgba::red())
                .draw_solid(dot_size);
        }
    }

    pub fn set_width(&mut self, width: f32)
    {
        self.width = width;

        self.x_translate = (self.width - self.unscaled_inner_width * self.scale) / 2.0;
    }

    pub fn set_scale<F1, F2>(
        &mut self,
        scale: f32,
        mut set_inner_scale: F1,
        mut set_inner_width: F2,
    )
    where
        F1: FnOnce(&mut I, f32),
        F2: FnOnce(&mut I, f32),
    {
        self.scale = scale;

        self.x_translate = (self.width - self.unscaled_inner_width * self.scale) / 2.0;

        set_inner_scale(&mut self.inner, self.scale);
        set_inner_width(&mut self.inner, self.scale * self.unscaled_inner_width);
    }

    pub fn on_pos_input_event<F>(
        &mut self,
        event: UiPosInputEvent,
        inner_on_pos_input_event: F,
    )
    where
        F: Fn(&mut I, UiPosInputEvent)
    {
        let event = event.map_pos(|v| v - Vec2::new(self.x_translate, 0.0));
        /*match event {
            UiPosInputEvent::CursorMoved(pos) => {
                self.debug_dot = Some(pos);
            }
            _ => ()
        }*/
        inner_on_pos_input_event(
            &mut self.inner,
            event,
        )
    }
}
