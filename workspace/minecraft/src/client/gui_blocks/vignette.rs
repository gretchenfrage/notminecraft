
use crate::gui::prelude::*;
use graphics::prelude::*;


/// GUI block for rendering the vignette.
#[derive(Debug)]
pub struct Vignette;

impl<'a> GuiNode<'a> for SimpleGuiBlock<Vignette> {
    never_blocks_cursor_impl!();

    fn draw(self, ctx: GuiSpatialContext, canvas: &mut Canvas2) {
        if !ctx.global.pressed_keys_semantic.contains(&VirtualKeyCode::F1) {
            canvas.reborrow()
                .color([1.0, 1.0, 1.0, 1.0 - 0x5b as f32 / 0x7f as f32])
                .draw_image(
                    &ctx.assets().vignette,
                    0,
                    self.size,
                );
        }
    }
}
