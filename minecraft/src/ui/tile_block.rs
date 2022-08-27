
use super::{
    False,
    UiBlock,
    UiBlockSetWidth,
    UiBlockSetHeight,
};
use graphics::{
    Renderer,
    frame_content::{
        Canvas2,
        GpuImage,
    },
};
use vek::*;


#[derive(Debug, Clone)]
pub struct UiTileBlockConfig {
    pub image: GpuImage,
    pub size_unscaled_untiled: Extent2<f32>,
    pub color: Rgba<f32>,
}

#[derive(Debug, Clone)]
pub struct UiTileBlock {
    config: UiTileBlockConfig,

    size: Extent2<f32>,
    scale: f32,

    tex_extent: Extent2<f32>,
}


fn tex_extent(
    size_unscaled_untiled: Extent2<f32>,
    size: Extent2<f32>,
    scale: f32,
) -> Extent2<f32>
{
    size / (size_unscaled_untiled * scale)
}

impl UiTileBlock {
    pub fn new(
        config: UiTileBlockConfig,
        size: Extent2<f32>,
        scale: f32,
    ) -> Self
    {
        let tex_extent = tex_extent(config.size_unscaled_untiled, size, scale);
        UiTileBlock {
            config,
            size,
            scale,
            tex_extent,
        }
    }
}

impl UiBlock for UiTileBlock {
    type WidthChanged = False;
    type HeightChanged = False;

    fn draw<'a>(&'a self, mut canvas: Canvas2<'a, '_>) {
        canvas.reborrow()
            .color(self.config.color)
            .draw_image_uv(
                &self.config.image,
                self.size,
                [0.0, 0.0],
                self.tex_extent,
            );
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

    fn set_scale(&mut self, _: &Renderer, scale: f32) -> (
        Self::WidthChanged,
        Self::HeightChanged,
    )
    {
        self.scale = scale;

        self.tex_extent = tex_extent(
            self.config.size_unscaled_untiled,
            self.size,
            self.scale,
        );

        (False, False)
    }
}

impl UiBlockSetWidth for UiTileBlock {
    fn set_width(&mut self, _: &Renderer, width: f32) {
        self.size.w = width;

        self.tex_extent = tex_extent(
            self.config.size_unscaled_untiled,
            self.size,
            self.scale,
        );
    }
}

impl UiBlockSetHeight for UiTileBlock {
    fn set_height(&mut self, _: &Renderer, height: f32) {
        self.size.h = height;

        self.tex_extent = tex_extent(
            self.config.size_unscaled_untiled,
            self.size,
            self.scale,
        );
    }
}
