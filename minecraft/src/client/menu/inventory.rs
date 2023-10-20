

use crate::{
    gui::prelude::*,
    client::{
        menu::MenuGuiParams,
        gui_blocks::{
            item_grid::{
                item_slot_click_logic::{
                    MultiplayerItemSlotClickLogic,
                    MultiplayerItemSlotIdxSpace,
                },
                HeldItemGuiBlock,
                ItemGridConfig,
                ItemGrid,
            },
        },
        meshing::{
            char_mesh::{
                CharMeshGuiBlock,
            },
        },
    },
};
use vek::*;


pub fn gui<'a>(args: MenuGuiParams<'a, '_>) -> impl GuiBlock<'a, DimParentSets, DimParentSets> + 'a {
    let (
        inventory_slots_state_bottom,
        inventory_slots_state_top,
    ) = args.inventory_slots_state.split_at_mut(9);

    align(0.5,
                logical_size(Vec2::new(176.0, 166.0) * 2.0,
                    layer((
                        &args.ctx.assets().gui_inventory,
                        margin(52.0, 0.0, 160.0, 0.0,
                            align(0.0,
                                logical_size([104.0, 140.0],
                                    CharMeshGuiBlock {
                                        char_mesh: args.char_mesh,
                                        head_pitch: args.head_pitch,
                                        pointing: args.pointing,
                                    }
                                )
                            )
                        ),
                        margin(14.0, 0.0, 166.0, 0.0,
                            align(0.0,
                                ItemGrid {
                                    slots: &args.inventory_slots[9..],
                                    slots_state: inventory_slots_state_top.iter_mut(),
                                    /*click_logic: StorageItemSlotClickLogic {
                                        held: held_item,
                                    },*/
                                    click_logic: MultiplayerItemSlotClickLogic {
                                        slot_offset: 9,
                                        open_menu_msg_idx: args.open_menu_msg_idx.unwrap(),
                                        connection: args.connection,
                                        predictions_to_make: args.predictions_to_make,
                                        idx_space: MultiplayerItemSlotIdxSpace::Inventory,
                                    },
                                    grid_size: [9, 3].into(),
                                    config: ItemGridConfig::default(),
                                    items_mesh: &args.items_mesh,
                                }
                            )
                        ),
                        margin(14.0, 0.0, 282.0, 0.0,
                            align(0.0,
                                ItemGrid {
                                    slots: &args.inventory_slots[..9],
                                    slots_state: inventory_slots_state_bottom.iter_mut(),
                                    /*click_logic: StorageItemSlotClickLogic {
                                        held: held_item,
                                    },*/
                                    click_logic: MultiplayerItemSlotClickLogic {
                                        // TODO: handling of open_menu_msg_idx Some vs None
                                        // and its relation to menu stack seems kinda delicate
                                        slot_offset: 0,
                                        open_menu_msg_idx: args.open_menu_msg_idx.unwrap(),
                                        connection: args.connection,
                                        predictions_to_make: args.predictions_to_make,
                                        idx_space: MultiplayerItemSlotIdxSpace::Inventory,
                                    },
                                    grid_size: [9, 1].into(),
                                    config: ItemGridConfig::default(),
                                    items_mesh: &args.items_mesh,
                                }
                            )
                        ),/*
                        margin(14.0, 0.0, 14.0, 0.0,
                            align(0.0,
                                ItemGrid {
                                    slots: inventory_slots_armor,
                                    slots_state: inventory_slots_armor_state,
                                    click_logic: StorageItemSlotClickLogic {
                                        held: held_item,
                                    },
                                    grid_size: [1, 4].into(),
                                    config: ItemGridConfig::default(),
                                    items_mesh: &items_mesh,
                                }
                            )
                        ),
                        margin(174.0, 0.0, 50.0, 0.0,
                            align(0.0,
                                ItemGrid {
                                    slots: inventory_slots_crafting,
                                    slots_state: inventory_slots_crafting_state,
                                    click_logic: StorageItemSlotClickLogic {
                                        held: held_item,
                                    },
                                    grid_size: [2, 2].into(),
                                    config: ItemGridConfig::default(),
                                    items_mesh: &items_mesh,
                                }
                            )
                        ),
                        margin(286.0, 0.0, 70.0, 0.0,
                            align(0.0,
                                ItemGrid {
                                    slots: once(inventory_slot_crafting_output),
                                    slots_state: once(inventory_slot_crafting_output_state),
                                    click_logic: StorageItemSlotClickLogic {
                                        held: held_item,
                                    },
                                    grid_size: [1, 1].into(),
                                    config: ItemGridConfig::default(),
                                    items_mesh: &items_mesh,
                                }
                            )
                        ),
                        */
                        HeldItemGuiBlock {
                            held: args.held_item,
                            held_state: args.held_item_state,
                            items_mesh: &args.items_mesh,
                        }
                    ))
                )
            )
}
