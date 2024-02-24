
use super::*;


pub const DEFAULT_SLOT_LOGICAL_SIZE: f32 = 32.0;

/// Reasonable-defaults `ItemGridLayoutLogic` implementation.
#[derive(Debug)]
pub struct ItemGridDefaultLayout {
    /// Number of item slots in each row.
    pub slots_per_row: usize,
    /// Makes slots bigger than their default logical size of 32 
    pub slot_scale: f32,
    /// Logical padding around slots.
    pub pad: f32,
}

impl ItemGridDefaultLayout {
    /// Construct, populating most parameters with defaults.
    pub fn new(slots_per_row: usize) -> Self {
        ItemGridDefaultLayout {
            slots_per_row,
            slot_scale: 1.0,
            pad: 2.0,
        }
    }
}

impl ItemGridDefaultLayout {
    fn common_calcs(&self, num_slots: usize, scale: f32,) -> CommonCalcs {
        let slot_inner_size = DEFAULT_SLOT_LOGICAL_SIZE * self.slot_scale * scale;
        let pad_size = self.pad * scale;
        let slot_outer_size = slot_inner_size + pad_size * 2.0;
        let num_rows =
            num_slots / self.slots_per_row
            + if num_slots % self.slots_per_row == 0 { 0 } else { 1 };
        CommonCalcs { slot_inner_size, pad_size, slot_outer_size, num_rows }
    }
}

struct CommonCalcs {
    slot_inner_size: f32,
    pad_size: f32,
    slot_outer_size: f32,
    num_rows: usize,
}

impl ItemGridLayoutLogic for ItemGridDefaultLayout {
    fn grid_size(&self, num_slots: usize, scale: f32) -> Extent2<f32> {
        let common = self.common_calcs(num_slots, scale);
        Extent2 {
            w: self.slots_per_row as f32 * common.slot_outer_size,
            h: common.num_rows as f32 * common.slot_outer_size
        }
    }

    fn cursor_over(&self, pos: Vec2<f32>, num_slots: usize, scale: f32) -> Option<usize> {
        let common = self.common_calcs(num_slots, scale);
        let xy = (pos / common.slot_outer_size).map(|n| n.floor() as i64);
        if xy.zip::<usize>(Vec2::new(self.slots_per_row, common.num_rows))
            .map(|(n, bound)| n >= 0 && n < bound as i64)
            .reduce_and()
        {
            let i = xy.y as usize * self.slots_per_row + xy.x as usize;
            if i < num_slots {
                Some(i)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn slot_pos_size(
        &self,
        item_slot_idx: usize,
        num_slots: usize,
        scale: f32,
    ) -> (Vec2<f32>, f32) {
        let common = self.common_calcs(num_slots, scale);
        let pos = Vec2::new(
            item_slot_idx % self.slots_per_row,
            item_slot_idx / self.slots_per_row,
        ).map(|n| n as f32) * common.slot_outer_size + common.pad_size;
        (pos, common.slot_inner_size)
    }
}
