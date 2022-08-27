
use graphics::{
    Renderer,
    frame_content::{
        FontId,
        HAlign,
        VAlign,
        TextBlock,
        TextSpan,
        LayedOutTextBlock,
        Canvas2,
    },
};
use vek::*;


/// The `UiText` drop shadow will be offset from the actual text by 1 /
/// `SHADOW_DROP_DIVISOR` of the font height in the downwards-right direction.
const SHADOW_DROP_DIVISOR: f32 = 8.0;

/// The `UiText` drop shadow will be tinted by this color.
const SHADOW_DROP_COLOR: Rgba<f32> = Rgba {
    r: 0.25,
    g: 0.25,
    b: 0.25,
    a: 1.0,
};

/// When we ask `ab_glyph` to lay out our text with bottom/right alignment,
/// there's this gap between where it puts the text and the actual bottom-right
/// corner. For now, we use this hack to fix it.
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

/// UI helper utility (not a true UI block) for displaying text with easy-to-
/// work-with positioning, a consistent _pre-scaling_ font size, and a drop
/// shadow.
///
/// A true UI block renders starting at the origin, and then continuing
/// rightwards and downwards. Conversely, this renders:
///
/// - If `h_align == HAlign::Left`: extending rightwards from the origin
/// - If `h_align == HAlign::Right`: extending leftwards from the origin
/// - If `h_align == HAlign::Center`: extending equally rightwards and
///   leftwards from the origin
///
/// And equivalently for `v_align` and the vertical axis.
///
/// A true UI block has a width and a height. This, on the other hand, has a
/// `wrap_width`, which is an `Option<f32>`. It is, however, settable.
#[derive(Debug, Clone)]
pub struct UiText {
    config: UiTextConfig,

    wrap_width: Option<f32>,
    scale: f32,

    text_translates: TextTranslates,
    text: LayedOutTextBlock,
}

#[derive(Debug, Clone)]
struct TextTranslates {
    mystery_gap_adjust_translate: Vec2<f32>,
    text_shadow_translate: Vec2<f32>,
    text_main_translate: Vec2<f32>,
}

fn text_translates(
    config: &UiTextConfig,
    scale: f32,
) -> TextTranslates
{
    let align_sign = Vec2 {
        x: config.h_align.sign(),
        y: config.v_align.sign(),
    };

    let mystery_gap_adjust_fractional =
        align_sign.map(|n| (n as f32 / 2.0 + 0.5));
    let mystery_gap_adjust_translate =
        mystery_gap_adjust_fractional
        * config.font_size
        * scale
        * BOTTOM_RIGHT_MYSTERY_GAP;
    
    let shadow_drop = config.font_size / SHADOW_DROP_DIVISOR * scale;
    let text_shadow_translate = align_sign
        .map(|n| (n as f32 / -2.0 + 0.5) * shadow_drop);
    let text_main_translate = align_sign
        .map(|n| (n as f32 / -2.0 - 0.5) * shadow_drop);

    TextTranslates {
        mystery_gap_adjust_translate,
        text_shadow_translate,
        text_main_translate,
    }
}

fn create_text(
    renderer: &Renderer,
    config: &UiTextConfig,
    wrap_width: Option<f32>,
    scale: f32,
) -> LayedOutTextBlock
{
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
        let text = create_text(
            renderer,
            &config,
            wrap_width,
            scale,
        );
        let text_translates = text_translates(
            &config,
            scale,
        );
        UiText {
            config,
            wrap_width,
            scale,

            text_translates,
            text,
        }
    }

    pub fn draw<'a>(&'a self, mut canvas: Canvas2<'a, '_>) {
        let mut canvas = canvas.reborrow()
            .translate(self.text_translates.mystery_gap_adjust_translate);
        canvas.reborrow()
            .translate(self.text_translates.text_shadow_translate)
            .color(SHADOW_DROP_COLOR)
            .draw_text(&self.text);
        canvas.reborrow()
            .translate(self.text_translates.text_main_translate)
            .draw_text(&self.text);
    }

    fn recreate_text(&mut self, renderer: &Renderer) {
        self.text = create_text(
            renderer,
            &self.config,
            self.wrap_width,
            self.scale,
        );
    }

    pub fn wrap_width(&self) -> Option<f32> {
        self.wrap_width
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }

    pub fn set_wrap_width(
        &mut self,
        renderer: &Renderer,
        wrap_width: Option<f32>,
    ) {
        self.wrap_width = wrap_width;

        self.recreate_text(renderer);
    }

    pub fn set_scale(&mut self, renderer: &Renderer, scale: f32) {
        self.scale = scale;

        self.text_translates = text_translates(&self.config, self.scale);
        self.recreate_text(renderer);
    }
}
