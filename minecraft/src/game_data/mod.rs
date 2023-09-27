
pub mod per_block;
pub mod per_item;
pub mod logic;
pub mod content;


pub use self::logic::{
    mesh_logic,
    hitscan_logic,
    physics_logic,
    transclone_logic,
};


use self::{
    per_block::PerBlock,
    mesh_logic::BlockMeshLogic,
    hitscan_logic::BlockHitscanLogic,
    physics_logic::BlockPhysicsLogic,
    transclone_logic::Transcloner,
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
        transclone_logic::Transcloner,
        content,
    };
    pub use crate::asset::consts::*;
    pub use chunk_data::*;
}


#[derive(Debug)]
pub struct GameDataBuilder {
    pub blocks: BlockRegistry,

    // required (doesn't have default):
    pub blocks_machine_name: PerBlock<String>,
    pub blocks_mesh_logic: PerBlock<BlockMeshLogic>,

    // optional (has default):
    pub blocks_meta_transcloner: PerBlock<Transcloner>,
    pub blocks_hitscan_logic: PerBlock<BlockHitscanLogic>,
    pub blocks_physics_logic: PerBlock<BlockPhysicsLogic>,
    pub blocks_can_place_over: PerBlock<bool>,
}

#[derive(Debug)]
pub struct GameData {
    pub blocks: Arc<BlockRegistry>,
    
    pub blocks_machine_name: PerBlock<String>,
    pub blocks_mesh_logic: PerBlock<BlockMeshLogic>,    

    pub blocks_meta_transcloner: PerBlock<Transcloner>,
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

            blocks_meta_transcloner: PerBlock::new(Transcloner::Unit),
            blocks_hitscan_logic: PerBlock::new(BlockHitscanLogic::BasicCube),
            blocks_physics_logic: PerBlock::new(BlockPhysicsLogic::BasicCube),
            blocks_can_place_over: PerBlock::new(false),
        };

        let content_air = content::air::ContentModule::init(&mut builder);
        let content_stone = content::stone::ContentModule::init(&mut builder);

        for bid in builder.blocks.iter() {
            debug_assert_eq!(
                builder.blocks.meta_type_id(bid),
                builder.blocks_meta_transcloner[bid].instance_type_id(),
                "block {:?} meta transcloner not set up right",
                &builder.blocks_machine_name[bid],
            );
        }

        GameData {
            blocks: builder.blocks.finalize(),

            blocks_machine_name: builder.blocks_machine_name,
            blocks_mesh_logic: builder.blocks_mesh_logic,

            blocks_meta_transcloner: builder.blocks_meta_transcloner,
            blocks_hitscan_logic: builder.blocks_hitscan_logic,
            blocks_physics_logic: builder.blocks_physics_logic,
            blocks_can_place_over: builder.blocks_can_place_over,

            content_air,
            content_stone,
        }
    }

    pub fn clone_chunk_blocks(&self, a: &ChunkBlocks) -> ChunkBlocks {
        let mut b = ChunkBlocks::new(&self.blocks);
        for lti in 0..=MAX_LTI {
            let a = TileBlockRead { chunk: a, lti };
            let b = TileBlockWrite { chunk: &mut b, lti };
            self.blocks_meta_transcloner[a.get()].clone_tile_block_meta(a, b);
        }
        b
    }

    pub fn clone_erased_tile_block(&self, a: &ErasedTileBlock) -> ErasedTileBlock {
        ErasedTileBlock {
            bid: a.bid,
            meta: self.blocks_meta_transcloner[a.bid].clone_erased_block_meta(&a.meta),
        }
    }
}
