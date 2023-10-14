//! Data types for save file content. Separate from the actual mechanisms for
//! running the save file elsewhere in these modules.

use crate::{
    game_binschema::GameBinschema,
    item::ItemSlot,
};
use vek::*;


/// Save file value for a player.
#[derive(Debug, GameBinschema, Clone)]
pub struct PlayerData {
    pub pos: Vec3<f32>,
    pub inventory_slots: [ItemSlot; 36],
}
