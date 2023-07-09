
use crate::{
    util::cos::Cosine,
    gui::{
        GuiNode,
        GuiSpatialContext,
        blocks::simple_gui_block::SimpleGuiBlock,
    },
};
use graphics::{
    Renderer,
    frame_content::{
        LayedOutTextBlock,
        FontId,
        TextBlock,
        TextSpan,
        HAlign,
        VAlign,
        Canvas2,
    },
};
use vek::*;


#[derive(Debug)]
pub struct GuiSplashText {
    scale_wave: Cosine,
    cache: Option<(CacheKey, LayedOutTextBlock)>,
}

#[derive(Debug)]
struct CacheKey {
    text: String,
    font: FontId,
    scale: f32,
}

impl GuiSplashText {
    pub fn new() -> Self
    {
        GuiSplashText {
            scale_wave: Cosine::new(0.5),
            cache: None,
        }
    }

    pub fn update(&mut self, elapsed: f32) {
        self.scale_wave.add_to_input(elapsed);
    }

    fn create_cache_content(
        &self,
        renderer: &Renderer,
        cache_key: &CacheKey,
    ) -> LayedOutTextBlock {
        renderer
            .lay_out_text(&TextBlock {
                spans: &[TextSpan {
                    text: &cache_key.text,
                    font: cache_key.font,
                    font_size: 16.0 * cache_key.scale,
                    color: Rgba::yellow(),
                }],
                h_align: HAlign::Center,
                v_align: VAlign::Center,
                wrap_width: None,
            })
    }

    fn validate_cache(
        &mut self,
        renderer: &Renderer,
        text: &str,
        font: FontId,
        scale: f32, 
    ) {
        let valid = self.cache
            .as_ref()
            .map(|&(ref key, _)|
                key.text == text
                && key.font == font
                && key.scale == scale
            )
            .unwrap_or(false);
        if !valid {
            let key = CacheKey {
                text: text.to_owned(),
                font,
                scale,
            };
            let content = self.create_cache_content(renderer, &key);

            self.cache = Some((key, content));
        }
    }
}

impl<'a> GuiNode<'a> for SimpleGuiBlock<&'a mut GuiSplashText> {
    fn blocks_cursor(&self, _: GuiSpatialContext) -> bool { false }

    fn draw(self, ctx: GuiSpatialContext<'a>, canvas: &mut Canvas2<'a, '_>) {
        self.inner
            .validate_cache(
                &ctx.global.renderer.borrow(),
                &ctx.assets().lang.menu_splash,
                ctx.assets().font,
                self.scale,
            );
        let &(_, ref layed_out) = self.inner.cache.as_ref().unwrap();

        canvas.reborrow()
            .translate(self.size * Vec2::new(1.0, 0.75))
            .scale(self.inner.scale_wave.get().abs() / 16.0 + 1.0)
            .rotate(f32::to_radians(22.5))
            .draw_text(layed_out);
    }
}
