use super::*;
    use crate::game::Cosine;
    use graphics::frame_content::{
        TextBlock,
        TextSpan,
        LayedOutTextBlock,
        FontId,
        HAlign,
        VAlign,
    };


    pub struct McSplashTextGuiBlock {
        layed_out: LayedOutTextBlock,
        bounce_scale: Cosine,
        translate_frac: Vec2<f32>,
    }

    impl McSplashTextGuiBlock {
        pub fn new(renderer: &Renderer, font: FontId) -> Self {
            let text = "Splash text!";
            let translate_frac = Vec2 {
                x: 0.75,
                y: 5.0 / 16.0,
            };

            let layed_out = renderer.lay_out_text(&TextBlock {
                spans: &[TextSpan {
                    text,
                    font,
                    font_size: 32.0, // TODO we could revalidate upon scale change and maybe should at some point but where we're doing all sorts of constant things that mess of pixel perfectness like rotation or 3D and not re-rasterizing each time then this is largely fine for now but this should be considered a TODO item
                    color: Rgba::yellow(),
                }],
                h_align: HAlign::Center,
                v_align: VAlign::Center,
                wrap_width: None,
            });
            let bounce_scale = Cosine::new(1.0 / 2.0);

            McSplashTextGuiBlock {
                layed_out,
                bounce_scale,
                translate_frac,
            }
        }

        pub fn update(&mut self, elapsed: f32) {
            self.bounce_scale.add_to_input(elapsed);
        }
    }

    impl<'a> GuiBlock<'a, DimParentSets, DimParentSets> for &'a McSplashTextGuiBlock {
        type Sized = McSplashTextSizedGuiBlock<'a>;

        fn size(self, w: f32, h: f32, scale: f32) -> ((), (), Self::Sized) {
            let sized = McSplashTextSizedGuiBlock {
                block: self,
                size: Extent2 { w, h },
                scale,
            };
            ((), (), sized)
        }
    }

    pub struct McSplashTextSizedGuiBlock<'a> {
        block: &'a McSplashTextGuiBlock,
        size: Extent2<f32>,
        scale: f32,
    }

    impl<'a> GuiNode<'a> for McSplashTextSizedGuiBlock<'a> {
        fn draw(self, _: &Renderer, mut canvas: Canvas2<'a, '_>) {
            canvas.reborrow()
                .translate(self.size * self.block.translate_frac)
                .scale(self.scale)
                .scale(self.block.bounce_scale.get().abs() / 16.0 + 1.0)
                .rotate(f32::to_radians(22.5))
                .draw_text(&self.block.layed_out);
        }
    }