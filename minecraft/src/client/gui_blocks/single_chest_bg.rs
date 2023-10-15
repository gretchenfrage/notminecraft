
use crate::gui::prelude::*;
use graphics::prelude::*;
use vek::*;


#[derive(Debug, Clone)]
pub struct SingleChestBg;

impl<'a> GuiNode<'a> for SimpleGuiBlock<SingleChestBg> {
    simple_blocks_cursor_impl!();

    fn draw(self, ctx: GuiSpatialContext, canvas: &mut Canvas2) {
        canvas.reborrow()
            .draw_image_uv(
                &ctx.assets().gui_chest,
                0,
                Extent2 {
                    w: self.size.w,
                    h: self.size.h * 71.0 / 168.0,
                },
                0.0,
                Extent2 { 
                    w: 1.0,
                    h: 71.0 / 222.0,
                }
            );
        canvas.reborrow()
            .translate(Vec2 {
                x: 0.0,
                y: self.size.h * 71.0 / 168.0,
            })
            .draw_image_uv(
                &ctx.assets().gui_chest,
                0,
                Extent2 {
                    w: self.size.w,
                    h: self.size.h * 97.0 / 168.0,
                },
                Vec2 {
                    x: 0.0,
                    y: 125.0 / 222.0,
                },
                Extent2 {
                    w: 1.0,
                    h: 97.0 / 222.0,
                },
            );
    }
}
