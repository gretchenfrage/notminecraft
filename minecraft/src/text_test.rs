
use crate::{
    asset::Assets,
    gui::{
        *,
        blocks::{
            *,
            simple_gui_block::SimpleGuiBlock,
        },
    },
    util::hex_color::hex_color,
};
use graphics::{
    Renderer,
    frame_content::*,
};
use vek::*;


#[derive(Debug)]
pub struct TextTest {
    text: [[LayedOutTextBlock; 3]; 3],
}

fn make(
    renderer: &Renderer,
    assets: &Assets,
    h_align: HAlign,
    v_align: VAlign,
) -> LayedOutTextBlock {
    //let font = renderer.load_
    renderer.lay_out_text(&TextBlock {
            spans: &[
                TextSpan {
                    text: "Hello World! Goodbye World!",
                    font: assets.font,
                    font_size: 48.0,
                    color: Rgba::black(),
                },
            ],
            h_align,
            v_align,
            wrap_width: Some(300.0),
        })
}

impl TextTest {
    pub fn new(
        renderer: &Renderer,
        assets: &Assets,
    ) -> Self
    {
        TextTest {
            text: [
                [
                    make(renderer, assets, HAlign::Right, VAlign::Bottom),
                    make(renderer, assets, HAlign::Center, VAlign::Bottom),
                    make(renderer, assets, HAlign::Left, VAlign::Bottom),
                ],
                [
                    make(renderer, assets, HAlign::Right, VAlign::Center),
                    make(renderer, assets, HAlign::Center, VAlign::Center),
                    make(renderer, assets, HAlign::Left, VAlign::Center),
                ],
                [
                    make(renderer, assets, HAlign::Right, VAlign::Top),
                    make(renderer, assets, HAlign::Center, VAlign::Top),
                    make(renderer, assets, HAlign::Left, VAlign::Top),
                ],
            ],
        }
    }

    fn gui<'a>(
        &'a mut self,
        _: &'a GuiWindowContext,
    ) -> impl GuiBlock<'a, DimParentSets, DimParentSets>
    {
        self
    }
}

impl GuiStateFrame for TextTest {
    impl_visit_nodes!();
}

impl<'a> GuiNode<'a> for SimpleGuiBlock<&'a mut TextTest> {
    fn blocks_cursor(&self, ctx: GuiSpatialContext<'a>) -> bool { false }

    fn draw(
        self,
        ctx: GuiSpatialContext<'a>,
        canvas: &mut Canvas2<'a, '_>,
    ) {
        for v in 0..3 {
            for h in 0..3 {
                if v % 2 != h % 2 {
                    canvas.reborrow()
                        .translate(Vec2::new(h as f32, v as f32) / 3.0 * self.size)
                        .color([0.8, 0.8, 0.8, 1.0])
                        .draw_solid(self.size / 3.0);
                }
            }
        }

        for (v, row) in self.inner.text.iter().enumerate() {
            for (h, text) in row.iter().enumerate() {
                let transl =
                    self.size
                    * Vec2::new(h, v).map(|n| match n {
                        0 => 1.0 / 3.0,
                        1 => 1.0 / 2.0,
                        2 => 2.0 / 3.0,
                        _ => unreachable!(),
                    });
                canvas.reborrow()
                    .translate(transl)
                    .draw_text(text);
                for (i, glyph_pos) in text.content_bounds().into_iter().enumerate() {
                    canvas.reborrow()
                        .translate(transl)
                        .translate(glyph_pos)
                        .translate(-1.0)
                        .color(match i % 2 == 0 {
                            true => Rgba::red(),
                            false => Rgba::green(),
                        })
                        .draw_solid(2.0);
                }
            }
        }
    }
} 
