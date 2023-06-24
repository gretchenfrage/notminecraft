
use crate::{
    item::ItemStack,
    gui::{
        blocks::*,
        *,
    },
};
use graphics::frame_content::Canvas2;
use std::{
    cell::RefCell,
    f32::consts::PI,
};
use vek::*;


pub const DEFAULT_SLOT_SIZE: f32 = 36.0;

#[derive(Debug)]
pub struct ItemSlot {
    pub content: RefCell<Option<ItemStack>>,
    /// Makes the slot larger, but not the item model.
    pub slot_scale: f32,
}

impl Default for ItemSlot {
    fn default() -> Self {
        ItemSlot {
            content: RefCell::new(None),
            slot_scale: 1.0,
        }
    }
}

impl ItemSlot {
    pub fn gui<'a>(&'a self) -> impl GuiBlock<'a, DimChildSets, DimChildSets> {
        ItemSlotGuiBlock { inner: self }
    }
}


#[derive(Debug)]
struct ItemSlotGuiBlock<'a> {
    inner: &'a ItemSlot,
}

#[derive(Debug)]
struct ItemSlotSizedGuiBlock<'a> {
    inner: &'a ItemSlot,
    ui_scale: f32,
}

impl<'a> GuiBlock<'a, DimChildSets, DimChildSets> for ItemSlotGuiBlock<'a> {
    type Sized = ItemSlotSizedGuiBlock<'a>;

    fn size(
        self,
        _: &GuiGlobalContext<'a>,
        _w_in: (),
        _h_in: (),
        ui_scale: f32,
    ) -> (f32, f32, Self::Sized) {
        let size = DEFAULT_SLOT_SIZE * ui_scale * self.inner.slot_scale;
        (size, size, ItemSlotSizedGuiBlock {
            inner: self.inner,
            ui_scale,
        })
    }
}

impl<'a> GuiNode<'a> for ItemSlotSizedGuiBlock<'a> {
    fn blocks_cursor(&self, ctx: GuiSpatialContext<'a>) -> bool {
        let size = DEFAULT_SLOT_SIZE * self.ui_scale * self.inner.slot_scale;
        ctx.cursor_in_area(0.0, size)
    }

    fn draw(self, ctx: GuiSpatialContext<'a>, canvas: &mut Canvas2<'a ,'_>) {
        let size = DEFAULT_SLOT_SIZE * self.ui_scale * self.inner.slot_scale;
        let view_proj = Mat4::new(
            1.0,  0.0,  0.0, 0.5,
            0.0, -1.0,  0.0, 0.5,
            0.0,  0.0, 0.01, 0.5,
            0.0,  0.0,  0.0, 1.0,
        );
        canvas.reborrow()
            .scale(size)
            .begin_3d(view_proj)
            .scale(0.5)
            .rotate(Quaternion::rotation_x(-PI / 5.0))
            .rotate(Quaternion::rotation_y(PI / 4.0))
            .translate(-0.5)
            .draw_mesh(
                &ctx.assets().block_item_mesh,
                &ctx.assets().blocks,
            );
        if ctx.cursor_in_area(0.0, size) {
            canvas.reborrow()
                .color([0.0, 0.0, 0.0, 0.25])
                .draw_solid(size);
        }
    }
}

/*
impl ItemSlot {
    pub fn gui<'a>(&'a mut self) -> impl GuiBlock<'a, DimChildSets, DimChildSets> {
        logical_size(DEFAULT_UI_SIZE,
            
        )
    }
}
*/

//impl<'a> GuiBlock<'a, DimChildSets, DimChildSets> {
//    
//}

/*
pub struct HeldItem {
    pub content: RefCell<Option<ItemStack>>,
}
*/
