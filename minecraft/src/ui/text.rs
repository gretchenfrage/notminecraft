
use super::{
    UiSize,
    UiElem,
};
use graphics::{
    Renderer,
    frame_content::{
        Canvas2,
        TextBlock,
        TextSpan,
        HorizontalAlign,
        VerticalAlign,
        LayedOutTextBlock,
        FontId,
    },
};
use vek::*;


const SHADOW_DROP_DIVISOR: f32 = 8.0;


#[derive(Debug, Clone)]
pub struct UiTextConfig {
    pub text: String,
    pub font: FontId,
    pub font_size: f32,
    pub color: Rgba<f32>,
    pub h_align: HorizontalAlign,
    pub v_align: VerticalAlign,
}

#[derive(Debug, Clone)]
pub struct UiText {
    config: UiTextConfig,
    wrap_width: Option<f32>,
    scale: f32,

    layed_out: LayedOutTextBlock,
}

fn lay_out(
    renderer: &Renderer,
    config: &UiTextConfig,
    wrap_width: Option<f32>,
    scale: f32,
) -> LayedOutTextBlock {
    renderer
        .lay_out_text(&TextBlock {
            spans: &[
                TextSpan {
                    text: &config.text,
                    font: config.font,
                    font_size: config.font_size * scale,
                    color: config.color,
                },
            ],
            h_align: config.h_align,
            v_align: config.v_align,
            wrap_width,
        })
}

impl UiText {
    pub fn new(
        renderer: &Renderer,
        config: UiTextConfig,
        wrap_width: Option<f32>,
        scale: f32,
    ) -> Self {
        let layed_out = lay_out(
            renderer,
            &config,
            wrap_width,
            scale,
        );
        UiText {
            config,
            wrap_width,
            scale,

            layed_out,
        }
    }

    fn re_lay_out(&mut self, renderer: &Renderer) {
        self.layed_out = lay_out(
            renderer,
            &self.config,
            self.wrap_width,
            self.scale,
        );
    }

    pub fn set_wrap_width(
        &mut self,
        renderer: &Renderer,
        wrap_width: Option<f32>,
    ) {
        self.wrap_width = wrap_width;
        self.re_lay_out(renderer);
    }
}

impl UiElem for UiText {
    fn draw<'a>(&self, mut canvas: Canvas2<'a, '_>) {
        let shadow_drop = self.config.font_size / SHADOW_DROP_DIVISOR * self.scale;
        canvas.reborrow()
            .translate([shadow_drop; 2])
            .color([0.25, 0.25, 0.25, 1.0])
            .draw_text(&self.layed_out);
        canvas.reborrow()
            .draw_text(&self.layed_out);
    }

    fn set_scale(&mut self, renderer: &Renderer, scale: f32) {
        self.scale = scale;
        self.re_lay_out(renderer);
    }
}
