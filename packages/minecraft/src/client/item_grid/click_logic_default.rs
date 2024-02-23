
use super::*;

/// Reasonable-defaults `ItemGridClickLogic` implementation.
#[derive(Debug)]
pub struct ItemGridDefaultClickLogic {

}

impl<I: std::fmt::Debug> ItemGridClickLogic<I> for ItemGridDefaultClickLogic {
    fn handle_click(
        self,
        item_slot_idx: usize,
        item_slot: &I,
        button: MouseButton,
        game: &Arc<GameData>,
    ) {
        debug!(?item_slot_idx, ?item_slot, ?button, "ItemGridDefaultClickLogic.handle_click");
    }
}
