use super::*;
    use graphics::frame_content::{
        TextBlock,
        TextSpan,
        LayedOutTextBlock,
        FontId,
        HAlign,
        VAlign,
    };


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


    pub struct TextGuiBlock {
        spans: Vec<TextGuiBlockSpan>,
        unscaled_font_size: f32,
        h_align: HAlign,
        v_align: VAlign,
        wrap: bool,

        cached: Option<TextGuiBlockCached>,
    }

    impl TextGuiBlock {
        pub fn new(spans: Vec<TextGuiBlockSpan>, unscaled_font_size: f32, h_align: HAlign, v_align: VAlign, wrap: bool) -> Self {
            TextGuiBlock {
                spans,
                unscaled_font_size,
                h_align,
                v_align,
                wrap,

                cached: None,
            }
        }
    }

    pub struct TextGuiBlockSpan {
        pub text: String,
        pub font: FontId,
        //pub unscaled_font_size: f32, TODO what we need is to sort of just take manual control of text block border logic
        pub color: Rgba<f32>,
    }

    struct TextGuiBlockCached {
        scale: f32,
        wrap_width: Option<f32>,
        layed_out: LayedOutTextBlock,
    }

    impl<'a> GuiBlock<'a, DimParentSets, DimParentSets> for &'a mut TextGuiBlock {
        type Sized = TextSizedGuiBlock<'a>;

        fn size(self, w: f32, h: f32, scale: f32) -> ((), (), Self::Sized) {
            let sized = TextSizedGuiBlock {
                block: self,
                size: Extent2 { w, h },
                scale,
            }; // TODO factor out this type of thing
            ((), (), sized)
        }
    }

    pub struct TextSizedGuiBlock<'a> {
        block: &'a mut TextGuiBlock,
        size: Extent2<f32>,
        scale: f32,
    }

    impl<'a> GuiNode<'a> for TextSizedGuiBlock<'a> { // TODO hey hold on, does this reference even have to be 'a?
        fn draw(mut self, renderer: &Renderer, mut canvas: Canvas2<'a, '_>) {
            let wrap_width =
                if self.block.wrap { Some(self.size.w) }
                else { None };

            if let &Some(ref cached) = &self.block.cached {
                if cached.wrap_width != wrap_width || cached.scale != self.scale {
                    self.block.cached = None;
                }
            }

            if self.block.cached.is_none() {
                self.block.cached = Some(TextGuiBlockCached {
                    scale: self.scale,
                    wrap_width,
                    layed_out: renderer.lay_out_text(&TextBlock {
                        spans: &self.block.spans.iter()
                            .map(|span| TextSpan {
                                text: &span.text,
                                font: span.font,
                                font_size: self.block.unscaled_font_size * self.scale,
                                color: span.color,
                            })
                            .collect::<Vec<_>>(),
                        h_align: self.block.h_align,
                        v_align: self.block.v_align,
                        wrap_width,
                    }),
                });
            }

            let layed_out = &self.block.cached.as_ref().unwrap().layed_out;

            let align_sign = Vec2 {
                x: self.block.h_align.sign(),
                y: self.block.v_align.sign(),
            };
            let align_translate_fractional = align_sign
                .map(|n| n as f32 / 2.0 + 0.5);
            let align_translate = align_translate_fractional * self.size;

            let mystery_gap_adjust_translate =
                align_translate_fractional
                * self.block.unscaled_font_size
                * self.scale
                * BOTTOM_RIGHT_MYSTERY_GAP;
            
            let shadow_drop = self.block.unscaled_font_size / SHADOW_DROP_DIVISOR * self.scale;
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