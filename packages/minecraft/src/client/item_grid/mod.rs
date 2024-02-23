//! Client system for handling interactive item grids.

mod layout_logic_default;
mod render_logic_default;
mod click_logic_default;

pub use self::{
    layout_logic_default::ItemGridDefaultLayout,
    render_logic_default::ItemGridDefaultRenderLogic,
    click_logic_default::ItemGridDefaultClickLogic,
};

use crate::{
    gui::prelude::*,
    game_data::*,
};
use graphics::prelude::*;
use std::{
    sync::Arc,
    fmt::Debug,
};
use vek::*;


/// Gui block for a grid of items.
pub fn item_grid_gui_block<'a, I, L, R, C>(
    item_slots: &'a [I],
    layout: L,
    render_logic: R,
    click_logic: C,
) -> impl GuiBlock<'a, DimChildSets, DimChildSets>
where
    I: Debug,
    L: ItemGridLayoutLogic,
    R: ItemGridRenderLogic<I>,
    C: ItemGridClickLogic<I>,
{
    ItemGridGuiBlock { item_slots, layout, render_logic, click_logic }
}

/// Logic/state for converting between item slot index and geometric space.
pub trait ItemGridLayoutLogic: Debug {
    /// Compute the gui block size of the grid as a whole.
    fn grid_size(&self, num_slots: usize, scale: f32) -> Extent2<f32>;

    /// Compute which item slot, if any, the cursor at the given position is moused over.
    fn cursor_over(&self, pos: Vec2<f32>, num_slots: usize, scale: f32) -> Option<usize>;

    /// Compute the relative position and size of where to actually draw the given item slot.
    fn slot_pos_size(
        &self,
        item_slot_idx: usize,
        num_slots: usize,
        scale: f32,
    ) -> (Vec2<f32>, f32);
}

/// Logic/state for rendering each item slot.
pub trait ItemGridRenderLogic<I>: Debug {

}

/// Logic/state for handling an item slot being clicked.
pub trait ItemGridClickLogic<I>: Debug {
    /// Handle an item slot being clicked.
    fn handle_click(
        self,
        item_slot_idx: usize,
        item_slot: &I,
        button: MouseButton,
        game: &Arc<GameData>,
    );
}


// ==== gui block implementation ====

#[derive(Debug)]
struct ItemGridGuiBlock<'a, I, L, R, C> {
    item_slots: &'a [I],
    layout: L,
    render_logic: R,
    click_logic: C,
}

impl<
    'a,
    I: Debug,
    L: ItemGridLayoutLogic,
    R: ItemGridRenderLogic<I>,
    C: ItemGridClickLogic<I>,
> GuiBlock<'a, DimChildSets, DimChildSets> for ItemGridGuiBlock<'a, I, L, R, C> {
    type Sized = SizedItemGridGuiBlock<'a, I, L, R, C>;

    fn size(
        self,
        ctx: &GuiGlobalContext<'a>,
        w_in: (),
        h_in: (),
        scale: f32,
    ) -> (f32, f32, Self::Sized) {
        let size = self.layout.grid_size(self.item_slots.len(), scale);
        (size.w, size.h, SizedItemGridGuiBlock { inner: self, scale })
    }
}

#[derive(Debug)]
struct SizedItemGridGuiBlock<'a, I, L, R, C> {
    inner: ItemGridGuiBlock<'a, I, L, R, C>,
    scale: f32,
}

impl<
    'a,
    I: Debug,
    L: ItemGridLayoutLogic,
    R: ItemGridRenderLogic<I>,
    C: ItemGridClickLogic<I>,
> GuiNode<'a> for SizedItemGridGuiBlock<'a, I, L, R, C> {
    fn blocks_cursor(&self, ctx: GuiSpatialContext<'a>) -> bool {
        ctx.cursor_in_area(0.0, self.inner.layout.grid_size(self.inner.item_slots.len(), self.scale))
    }

    fn on_cursor_click(self, ctx: GuiSpatialContext, hits: bool, button: MouseButton) {
        if !hits {
            return;
        }
        if let Some(i) = ctx.cursor_pos
            .and_then(|pos| self.inner.layout.cursor_over(
                pos,
                self.inner.item_slots.len(),
                self.scale,
            ))
        {
            self.inner.click_logic.handle_click(i, &self.inner.item_slots[i], button, ctx.game());
        }
    }

    fn draw(self, ctx: GuiSpatialContext<'a>, canvas: &mut Canvas2<'a, '_>) {
        let cursor_over = ctx.cursor_pos
            .and_then(|pos| self.inner.layout.cursor_over(
                pos,
                self.inner.item_slots.len(),
                self.scale,
            ));
        for (i, item_slot) in self.inner.item_slots.iter().enumerate() {
            if cursor_over == Some(i) {
                const SELECTED_ALPHA: f32 = (0xc5 as f32 - 0x8b as f32) / (0xff as f32 - 0x8b as f32);
                let (
                    pos,
                    size,
                ) = self.inner.layout.slot_pos_size(i, self.inner.item_slots.len(), self.scale);
                canvas.reborrow()
                    .translate(pos)
                    .color([1.0, 1.0, 1.0, SELECTED_ALPHA])
                    .draw_solid(size);
            }
        }
    }
}
