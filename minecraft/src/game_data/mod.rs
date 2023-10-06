
pub mod per_block;
pub mod per_item;
pub mod logic;
pub mod content;


pub use self::logic::{
    block_mesh_logic,
    item_mesh_logic,
    hitscan_logic,
    physics_logic,
    transclone_logic,
};


use self::{
    per_block::PerBlock,
    per_item::PerItem,
    block_mesh_logic::BlockMeshLogic,
    item_mesh_logic::ItemMeshLogic,
    hitscan_logic::BlockHitscanLogic,
    physics_logic::BlockPhysicsLogic,
    transclone_logic::{Transcloner, TransclonerFor},
    content::ContentModules,
};
use crate::{
    item::{
        ItemRegistry,
        ItemId,
    },
    asset::LangKey,
};
use chunk_data::*;
use std::{
    sync::Arc,
    fmt::Debug,
    num::NonZeroU8,
};


pub mod content_module_prelude {
    pub use super::{
        GameDataBuilder,
        per_block::PerBlock,
        block_mesh_logic::BlockMeshLogic,
        item_mesh_logic::ItemMeshLogic,
        hitscan_logic::BlockHitscanLogic,
        physics_logic::BlockPhysicsLogic,
        transclone_logic::{Transcloner, TransclonerFor},
        content,
    };
    pub use crate::{
        asset::{
            consts::*,
            LangKey,
        },
        item::*,
    };
    pub use chunk_data::*;
}


#[derive(Debug)]
pub struct GameDataBuilder {
    // ==== blocks ====
    pub blocks: BlockRegistry,

    // required (doesn't have default):
    pub blocks_machine_name: PerBlock<String>,
    pub blocks_mesh_logic: PerBlock<BlockMeshLogic>,

    // optional (has default):
    pub blocks_meta_transcloner: PerBlock<Transcloner>,
    pub blocks_hitscan_logic: PerBlock<BlockHitscanLogic>,
    pub blocks_physics_logic: PerBlock<BlockPhysicsLogic>,
    pub blocks_can_place_over: PerBlock<bool>,


    // ==== items ====
    pub items: ItemRegistry,

    // required (doesn't have default):
    pub items_machine_name: PerItem<String>,

    // optional (has default):
    pub items_meta_transcloner: PerItem<Transcloner>,
    pub items_name: PerItem<Option<LangKey>>,
    pub items_mesh_logic: PerItem<ItemMeshLogic>,
    /// Inclusive upper bound on how many can be stacked.
    pub items_max_count: PerItem<NonZeroU8>,
    /// Inclusive upper bound on its damage level.
    pub items_max_damage: PerItem<u16>,
}

impl GameDataBuilder {
    /// Register a new block with the given meta type, automatically populate
    /// all strictly _required_ per-block entries (and also its meta
    /// transcloner), and return its assigned block ID.
    pub fn register_block<M>(
        &mut self,
        machine_name: &str,
        mesh_logic: BlockMeshLogic,
    ) -> BlockId<M>
    where
        M: Debug + Send + Sync + TransclonerFor + 'static,
    {
        let bid = self.blocks.register();
        self.blocks_machine_name.set(bid, machine_name.to_owned());
        self.blocks_mesh_logic.set(bid, mesh_logic);
        self.blocks_meta_transcloner.set(bid, M::transcloner_for());
        bid
    }

    pub fn register_item<M>(
        &mut self,
        machine_name: &str,
        name: LangKey,
        mesh_logic: ItemMeshLogic,
    ) -> ItemId<M>
    where
        M: TransclonerFor,
    {
        let iid = self.items.register();
        self.items_machine_name.set(iid, machine_name.to_owned());
        self.items_meta_transcloner.set(iid, M::transcloner_for());
        self.items_name.set(iid, Some(name));
        self.items_mesh_logic.set(iid, mesh_logic);
        iid
    }
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

    pub items: ItemRegistry,
    pub items_machine_name: PerItem<String>,
    pub items_mesh_logic: PerItem<ItemMeshLogic>,
    pub items_meta_transcloner: PerItem<Transcloner>,
    pub items_name: PerItem<Option<LangKey>>,
    pub items_max_count: PerItem<NonZeroU8>,
    pub items_max_damage: PerItem<u16>,

    pub content: ContentModules,
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

            items: ItemRegistry::new(),

            items_machine_name: PerItem::new_no_default(),
            items_mesh_logic: PerItem::new_no_default(),

            items_meta_transcloner: PerItem::new(Transcloner::Unit),
            items_name: PerItem::new(None),
            items_max_count: PerItem::new(64.try_into().unwrap()),
            items_max_damage: PerItem::new(0),
        };

        let content = ContentModules::init(&mut builder);

        for bid in builder.blocks.iter() {
            debug_assert_eq!(
                builder.blocks.meta_type_id(bid),
                builder.blocks_meta_transcloner[bid].instance_type_id(),
                "block {:?} meta transcloner not set up right",
                &builder.blocks_machine_name[bid],
            );
        }

        // TODO: validate item metadata type
        // TODO: warn about item types without names

        GameData {
            blocks: builder.blocks.finalize(),

            blocks_machine_name: builder.blocks_machine_name,
            blocks_mesh_logic: builder.blocks_mesh_logic,

            blocks_meta_transcloner: builder.blocks_meta_transcloner,
            blocks_hitscan_logic: builder.blocks_hitscan_logic,
            blocks_physics_logic: builder.blocks_physics_logic,
            blocks_can_place_over: builder.blocks_can_place_over,

            items: builder.items,

            items_machine_name: builder.items_machine_name,

            items_meta_transcloner: builder.items_meta_transcloner,
            items_name: builder.items_name,
            items_mesh_logic: builder.items_mesh_logic,
            items_max_count: builder.items_max_count,
            items_max_damage: builder.items_max_damage,

            content
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

    pub fn clone_erased_tile_block(&self, a: &ErasedBidMeta) -> ErasedBidMeta {
        ErasedBidMeta {
            bid: a.bid,
            meta: self.blocks_meta_transcloner[a.bid].clone_erased_block_meta(&a.meta),
        }
    }
}
