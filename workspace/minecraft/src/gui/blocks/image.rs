
use crate::gui::{
    GuiNode,
    GuiSpatialContext,
};
use super::simple_gui_block::{
    SimpleGuiBlock,
    simple_blocks_cursor_impl,
};
use graphics::frame_content::{
    GpuImageArray,
    Canvas2,
};

// instead of creating a wrapper, we'll just make `GpuBlock` be implemented for
// `&GpuImageArray` directly!

impl<'a> GuiNode<'a> for SimpleGuiBlock<&'a GpuImageArray> {
    simple_blocks_cursor_impl!();

    fn draw(self, _: GuiSpatialContext<'a>, canvas: &mut Canvas2<'a, '_>) {
        canvas.reborrow()
            .debug_tag("GpuImageArray")
            .draw_image_uv(
                self.inner,
                0,
                self.size,
                0.0,
                1.0,
            );
    }
}
