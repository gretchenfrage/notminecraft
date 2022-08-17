
use super::{
    text::{
        UiTextBlock,
        UiTextBlockConfig,
        UiTextConfig,
    },
    tile_9::{
        UiTile9,
        Tile9PxRanges,
    },
    Margins,
    UiSize,
    UiPosInputEvent,
};
use graphics::{
    Renderer,
    frame_content::{
        Canvas2,
        FontId,
        GpuImage,
        HAlign,
        VAlign,
    },
};
use image::DynamicImage;
use vek::*;


#[derive(Debug)]
pub struct UiMenuButton {
    text: UiTextBlock,
    background: UiTile9,
    background_highlight: UiTile9,
    highlight: bool,
    unscaled_height: f32,
    size: UiSize,
}

#[derive(Debug, Clone)]
pub struct UiMenuButtonConfig {
    pub text: String,
    pub font: FontId,
    pub font_size: f32,
    pub text_color: Rgba<f32>,
    pub texture: DynamicImage,
    pub texture_scale: f32,
    pub tile_9_px_ranges: Tile9PxRanges,
    pub tile_9_px_ranges_highlight: Tile9PxRanges,
    pub unscaled_height: f32,
}

impl UiMenuButton {
    pub fn new(
        renderer: &Renderer,
        config: UiMenuButtonConfig,
        width: f32,
        scale: f32,
    ) -> Self {
        let size = UiSize {
            size: [
                width,
                config.unscaled_height * scale,
            ].into(),
            scale,
        };
        let text = UiTextBlock::new(
            renderer,
            UiTextBlockConfig {
                text_config: UiTextConfig {
                    text: config.text,
                    font: config.font,
                    font_size: config.font_size,
                    color: config.text_color,
                    h_align: HAlign::Center,
                    v_align: VAlign::Center,
                },
                margins: Margins {
                    top: 0.0,
                    bottom: 0.0,
                    left: 0.0,
                    right: 0.0, // TODO redundant in this scenario it seems
                },
                wrap: false,
            },
            size,
        );
        let background = UiTile9::new(
            renderer,
            config.texture.clone(),
            config.tile_9_px_ranges,
            config.texture_scale,
            size,
        );
        let background_highlight = UiTile9::new(
            renderer,
            config.texture,
            config.tile_9_px_ranges_highlight,
            config.texture_scale,
            size,
        );
        UiMenuButton {
            text,
            background,
            background_highlight,
            highlight: false,
            unscaled_height: config.unscaled_height,
            size,
        }
    }

    pub fn size(&self) -> UiSize {
        self.size
    }

    pub fn draw<'a>(&'a self, mut canvas: Canvas2<'a, '_>) {
        match self.highlight {
            false => &self.background,
            true => &self.background_highlight,
        }.draw(canvas.reborrow());
        self.text.draw(canvas.reborrow());
    }

    pub fn set_width(&mut self, renderer: &Renderer, width: f32) {
        self.size.size.w = width;

        self.background.set_size(self.size.size);
        self.background_highlight.set_size(self.size.size);
        self.text.set_size(renderer, self.size.size);
    }

    pub fn set_scale(&mut self, renderer: &Renderer, scale: f32) {
        self.size.scale = scale;

        self.background.set_scale(self.size.scale);
        self.background_highlight.set_scale(self.size.scale);
        self.text.set_scale(renderer, self.size.scale);

        self.size.size.h = self.unscaled_height * self.size.scale;

        self.background.set_size(self.size.size);
        self.background_highlight.set_size(self.size.size);
        self.text.set_size(renderer, self.size.size);
    }

    pub fn on_pos_input_event(&mut self, event: UiPosInputEvent) {
        match event {
            UiPosInputEvent::CursorMoved(pos) => {
                let over =
                    pos.x >= 0.0
                    && pos.y >= 0.0
                    && pos.x <= self.size.size.w
                    && pos.y <= self.size.size.h;
                if over {
                    //debug!("over !");
                }
                self.highlight = over;
            }
            _ => (),
        }
    }
}
