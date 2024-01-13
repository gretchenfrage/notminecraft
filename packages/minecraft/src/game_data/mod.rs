//! Statically defines game logic in abstracted and systematized ways.

pub mod per_block;
pub mod per_item;
pub mod logic;
pub mod content;


pub use self::logic::{
    hitscan_logic,
    physics_logic,
    transclone_logic,
};

#[cfg(feature = "client")]
pub use self::logic::{
    block_mesh_logic,
    item_mesh_logic,
};


use self::{
    per_block::PerBlock,
    per_item::PerItem,
    hitscan_logic::BlockHitscanLogic,
    physics_logic::BlockPhysicsLogic,
    transclone_logic::{
        BlockTranscloner,
        BlockTransclonerFor,
        ItemTranscloner,
        ItemTransclonerFor,
    },
    content::ContentModules,
};
use crate::item::{
    ItemRegistry,
    ItemId,
};
use chunk_data::*;
use std::{
    sync::Arc,
    fmt::Debug,
    num::NonZeroU8,
};

#[cfg(feature = "client")]
use self::{
    block_mesh_logic::BlockMeshLogic,
    item_mesh_logic::ItemMeshLogic,
};
#[cfg(feature = "client")]
use crate::{
    asset::LangKey
};


/// Common re-exports for content modules.
pub mod content_module_prelude {
    pub use super::{
        GameDataBuilder,
        per_block::PerBlock,
        hitscan_logic::BlockHitscanLogic,
        physics_logic::BlockPhysicsLogic,
        transclone_logic::{
            BlockTranscloner,
            BlockTransclonerFor,
            ItemTranscloner,
            ItemTransclonerFor,
        },
        content,
    };
    pub use crate::{
        item::*,
        util_array::array_default,
    };
    #[cfg(feature = "client")]
    pub use super::{
        block_mesh_logic::BlockMeshLogic,
        item_mesh_logic::ItemMeshLogic,
    };
    #[cfg(feature = "client")]
    pub use crate::{
        asset::{
            consts::*,
            LangKey,
        },
        gui::prelude::*,
        client::{
            item_grid::prelude::*,
            menu::MenuGuiParams,
        },
    };
    pub use chunk_data::*;
    pub use mesh_data::*;
    pub use graphics::prelude::*;
    pub use game_binschema_derive::GameBinschema;
    pub use vek::*;
}


/// Builder of `GameData`, passed to content module initialization.
#[derive(Debug)]
pub struct GameDataBuilder {
    // ==== blocks ====
    pub blocks: BlockRegistry,

    // required (doesn't have default):
    pub blocks_machine_name: PerBlock<String>,

    #[cfg(feature = "client")]
    pub blocks_mesh_logic: PerBlock<BlockMeshLogic>,

    // optional (has default):
    pub blocks_meta_transcloner: PerBlock<BlockTranscloner>,
    pub blocks_hitscan_logic: PerBlock<BlockHitscanLogic>,
    pub blocks_physics_logic: PerBlock<BlockPhysicsLogic>,
    pub blocks_can_place_over: PerBlock<bool>,


    // ==== items ====
    pub items: ItemRegistry,

    // required (doesn't have default):
    pub items_machine_name: PerItem<String>,
    #[cfg(feature = "client")]
    pub items_name: PerItem<Option<LangKey>>,
    #[cfg(feature = "client")]
    pub items_mesh_logic: PerItem<ItemMeshLogic>,

    // optional (has default):
    pub items_meta_transcloner: PerItem<ItemTranscloner>,
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
        #[cfg(feature = "client")]
        mesh_logic: BlockMeshLogic,
    ) -> BlockId<M>
    where
        M: Debug + Send + Sync + BlockTransclonerFor + 'static,
    {
        let bid = self.blocks.register();
        self.blocks_machine_name.set(bid, machine_name.to_owned());
        #[cfg(feature = "client")]
        self.blocks_mesh_logic.set(bid, mesh_logic);
        self.blocks_meta_transcloner.set(bid, M::transcloner_for());
        bid
    }

    pub fn register_item<M>(
        &mut self,
        machine_name: &str,
        #[cfg(feature = "client")]
        name: LangKey,
        #[cfg(feature = "client")]
        mesh_logic: ItemMeshLogic,
    ) -> ItemId<M>
    where
        M: ItemTransclonerFor,
    {
        let iid = self.items.register();
        self.items_machine_name.set(iid, machine_name.to_owned());
        self.items_meta_transcloner.set(iid, M::transcloner_for());
        #[cfg(feature = "client")]
        self.items_name.set(iid, Some(name));
        #[cfg(feature = "client")]
        self.items_mesh_logic.set(iid, mesh_logic);
        iid
    }
}

/// Static definition of game logic in abstracted and systematized ways.
#[derive(Debug)]
pub struct GameData {
    /// The palette of blocks, of which tiles hold instances.
    pub blocks: Arc<BlockRegistry>,
    /// Machine name for each block (used, for example, in the serialization schema).
    pub blocks_machine_name: PerBlock<String>,
    /// Logic for generating graphical meshes for instances of each block.
    #[cfg(feature = "client")]
    pub blocks_mesh_logic: PerBlock<BlockMeshLogic>,
    /// Transcloner for this block's meta type. See transcloner docs.
    pub blocks_meta_transcloner: PerBlock<BlockTranscloner>,
    /// Logic for hitscans against instances of each block.
    pub blocks_hitscan_logic: PerBlock<BlockHitscanLogic>,
    /// Logic for physics geometry of instances of each block.
    pub blocks_physics_logic: PerBlock<BlockPhysicsLogic>,
    /// Whether instances of each block can be "placed over".
    ///
    /// This means that some other block can be placed where an instance of this block is without
    /// first removing this block, and this block just gets frictionlessly overwritten with the new
    /// one.
    pub blocks_can_place_over: PerBlock<bool>,
    
    /// The space of items, of which instances can exist.
    pub items: ItemRegistry,
    /// Machine name for each item (used, for example, in the serialization schema).
    pub items_machine_name: PerItem<String>,
    #[cfg(feature = "client")]
    pub items_mesh_logic: PerItem<ItemMeshLogic>,
    #[cfg(feature = "client")]
    pub items_name: PerItem<Option<LangKey>>,
    pub items_meta_transcloner: PerItem<ItemTranscloner>,
    pub items_max_count: PerItem<NonZeroU8>,
    pub items_max_damage: PerItem<u16>,

    /// See content modules docs.
    pub content: ContentModules,
}


impl GameData {
    pub fn new() -> Self {
        let mut builder = GameDataBuilder {
            blocks: BlockRegistry::new(),

            blocks_machine_name: PerBlock::new_no_default(),
            blocks_meta_transcloner: PerBlock::new(BlockTranscloner::Unit),
            blocks_hitscan_logic: PerBlock::new(BlockHitscanLogic::BasicCube),
            blocks_physics_logic: PerBlock::new(BlockPhysicsLogic::BasicCube),
            blocks_can_place_over: PerBlock::new(false),

            #[cfg(feature = "client")]
            blocks_mesh_logic: PerBlock::new_no_default(),

            items: ItemRegistry::new(),

            items_machine_name: PerItem::new_no_default(),
            items_meta_transcloner: PerItem::new(ItemTranscloner::Unit),
            #[cfg(feature = "client")]
            items_name: PerItem::new(None),
            #[cfg(feature = "client")]
            items_mesh_logic: PerItem::new_no_default(),
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
            blocks_meta_transcloner: builder.blocks_meta_transcloner,
            blocks_hitscan_logic: builder.blocks_hitscan_logic,
            blocks_physics_logic: builder.blocks_physics_logic,
            blocks_can_place_over: builder.blocks_can_place_over,

            #[cfg(feature = "client")]
            blocks_mesh_logic: builder.blocks_mesh_logic,

            items: builder.items,

            items_machine_name: builder.items_machine_name,
            #[cfg(feature = "client")]
            items_mesh_logic: builder.items_mesh_logic,
            #[cfg(feature = "client")]
            items_name: builder.items_name,
            items_meta_transcloner: builder.items_meta_transcloner,
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
