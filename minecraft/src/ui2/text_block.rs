
use super::{
    False,
    UiBlock,
    UiBlockSetWidth,
    UiBlockSetHeight,
    text::{
        UiText,
        UiTextConfig,
    },
};
use graphics::{
    Renderer,
    frame_content::{
        Canvas2,
        FontId,
        HAlign,
        VAlign,
    },
};
use vek::*;


#[derive(Debug, Clone)]
pub struct UiTextBlockConfig {
    pub text: String,
    pub font: FontId,
    pub font_size: f32,
    pub color: Rgba<f32>,
    pub h_align: HAlign,
    pub v_align: VAlign,
    pub wrap: bool,
}

/// UI block with settable size containing text. Uses `UiText`, which provides
/// things like consistent pre-scaling font size, drop shadow, and
/// vertical/horizontal alignment.
#[derive(Debug, Clone)]
pub struct UiTextBlock {
    config: UiTextBlockConfig,
    
    size: Extent2<f32>,
    scale: f32,

    inner_translate: Vec2<f32>,
    inner: UiText,
}


fn inner_translate(
    config: &UiTextBlockConfig,
    size: Extent2<f32>,
) -> Vec2<f32> {
    Vec2 {
        x: config.h_align.sign(),
        y: config.v_align.sign(),
    }
        .map(|n| n as f32 / 2.0 + 0.5)
        * size
}

impl UiTextBlock {
    pub fn new(
        renderer: &Renderer,
        config: UiTextBlockConfig,
        size: Extent2<f32>,
        scale: f32,
    ) -> Self {
        let inner = UiText::new(
            renderer,
            UiTextConfig {
                text: config.text.clone(),
                font: config.font,
                font_size: config.font_size,
                color: config.color,
                h_align: config.h_align,
                v_align: config.v_align,
            },
            if config.wrap { Some(size.w) } else { None },
            scale,
        );
        let inner_translate = inner_translate(&config, size);
        UiTextBlock {
            config,

            size,
            scale,

            inner_translate,
            inner,
        }
    }
}

impl UiBlock for UiTextBlock {
    type WidthChanged = False;
    type HeightChanged = False;

    fn draw<'a>(&'a self, mut canvas: Canvas2<'a, '_>) {
        self.inner.draw(canvas.reborrow()
            .translate(self.inner_translate));
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

        self.inner.set_scale(renderer, scale);
        
        (False, False)
    }
}

impl UiBlockSetWidth for UiTextBlock {
    fn set_width(&mut self, renderer: &Renderer, width: f32) {
        self.size.w = width;

        self.inner_translate = inner_translate(&self.config, self.size);
        if self.config.wrap {
            self.inner.set_wrap_width(renderer, Some(self.size.w));
        }
    }
}

impl UiBlockSetHeight for UiTextBlock {
    fn set_height(&mut self, _: &Renderer, height: f32) {
        self.size.h = height;

        self.inner_translate = inner_translate(&self.config, self.size);
    }
}
