
use super::{
    False,
    UiBlock,
    UiBlockSetWidth,
    UiBlockSetHeight,
};
use graphics::{
    Renderer,
    frame_content::Canvas2,
};
use vek::*;


#[derive(Debug, Clone)]
pub struct UiHMarginBlockConfig {
    pub margin_left: f32,
    pub margin_right: f32,
}

#[derive(Debug, Clone)]
pub struct UiHMarginBlock<I> {
    config: UiHMarginBlockConfig,

    size: Extent2<f32>,
    scale: f32,
    
    inner_x_translate: f32,
    pub inner: I,
}

fn inner_width(
    config: &UiHMarginBlockConfig,
    size: Extent2<f32>,
    scale: f32,
) -> f32
{
    size.w - (config.margin_left + config.margin_right) * scale
}

impl<I> UiHMarginBlock<I> {
    pub fn new<F>(
        config: UiHMarginBlockConfig,
        create_inner: F,
        size: Extent2<f32>,
        scale: f32,
    ) -> Self
    where
        F: FnOnce(Extent2<f32>, f32) -> I,
    {
        let inner_size = Extent2 {
            w: inner_width(&config, size, scale),
            h: size.h,
        };
        let inner = create_inner(inner_size, scale);
        let inner_x_translate = config.margin_left * scale;
        
        UiHMarginBlock {
            config,

            size,
            scale,

            inner_x_translate,
            inner,
        }
    }
}

impl<I> UiBlock for UiHMarginBlock<I>
where
    I: UiBlock<WidthChanged=False> + UiBlockSetWidth,
{
    type WidthChanged = False;
    type HeightChanged = I::HeightChanged;

    fn draw<'a>(&'a self, mut canvas: Canvas2<'a, '_>) {
        self.inner.draw(canvas.reborrow()
            .translate([self.inner_x_translate, 0.0]));
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

        self.inner_x_translate = self.config.margin_left * self.scale;

        let (False, height_changed) = self.inner.set_scale(renderer, scale);

        let inner_width = inner_width(&self.config, self.size, self.scale);
        self.inner.set_width(renderer, inner_width);

        (False, height_changed)
    }
}

impl<I> UiBlockSetWidth for UiHMarginBlock<I>
where
    I: UiBlockSetWidth,
{
    fn set_width(&mut self, renderer: &Renderer, width: f32) {
        self.size.w = width;

        let inner_width = inner_width(&self.config, self.size, self.scale);
        self.inner.set_width(renderer, inner_width)
    }
}

impl<I> UiBlockSetHeight for UiHMarginBlock<I>
where
    I: UiBlockSetHeight,
{
    fn set_height(&mut self, renderer: &Renderer, height: f32) {
        self.size.h = height;
        self.inner.set_height(renderer, height);
    }
}


// ==== TODO: dedupe with macros, or traits ====

#[derive(Debug, Clone)]
pub struct UiVMarginBlockConfig {
    pub margin_top: f32,
    pub margin_bottom: f32,
}

#[derive(Debug, Clone)]
pub struct UiVMarginBlock<I> {
    config: UiVMarginBlockConfig,

    size: Extent2<f32>,
    scale: f32,
    
    inner_y_translate: f32,
    pub inner: I,
}

fn inner_height(
    config: &UiVMarginBlockConfig,
    size: Extent2<f32>,
    scale: f32,
) -> f32
{
    size.h - (config.margin_top + config.margin_bottom) * scale
}

impl<I> UiVMarginBlock<I> {
    pub fn new<F>(
        config: UiVMarginBlockConfig,
        create_inner: F,
        size: Extent2<f32>,
        scale: f32,
    ) -> Self
    where
        F: FnOnce(Extent2<f32>, f32) -> I,
    {
        let inner_size = Extent2 {
            w: size.w,
            h: inner_height(&config, size, scale),
        };
        let inner = create_inner(inner_size, scale);
        let inner_y_translate = config.margin_top * scale;
        
        UiVMarginBlock {
            config,

            size,
            scale,

            inner_y_translate,
            inner,
        }
    }
}

impl<I> UiBlock for UiVMarginBlock<I>
where
    I: UiBlock<HeightChanged=False> + UiBlockSetHeight,
{
    type WidthChanged = I::WidthChanged;
    type HeightChanged = False;

    fn draw<'a>(&'a self, mut canvas: Canvas2<'a, '_>) {
        self.inner.draw(canvas.reborrow()
            .translate([0.0, self.inner_y_translate]));
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

        self.inner_y_translate = self.config.margin_top * self.scale;

        let (width_changed, False) = self.inner.set_scale(renderer, scale);

        let inner_height = inner_height(&self.config, self.size, self.scale);
        self.inner.set_height(renderer, inner_height);

        (width_changed, False)
    }
}

impl<I> UiBlockSetWidth for UiVMarginBlock<I>
where
    I: UiBlockSetWidth,
{
    fn set_width(&mut self, renderer: &Renderer, width: f32) {
        self.size.w = width;
        self.inner.set_width(renderer, width);
    }
}

impl<I> UiBlockSetHeight for UiVMarginBlock<I>
where
    I: UiBlockSetHeight,
{
    fn set_height(&mut self, renderer: &Renderer, height: f32) {
        self.size.h = height;

        let inner_height = inner_height(&self.config, self.size, self.scale);
        self.inner.set_height(renderer, inner_height)
    }
}
