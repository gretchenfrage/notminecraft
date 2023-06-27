
use crate::{
    item::{
        ItemStack,
        ItemInstance,
    },
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


/*
pub fn gui_slot_grid<F, E, const LEN: usize>(
    slots: &[ItemSlot; LEN],
    dims: E,
    config: F,
) -> impl GuiBlockSeq<DimChildSets, DimParentSets>
where
    F: FnMut() -> SlotGuiConfig,
    E: Into<Extent2<u32>>,
{
    let dims = dims.into();
    slots
}*/
//pub struct ItemGridBuilder<


pub const DEFAULT_SLOT_SIZE: f32 = 36.0;


#[derive(Debug)]
pub struct ItemSlot(pub RefCell<Option<ItemStack>>);

impl ItemSlot {
    pub fn gui<'a>(
        &'a self,
        config: SlotGuiConfig,
    ) -> impl GuiBlock<'a, DimChildSets, DimChildSets> {
        ItemSlotGuiBlock {
            inner: self,
            size: config.logical_size,
            interactable: config.interactable,
        }
    }
}

impl Default for ItemSlot {
    fn default() -> Self {
        ItemSlot(RefCell::new(None))
    }
}


#[derive(Debug, Clone)]
pub struct SlotGuiConfig {
    pub logical_size: f32,
    pub interactable: bool,
}

impl SlotGuiConfig {
    pub fn new() -> Self {
        SlotGuiConfig {
            logical_size: DEFAULT_SLOT_SIZE,
            interactable: true,
        }
    }

    pub fn non_interactable(mut self) -> Self {
        self.interactable = false;
        self
    }
}

impl Default for SlotGuiConfig {
    fn default() -> Self {
        SlotGuiConfig::new()
    }
}


#[derive(Debug)]
struct ItemSlotGuiBlock<'a> {
    inner: &'a ItemSlot,
    size: f32,
    interactable: bool,
}

impl<'a> GuiBlock<'a, DimChildSets, DimChildSets> for ItemSlotGuiBlock<'a> {
    type Sized = Self;

    fn size(
        mut self,
        _: &GuiGlobalContext<'a>,
        _w_in: (),
        _h_in: (),
        scale: f32,
    ) -> (f32, f32, Self::Sized) {
        self.size *= scale;
        (self.size, self.size, self)
    }
}

impl<'a> GuiNode<'a> for ItemSlotGuiBlock<'a> {
    fn blocks_cursor(&self, ctx: GuiSpatialContext<'a>) -> bool {
        ctx.cursor_in_area(0.0, self.size)
    }

    fn draw(self, ctx: GuiSpatialContext<'a>, canvas: &mut Canvas2<'a ,'_>) {
        let view_proj = Mat4::new(
            1.0,  0.0,  0.0, 0.5,
            0.0, -1.0,  0.0, 0.5,
            0.0,  0.0, 0.01, 0.5,
            0.0,  0.0,  0.0, 1.0,
        );
        if let Some(stack) = self.inner.0.borrow().as_ref() {
            let imi = *ctx.game().items_mesh_index.get(stack.item.iid);
            canvas.reborrow()
                .scale(self.size)
                .begin_3d(view_proj)
                .draw_mesh(
                    &ctx.assets().item_meshes[imi],
                    &ctx.assets().blocks,
                );
        }
        //canvas.reborrow()
        //    .scale(size)
        //    .begin_3d(view_proj)
        //    .scale(0.5)
        //    .rotate(Quaternion::rotation_x(-PI / 5.0))
        //    .rotate(Quaternion::rotation_y(PI / 4.0))
        //    .translate(-0.5)
        //    .draw_mesh(
        //        &ctx.assets().block_item_mesh,
        //        &ctx.assets().blocks,
        //    );
        if self.interactable {
            if ctx.cursor_in_area(0.0, self.size) {
                canvas.reborrow()
                    .color([0.0, 0.0, 0.0, 0.25])
                    .draw_solid(self.size);
            }
        }
    }

    fn on_cursor_click(
        self,
        ctx: GuiSpatialContext,
        hits: bool,
        button: MouseButton,
    ) {
        if !hits { return }
        if !ctx.cursor_in_area(0.0, self.size) { return }
        if button != MouseButton::Middle { return }

        let mut slot = self.inner.0.borrow_mut();
        *slot = Some(ItemStack::one(ItemInstance::new(ctx.game().iid_stone, ())));
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
