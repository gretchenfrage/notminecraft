
mod layout_calcs;
pub mod borrow_item_slot;
pub mod item_slot_click_logic;
pub mod item_slot_gui_state;


use self::{
    borrow_item_slot::BorrowItemSlot,
    item_slot_click_logic::ItemSlotClickLogic,
    item_slot_gui_state::{
        ItemSlotGuiStateGeneral,
        ItemSlotGuiStateNoninteractive,
        draw_item_noninteractive,
    },
    layout_calcs::{
        ItemGridLayoutCalcs,
        ItemSlotLayoutCalcs,
    },
};
use crate::{
    gui::prelude::*,
    client::meshing::item_mesh::ItemMesh,
    game_data::per_item::PerItem,
};
use graphics::prelude::*;
use std::fmt::Debug;
use vek::*;


#[derive(Debug)]
pub struct HeldItemGuiBlock<'a, H> {
    pub held: H,
    pub held_state: &'a mut ItemSlotGuiStateNoninteractive,
    pub items_mesh: &'a PerItem<ItemMesh>,
}

#[derive(Debug)]
#[doc(hidden)]
pub struct HeldItemGuiBlockSized<'a, H> {
    inner: HeldItemGuiBlock<'a, H>,
    scale: f32,
}

impl<
    'a,
    H: BorrowItemSlot + Debug,
> GuiBlock<'a, DimParentSets, DimParentSets> for HeldItemGuiBlock<'a, H> {
    type Sized = HeldItemGuiBlockSized<'a, H>;

    fn size(self, _: &GuiGlobalContext, _: f32, _: f32, scale: f32) -> ((), (), Self::Sized) {
        ((), (), HeldItemGuiBlockSized { inner: self, scale })
    }
}

impl<
    'a,
    H: BorrowItemSlot + Debug,
> GuiNode<'a> for HeldItemGuiBlockSized<'a, H> {
    never_blocks_cursor_impl!();

    fn draw(mut self, ctx: GuiSpatialContext<'a>, canvas: &mut Canvas2<'a, '_>) {
        if let Some(pos) = ctx.cursor_pos {
            let layout = ItemSlotLayoutCalcs::new(self.scale, &ItemGridConfig::default());
            let mut canvas = canvas.reborrow()
                .translate(pos)
                .translate(-layout.slot_outer_size / 2.0);
            let mut held_guard = self.inner.held.borrow();
            let held = H::deref(&mut held_guard);
            draw_item_noninteractive(
                ctx,
                &mut canvas,
                self.scale,
                &layout,
                held.as_ref(),
                self.inner.held_state,
                self.inner.items_mesh,
            );
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ItemGridConfig {
    /// Makes slots bigger than their default logical size of 32 
    pub slot_scale: f32,
    /// Logical padding around slots.
    pub pad: f32,
}

impl Default for ItemGridConfig {
    fn default() -> Self {
        ItemGridConfig {
            slot_scale: 1.0,
            pad: 2.0,
        }
    }
}

const SLOT_DEFAULT_SLOT_SIZE: f32 = 32.0;
const SLOT_DEFAULT_TEXT_SIZE: f32 = 16.0;

#[derive(Debug)]
pub struct ItemGrid<'a, I1, I2, C> {
    pub slots: I1,
    pub slots_state: I2,
    pub click_logic: C,
    pub grid_size: Extent2<u32>,
    pub config: ItemGridConfig,
    pub items_mesh: &'a PerItem<ItemMesh>,
}

#[derive(Debug)]
#[doc(hidden)]
pub struct ItemGridSized<'a, I1, I2, C> {
    inner: ItemGrid<'a, I1, I2, C>,
    scale: f32,
}

impl<
    'a,
    I1: IntoIterator + Debug,
    I2: IntoIterator + Debug,
    C: ItemSlotClickLogic + Debug,
> GuiBlock<'a, DimChildSets, DimChildSets> for ItemGrid<'a, I1, I2, C>
where
    <I1 as IntoIterator>::Item: BorrowItemSlot,
    <I2 as IntoIterator>::Item: ItemSlotGuiStateGeneral<'a, <I1 as IntoIterator>::Item>,
{
    type Sized = ItemGridSized<'a, I1, I2, C>;

    fn size(
        self,
        _: &GuiGlobalContext<'a>,
        (): (),
        (): (),
        scale: f32,
    ) -> (f32, f32, Self::Sized) {
        let size = self.grid_size.map(|n| n as f32)
            * (SLOT_DEFAULT_SLOT_SIZE * self.config.slot_scale + self.config.pad * 2.0)
            * scale;
        (size.w, size.h, ItemGridSized {
            inner: self,
            scale,
        })
    }
}



impl<
    'a,
    I1: IntoIterator + Debug,
    I2: IntoIterator + Debug,
    C: ItemSlotClickLogic + Debug,
> GuiNode<'a> for ItemGridSized<'a, I1, I2, C>
where
    <I1 as IntoIterator>::Item: BorrowItemSlot,
    <I2 as IntoIterator>::Item: ItemSlotGuiStateGeneral<'a, <I1 as IntoIterator>::Item>,
{
    fn blocks_cursor(&self, ctx: GuiSpatialContext) -> bool {
        let &ItemGridSized { ref inner, scale } = self;
        let size = ItemGridLayoutCalcs::new(ctx, scale, inner.grid_size, &inner.config).size;
        ctx.cursor_in_area(0.0, size)
    }

    fn draw(self, ctx: GuiSpatialContext<'a>, canvas: &mut Canvas2<'a, '_>) {
        let ItemGridSized { inner, scale } = self;
        
        // layout calcs
        let layout = ItemGridLayoutCalcs::new(ctx, scale, inner.grid_size, &inner.config);

        let mut draw_cursor_over_state = None;

        // render slots
        let mut slots = inner.slots.into_iter();
        let mut slots_state = inner.slots_state.into_iter();

        for y in 0..inner.grid_size.h {
            for x in 0..inner.grid_size.w {
                let xy = Vec2 { x, y };

                let borrow_slot = slots.next()
                    .expect("ItemGrid slots produced None when expected Some");
                let slot_state = slots_state.next()
                    .expect("ItemGrid slots_state produced None when expected Some");

                //let mut slot_guard = borrow_slot.borrow();
                //let slot = <<I1 as IntoIterator>::Item as BorrowItemSlot<'_>>::deref(&mut slot_guard);

                let mut canvas = canvas.reborrow()
                    .translate(xy.map(|n| n as f32) * layout.inner.slot_outer_size);

                // debug background
                if false {
                    canvas.reborrow()
                        .translate(layout.inner.pad_size)
                        .color([1.0, 0.0, 0.0, 0.5])
                        .draw_solid(layout.inner.slot_inner_size);
                }

                let curr_draw_cursor_over_state = slot_state.draw(
                    ctx,
                    &mut canvas,
                    scale,
                    &layout,
                    borrow_slot,
                    inner.items_mesh,
                );

                if layout.cursor_over == Some(xy) {
                    draw_cursor_over_state = Some(curr_draw_cursor_over_state);
                }
            }
        }

        // specifics for moused over slot
        if let Some(xy) = layout.cursor_over {
            <<I2 as IntoIterator>::Item as ItemSlotGuiStateGeneral<_>>::draw_cursor_over(
                draw_cursor_over_state.unwrap(),
                ctx,
                canvas,
                scale,
                xy,
                &layout,
            );
        }
    }

    fn on_cursor_click(self, ctx: GuiSpatialContext, hits: bool, button: MouseButton) {
        let ItemGridSized { inner, scale } = self;
        
        // layout calculation
        let cursor_over = ItemGridLayoutCalcs::new(ctx, scale, inner.grid_size, &inner.config).cursor_over;

        // calculate which slot clicked, or return
        if !hits { return }
        let xy = match cursor_over {
            Some(xy) => xy,
            None => return,
        };

        // convert to index and get actual slot
        let i = xy.y as usize * inner.grid_size.w as usize + xy.x as usize;
        let mut borrow_slot = inner.slots.into_iter().nth(i)
            .expect("ItemGrid slots produced None when expected Some");

        let mut slot_guard = borrow_slot.borrow();
        let slot = <<I1 as IntoIterator>::Item as BorrowItemSlot>::deref(&mut slot_guard);
        
        inner.click_logic.on_click(i, slot, button, ctx.game());
    }
}
