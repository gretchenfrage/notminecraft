
use crate::gui::{
    GuiNode,
    GuiSpatialContext,
    GuiBlock,
    DimParentSets,
};
use super::simple_gui_block::{
    SimpleGuiBlock,
    simple_blocks_cursor_impl,
};
use graphics::frame_content::{
    GpuImageArray,
    Canvas2,
};
use vek::*;


pub fn tile_image<'a, E: Into<Extent2<f32>>>(
    image: &'a GpuImageArray,
    image_logical_size: E,
) -> impl GuiBlock<'a, DimParentSets, DimParentSets>
{
    let image_logical_size = image_logical_size.into();
    TileImage {
        image,
        image_logical_size,
    }
}


#[derive(Debug)]
struct TileImage<'a> {
    image: &'a GpuImageArray,
    image_logical_size: Extent2<f32>,
}

impl<'a> GuiNode<'a> for SimpleGuiBlock<TileImage<'a>> {
    simple_blocks_cursor_impl!();

    fn draw(self, _: GuiSpatialContext<'a>, canvas: &mut Canvas2<'a, '_>) {
        let extent = self.size / (self.inner.image_logical_size * self.scale);
        canvas.reborrow()
            .debug_tag("tile_image")
            .draw_image_uv(
                &self.inner.image,
                0,
                self.size,
                0.0,
                extent,
            );
    }
}
