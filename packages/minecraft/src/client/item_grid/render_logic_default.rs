
use super::*;
use crate::{
    game_data::per_item::PerItem,
    item::*,
};
use std::f32::consts::PI;


/// Reasonable-defaults `ItemGridRenderLogic` implementation.
#[derive(Debug)]
pub struct ItemGridDefaultRenderLogic<'a> {
    pub item_mesh: &'a PerItem<Mesh>,
}

impl<'a> ItemGridRenderLogic<'a, Option<ItemStack>> for ItemGridDefaultRenderLogic<'a> {
    fn draw(
        &mut self,
        ctx: GuiSpatialContext<'a>,
        canvas: &mut Canvas2<'a, '_>,
        item_slot_idx: usize,
        item_slot: &'a Option<ItemStack>,
        size: f32,
        scale: f32,
        is_cursor_over: bool,
    ) {
        if let Some(stack) = item_slot.as_ref() {
            canvas.reborrow()
                .translate(size / 2.0)
                .scale(size * 1.1)
                .translate(-0.5)
                .begin_3d(
                    Mat4::new(
                        1.0,  0.0,  0.0, 0.5,
                        0.0, -1.0,  0.0, 0.5,
                        0.0,  0.0, 0.01, 0.5,
                        0.0,  0.0,  0.0, 1.0,
                    ),
                    Fog::None,
                )
                .scale(0.56)
                .rotate(Quaternion::rotation_x(-PI * 0.17))
                .rotate(Quaternion::rotation_y(PI / 4.0))
                .translate(-0.5)
                .draw_mesh(
                    &self.item_mesh[stack.iid],
                    &ctx.assets().blocks,
                );
        }
        if is_cursor_over {
            const SELECTED_ALPHA: f32 = (0xc5 as f32 - 0x8b as f32) / (0xff as f32 - 0x8b as f32);
            canvas.reborrow()
                .color([1.0, 1.0, 1.0, SELECTED_ALPHA])
                .draw_solid(size);
        }
    }
}
