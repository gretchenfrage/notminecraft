
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
    GpuImage,
    Canvas2,
};
use vek::*;


pub fn tile_image<'a, E: Into<Extent2<f32>>>(
    image: &'a GpuImage,
    image_logical_size: E,
) -> impl GuiBlock<'a, DimParentSets, DimParentSets>
{
    let image_logical_size = image_logical_size.into();
    TileImage {
        image,
        image_logical_size,
    }
}


struct TileImage<'a> {
    image: &'a GpuImage,
    image_logical_size: Extent2<f32>,
}

impl<'a> GuiNode<'a> for SimpleGuiBlock<TileImage<'a>> {
    simple_blocks_cursor_impl!();

    fn draw(self, _: GuiSpatialContext, canvas: &mut Canvas2<'a, '_>) {
        let extent = self.size / (self.inner.image_logical_size * self.scale);
        canvas.reborrow()
            .draw_image_uv(
                &self.inner.image,
                self.size,
                0.0,
                extent,
            );
    }
}
