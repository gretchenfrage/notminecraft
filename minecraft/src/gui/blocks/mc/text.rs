
use crate::gui::{
    GuiSpatialContext,
    GuiNode,
    GuiBlock,
    DimParentSets,
    DimChildSets,
    GuiGlobalContext,
};
use graphics::{
    Renderer,
    frame_content::{
        Canvas2,
        TextBlock,
        TextSpan,
        LayedOutTextBlock,
        FontId,
        HAlign,
        VAlign,
    },
};
use vek::*;


// ==== constants ====


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


// ==== config ====


#[derive(Debug)]
pub struct GuiTextBlockConfig<'a> {
    pub text: &'a str,
    pub font: FontId,
    pub logical_font_size: f32,
    pub color: Rgba<f32>,
    pub h_align: HAlign,
    pub v_align: VAlign,
}


// ==== inner ====


/// Utility for GUI blocks that displays text. Designed to cache layout. Does
/// the drop shadow.
///
/// Since text layout is an expensive operation, this GUI node is designed to
/// cache its layout so it doesn't have to recalculate unless the size or scale
/// changes. The way this is done is that `GuiBlock` is implemented not for
/// `GuiTextBlockInner` itself, but for `&mut GuiTextBlockInner`.
#[derive(Debug)]
pub struct GuiTextBlockInner {
    text: String,
    font: FontId,
    logical_font_size: f32,
    color: Rgba<f32>,
    h_align: HAlign,
    v_align: VAlign,
    wrap: bool,
    
    cache: Option<(CacheKey, LayedOutTextBlock)>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct CacheKey {
    wrap_width: Option<f32>,
    scale: f32,
}

impl GuiTextBlockInner {
    pub fn new(config: &GuiTextBlockConfig, wrap: bool) -> Self {
        GuiTextBlockInner {
            text: config.text.to_owned(),
            font: config.font,
            logical_font_size: config.logical_font_size,
            color: config.color,
            h_align: config.h_align,
            v_align: config.v_align,
            wrap: wrap,

            cache: None,
        }
    }

    fn create_cache_content(
        &self,
        renderer: &Renderer,
        cache_key: CacheKey,
    ) -> LayedOutTextBlock {
        let font_size = self.logical_font_size * cache_key.scale;

        renderer
            .lay_out_text(&TextBlock {
                spans: &[
                    TextSpan {
                        text: &self.text,
                        font: self.font,
                        font_size,
                        color: self.color,
                    },
                ],
                h_align: self.h_align,
                v_align: self.v_align,
                wrap_width: cache_key.wrap_width,
            })
    }

    fn validate_cache(&mut self, renderer: &Renderer, cache_key: CacheKey) {
        let dirty = match self.cache {
            None => true,
            Some((cached_cache_key, _)) => cached_cache_key != cache_key,
        };
        if dirty {
            let content = self.create_cache_content(renderer, cache_key);
            self.cache = Some((cache_key, content));
        }  
    }

    pub fn content_bounds(
        &mut self,
        wrap_width: Option<f32>,
        scale: f32,
        renderer: &Renderer,
    ) -> [Vec2<f32>; 2] {
        assert_eq!(self.wrap, wrap_width.is_some());
        let cache_key = CacheKey {
            wrap_width: if self.wrap { Some(wrap_width.unwrap()) } else { None },
            scale: scale,
        };
        self.validate_cache(renderer, cache_key);
        let layed_out = self.cache
            .as_ref()
            .map(|&(_, ref content)| content)
            .unwrap();
        layed_out.content_bounds()
    }

    pub fn draw<'a>(
        &'a mut self,
        size: Extent2<f32>,
        scale: f32,
        canvas: &mut Canvas2<'a, '_>,
        renderer: &Renderer,
    ) {
        let cache_key = CacheKey {
            wrap_width: Some(size.w).filter(|_| self.wrap),
            scale: scale,
        };

        self.validate_cache(renderer, cache_key);

        let layed_out = self.cache
            .as_ref()
            .map(|&(_, ref content)| content)
            .unwrap();

        let align_sign = Vec2 {
            x: self.h_align.sign(),
            y: self.v_align.sign(),
        };
        let align_translate_fractional = align_sign
            .map(|n| n as f32 / 2.0 + 0.5);
        let align_translate = align_translate_fractional * size;

        let mystery_gap_adjust_translate =
            align_translate_fractional
            * self.logical_font_size
            * scale
            * BOTTOM_RIGHT_MYSTERY_GAP;
        
        let shadow_drop = self.logical_font_size / SHADOW_DROP_DIVISOR * scale;
        let text_shadow_translate = align_sign
            .map(|n| (n as f32 / -2.0 + 0.5) * shadow_drop);
        let text_main_translate = align_sign
            .map(|n| (n as f32 / -2.0 - 0.5) * shadow_drop);

        let mut canvas = canvas.reborrow()
            .translate(align_translate)
            .translate(mystery_gap_adjust_translate);
        canvas.reborrow()
            .translate(text_shadow_translate)
            .color(SHADOW_DROP_COLOR)
            .draw_text(&layed_out);
        canvas.reborrow()
            .translate(text_main_translate)
            .draw_text(&layed_out);
    }
}


// ==== gui blocks ====


/// GUI text block. If `WRAP` is false, doesn't wrap, and sets its own width
/// and height. If `WRAP` is true, wraps based on parent set width.
#[derive(Debug)]
pub struct GuiTextBlock<const WRAP: bool>(GuiTextBlockInner);

impl<const WRAP: bool> GuiTextBlock<WRAP> {
    pub fn new(config: &GuiTextBlockConfig) -> Self {
        GuiTextBlock(GuiTextBlockInner::new(config, WRAP))
    }
}

// wrap false

impl<'a> GuiBlock<'a, DimChildSets, DimChildSets> for &'a mut GuiTextBlock<false> {
    type Sized = GuiTextBlockWrapFalseSized<'a>;

    fn size(
        self,
        ctx: &GuiGlobalContext<'a>,
        (): (),
        (): (),
        scale: f32,
    ) -> (f32, f32, Self::Sized) {
        let [min, max] = self.0.content_bounds(None, scale, &*ctx.renderer.borrow());
        let size = Extent2 {
            w: max.x - min.x,
            h: max.y - min.y,
        };
        (
            size.w,
            size.h,
            GuiTextBlockWrapFalseSized {
                inner: &mut self.0,
                size,
                scale,
            }
        )
    }
}

#[derive(Debug)]
#[doc(hidden)]
pub struct GuiTextBlockWrapFalseSized<'a> {
    inner: &'a mut GuiTextBlockInner,
    size: Extent2<f32>,
    scale: f32,
}

impl<'a> GuiNode<'a> for GuiTextBlockWrapFalseSized<'a> {
    fn blocks_cursor(&self, _: GuiSpatialContext) -> bool { false }

    fn draw(self, ctx: GuiSpatialContext<'a>, canvas: &mut Canvas2<'a, '_>) {
        self.inner.draw(
            self.size,
            self.scale,
            canvas,
            &*ctx.global.renderer.borrow(),
        );
    }
}

// wrap true

impl<'a> GuiBlock<'a, DimParentSets, DimChildSets> for &'a mut GuiTextBlock<true> {
    type Sized = GuiTextBlockWrapTrueSized<'a>;

    fn size(
        self,
        ctx: &GuiGlobalContext<'a>,
        w: f32,
        (): (),
        scale: f32,
    ) -> ((), f32, Self::Sized) {
        let [min, max] = self.0.content_bounds(Some(w), scale, &*ctx.renderer.borrow());
        let size = Extent2 {
            w,
            h: max.y - min.y,
        };
        (
            (),
            size.h,
            GuiTextBlockWrapTrueSized {
                inner: &mut self.0,
                size,
                scale,
            }
        )
    }
} 

#[derive(Debug)]
#[doc(hidden)]
pub struct GuiTextBlockWrapTrueSized<'a> {
    inner: &'a mut GuiTextBlockInner,
    size: Extent2<f32>,
    scale: f32,
}

impl<'a> GuiNode<'a> for GuiTextBlockWrapTrueSized<'a> {
    fn blocks_cursor(&self, _: GuiSpatialContext) -> bool { false }

    fn draw(self, ctx: GuiSpatialContext<'a>, canvas: &mut Canvas2<'a, '_>) {
        self.inner.draw(
            self.size,
            self.scale,
            canvas,
            &*ctx.global.renderer.borrow(),
        );
    }
}

/*
impl<'a> GuiNode<'a> for SimpleGuiBlock<&'a mut GuiTextBlockInner> {
    fn blocks_cursor(&self, _: GuiSpatialContext) -> bool { false }

    fn draw(self, ctx: GuiSpatialContext<'a>, canvas: &mut Canvas2<'a, '_>) {
        self.inner.draw(
            self.size,
            self.scale,
            canvas,
            &ctx.global.renderer.borrow(),
        );
    }
}
*/