
use crate::{
    client::item_grid::{
        SLOT_DEFAULT_SLOT_SIZE,
        ItemGridConfig,
    },
    gui::prelude::*,
};
use vek::*;


#[derive(Debug, Copy, Clone)]
pub struct ItemSlotLayoutCalcs {
    // side length of each slot not including pad 
    pub slot_inner_size: f32,
    // thickness of pad around each slot
    pub pad_size: f32,
    // side length of each slot including pad
    pub slot_outer_size: f32,
}

impl ItemSlotLayoutCalcs {
    pub fn new(scale: f32, config: &ItemGridConfig) -> Self {
        let slot_inner_size = SLOT_DEFAULT_SLOT_SIZE * config.slot_scale * scale;
        let pad_size = config.pad * scale;
        let slot_outer_size = slot_inner_size + pad_size * 2.0;

        ItemSlotLayoutCalcs {
            slot_inner_size,
            pad_size,
            slot_outer_size,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ItemGridLayoutCalcs {
    pub inner: ItemSlotLayoutCalcs,
    // size of entire grid
    pub size: Extent2<f32>,
    // grid coordinates of moused-over slot
    pub cursor_over: Option<Vec2<u32>>,
}

impl ItemGridLayoutCalcs {
    pub fn new(
        ctx: GuiSpatialContext,
        scale: f32,
        grid_size: Extent2<u32>,
        config: &ItemGridConfig,
    ) -> Self {
        let inner = ItemSlotLayoutCalcs::new(scale, config);
        let size = grid_size.map(|n| n as f32) * inner.slot_outer_size;

        let cursor_over = ctx.cursor_pos
            .map(|pos| pos / inner.slot_outer_size)
            .map(|xy| xy.map(|n| n.floor() as i64))
            .filter(|xy| xy
                .zip::<u32>(grid_size.into())
                .map(|(n, bound)| n >= 0 && n < bound as i64)
                .reduce_and())
            .map(|xy| xy.map(|n| n as u32));

        ItemGridLayoutCalcs {
            inner,
            size,
            cursor_over,
        }
    }
}

