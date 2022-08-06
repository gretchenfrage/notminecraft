
use super::{
    Margins,
    UiSize,
};
use graphics::{
    Renderer,
    frame_content::{
        Canvas2,
        TextBlock,
        TextSpan,
        HAlign,
        VAlign,
        LayedOutTextBlock,
        FontId,
    },
};
use vek::*;


const SHADOW_DROP_DIVISOR: f32 = 8.0;
const BOTTOM_RIGHT_MYSTERY_GAP: Extent2<f32> =
    Extent2 {
        w: 2.0 / 8.0,
        h: 3.0 / 8.0,
    };


#[derive(Debug, Clone)]
pub struct UiTextConfig {
    pub text: String,
    pub font: FontId,
    pub font_size: f32,
    pub color: Rgba<f32>,
    pub h_align: HAlign,
    pub v_align: VAlign,
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

    pub fn draw<'a>(&'a self, mut canvas: Canvas2<'a, '_>) {
        let shadow_drop = self.config.font_size / SHADOW_DROP_DIVISOR * self.scale;
        let align_sign = Vec2 {
            x: self.config.h_align.sign(),
            y: self.config.v_align.sign(),
        };
        
        let mystery_gap_adjust_fractional =
            align_sign.map(|n| (n as f32 / 2.0 + 0.5));
        let mystery_gap_adjust =
            mystery_gap_adjust_fractional
            * self.config.font_size
            * self.scale
            * BOTTOM_RIGHT_MYSTERY_GAP;
        let mut canvas = canvas.reborrow()
            .translate(mystery_gap_adjust);

        canvas.reborrow()
            .translate(align_sign.map(|n| (n as f32 / -2.0 + 0.5) * shadow_drop))
            .color([0.25, 0.25, 0.25, 1.0])
            .draw_text(&self.layed_out);
        canvas.reborrow()
            .translate(align_sign.map(|n| (n as f32 / -2.0 - 0.5) * shadow_drop))
            .draw_text(&self.layed_out);
    }

    pub fn set_wrap_width(
        &mut self,
        renderer: &Renderer,
        wrap_width: Option<f32>,
    ) {
        self.wrap_width = wrap_width;
        self.re_lay_out(renderer);
    }

    pub fn set_scale(&mut self, renderer: &Renderer, scale: f32) {
        self.scale = scale;
        self.re_lay_out(renderer);
    }
}


pub struct UiTextBlockConfig {
    pub text_config: UiTextConfig,
    pub margins: Margins,
    pub wrap: bool,
}

pub struct UiTextBlock {
    ui_text: UiText,
    margins: Margins,
    wrap: bool,
    align_translate_fraction: Vec2<f32>,
    margin_translate_unscaled: Vec2<f32>,
    size: UiSize,
}

fn wrap_width(margins: Margins, wrap: bool, size: UiSize) -> Option<f32> {
    if wrap {
        let h_margins_unscaled = margins.left + margins.right;
        let h_margins = h_margins_unscaled * size.scale;
        Some(size.size.w - h_margins)
    } else {
        None
    }
}

impl UiTextBlock {
    pub fn new(
        renderer: &Renderer,
        config: UiTextBlockConfig,
        size: UiSize,
    ) -> Self {
        let wrap_width = wrap_width(config.margins, config.wrap, size);
        let align_translate_fraction = Vec2 {
            x: match config.text_config.h_align {
                HAlign::Left => 0.0,
                HAlign::Center => 0.5,
                HAlign::Right => 1.0,
            },
            y: match config.text_config.v_align {
                VAlign::Top => 0.0,
                VAlign::Center => 0.5,
                VAlign::Bottom => 1.0,
            },
        };
        let margin_translate_unscaled = Vec2 {
            x: match config.text_config.h_align {
                HAlign::Left => config.margins.left,
                HAlign::Center => 0.0,
                HAlign::Right => -config.margins.right,
            },
            y: match config.text_config.v_align {
                VAlign::Top => config.margins.top,
                VAlign::Center => 0.0,
                VAlign::Bottom => -config.margins.bottom,
            },
        };
        let ui_text = UiText::new(
            renderer,
            config.text_config,
            wrap_width,
            size.scale,
        );
        UiTextBlock {
            ui_text,
            margins: config.margins,
            wrap: config.wrap,
            align_translate_fraction,
            margin_translate_unscaled,
            size,
        }
    }

    pub fn draw<'a>(&'a self, mut canvas: Canvas2<'a, '_>) {
        let canvas = canvas.reborrow()
            .translate(self.align_translate_fraction * self.size.size)
            .translate(self.margin_translate_unscaled * self.size.scale);
        self.ui_text.draw(canvas); // TODO allow for chaining syntax for these such cases
    }

    pub fn set_size(
        &mut self,
        renderer: &Renderer,
        size: impl Into<Extent2<f32>>,
    ) {
        self.size.size = size.into();
        let wrap_width = wrap_width(self.margins, self.wrap, self.size);
        self.ui_text.set_wrap_width(renderer, wrap_width);
    }

    pub fn set_scale(&mut self, renderer: &Renderer, scale: f32) {
        self.size.scale = scale;
        self.ui_text.set_scale(renderer, self.size.scale);
    }
}
