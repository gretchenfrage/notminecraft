
use crate::game_data::content_module_prelude::*;


#[derive(Debug)]
pub struct ContentModule {
    pub bid_chest: BlockId<ChestBlockMeta>,
}

impl ContentModule {
    pub fn init(builder: &mut GameDataBuilder) -> Self {
        let bid_chest = builder.register_block(
            "chest",
            #[cfg(feature = "client")]
            BlockMeshLogic::basic_cube_faces({
                let mut faces = PerFace::repeat(BTI_CHEST_SIDE);
                faces[Face::PosY] = BTI_CHEST_TOP_BOTTOM;
                faces[Face::NegY] = BTI_CHEST_TOP_BOTTOM;
                faces[Face::NegZ] = BTI_CHEST_FRONT;
                faces
            }),
        );

        ContentModule {
            bid_chest
        }
    }
}

/// Metadata for chest blocks.
#[derive(Debug, Clone, GameBinschema)]
pub struct ChestBlockMeta {
    pub slots: [Option<ItemStack>; 27],
}

impl Default for ChestBlockMeta {
    fn default() -> Self {
        ChestBlockMeta {
            slots: array_default(),
        }
    }
}

/*
/// Game menu for an opened chest.
#[derive(Debug)]
pub struct ChestMenu {
    gtc: Vec3<i64>,
    slots_state: Box<[ItemSlotGuiState; 27]>,
}

impl ChestMenu {
    pub fn new(gtc: Vec3<i64>) -> ChestMenu {
        ChestMenu {
            gtc,
            slots_state: Box::new(array_default()),
        }
    }

    pub fn gui<'a>(
        &'a mut self,
        args: MenuGuiParams<'a, '_>,
    ) -> impl GuiBlock<'a, DimParentSets, DimParentSets> + 'a {
        let (
            inventory_slots_state_bottom,
            inventory_slots_state_top,
        ) = args.inventory_slots_state.split_at_mut(9);
        // TODO: don't panic
        let tile = args.getter.gtc_get(self.gtc).unwrap();
        let meta = tile
            .get(args.tile_blocks)
            .try_meta(args.ctx.game().content.chest.bid_chest).unwrap();
        align(0.5,
            logical_size([352.0, 336.0],
                layer((
                    SingleChestBg,
                    margin(14.0, 0.0, 34.0, 0.0,
                        align(0.0,
                            ItemGrid {
                                slots: meta.slots.iter(),
                                slots_state: self.slots_state.iter_mut(),
                                click_logic: MultiplayerItemSlotClickLogic {
                                    slot_offset: 36,
                                    open_menu_msg_idx: args.open_menu_msg_idx.unwrap(),
                                    connection: args.connection,
                                    predictions_to_make: args.predictions_to_make,
                                    idx_space: MultiplayerItemSlotIdxSpace::Chest {
                                        ci: tile.ci,
                                        lti: tile.lti,
                                        meta,
                                    },
                                },
                                grid_size: [9, 3].into(),
                                config: ItemGridConfig::default(),
                                items_mesh: args.items_mesh,
                            }
                        )
                    ),
                    margin(14.0, 0.0, 170.0, 0.0,
                        align(0.0,
                            ItemGrid {
                                slots: &args.inventory_slots[9..],
                                slots_state: inventory_slots_state_top.iter_mut(),
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
                    margin(14.0, 0.0, 286.0, 0.0,
                        align(0.0,
                            ItemGrid {
                                slots: &args.inventory_slots[..9],
                                slots_state: inventory_slots_state_bottom.iter_mut(),
                                click_logic: MultiplayerItemSlotClickLogic {
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
                    ),
                    HeldItemGuiBlock {
                        held: args.held_item,
                        held_state: args.held_item_state,
                        items_mesh: &args.items_mesh,
                    }
                ))
            )
        )
    }
}


/// Gui block for a single chest background texture.
#[derive(Debug, Clone)]
pub struct SingleChestBg;

impl<'a> GuiNode<'a> for SimpleGuiBlock<SingleChestBg> {
    simple_blocks_cursor_impl!();

    fn draw(self, ctx: GuiSpatialContext, canvas: &mut Canvas2) {
        canvas.reborrow()
            .draw_image_uv(
                &ctx.assets().gui_chest,
                0,
                Extent2 {
                    w: self.size.w,
                    h: self.size.h * 71.0 / 168.0,
                },
                0.0,
                Extent2 { 
                    w: 1.0,
                    h: 71.0 / 222.0,
                }
            );
        canvas.reborrow()
            .translate(Vec2 {
                x: 0.0,
                y: self.size.h * 71.0 / 168.0,
            })
            .draw_image_uv(
                &ctx.assets().gui_chest,
                0,
                Extent2 {
                    w: self.size.w,
                    h: self.size.h * 97.0 / 168.0,
                },
                Vec2 {
                    x: 0.0,
                    y: 125.0 / 222.0,
                },
                Extent2 {
                    w: 1.0,
                    h: 97.0 / 222.0,
                },
            );
    }
}
*/