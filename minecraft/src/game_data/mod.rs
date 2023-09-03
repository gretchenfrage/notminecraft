
pub mod per_block;
pub mod per_item;
pub mod logic;
pub mod content;


pub use self::logic::{
    mesh_logic,
    hitscan_logic,
    physics_logic,
};


use self::{
    per_block::PerBlock,
    mesh_logic::BlockMeshLogic,
    hitscan_logic::BlockHitscanLogic,
    physics_logic::BlockPhysicsLogic,
};
use chunk_data::*;
use std::sync::Arc;


pub mod content_module_prelude {
    pub use super::{
        GameDataBuilder,
        per_block::PerBlock,
        mesh_logic::BlockMeshLogic,
        hitscan_logic::BlockHitscanLogic,
        physics_logic::BlockPhysicsLogic,
        content,
    };
    pub use crate::asset::consts::*;
    pub use chunk_data::*;
}


#[derive(Debug)]
pub struct GameDataBuilder {
    pub blocks: BlockRegistry,

    pub blocks_machine_name: PerBlock<String>,
    pub blocks_mesh_logic: PerBlock<BlockMeshLogic>,
    pub blocks_hitscan_logic: PerBlock<BlockHitscanLogic>,
    pub blocks_physics_logic: PerBlock<BlockPhysicsLogic>,
    pub blocks_can_place_over: PerBlock<bool>,
}

#[derive(Debug)]
pub struct GameData {
    pub blocks: Arc<BlockRegistry>,

    pub blocks_machine_name: PerBlock<String>,
    pub blocks_mesh_logic: PerBlock<BlockMeshLogic>,
    pub blocks_hitscan_logic: PerBlock<BlockHitscanLogic>,
    pub blocks_physics_logic: PerBlock<BlockPhysicsLogic>,
    pub blocks_can_place_over: PerBlock<bool>,

    pub content_air: content::air::ContentModule,
    pub content_stone: content::stone::ContentModule,
}


impl GameData {
    pub fn new() -> Self {
        let mut builder = GameDataBuilder {
            blocks: BlockRegistry::new(),
            blocks_machine_name: PerBlock::new_no_default(),
            blocks_mesh_logic: PerBlock::new_no_default(),
            blocks_hitscan_logic: PerBlock::new(BlockHitscanLogic::BasicCube),
            blocks_physics_logic: PerBlock::new(BlockPhysicsLogic::BasicCube),
            blocks_can_place_over: PerBlock::new(false),
        };

        let content_air = content::air::ContentModule::init(&mut builder);
        let content_stone = content::stone::ContentModule::init(&mut builder);

        GameData {
            blocks: builder.blocks.finalize(),

            blocks_machine_name: builder.blocks_machine_name,
            blocks_mesh_logic: builder.blocks_mesh_logic,
            blocks_hitscan_logic: builder.blocks_hitscan_logic,
            blocks_physics_logic: builder.blocks_physics_logic,
            blocks_can_place_over: builder.blocks_can_place_over,

            content_air,
            content_stone,
        }
    }
}
