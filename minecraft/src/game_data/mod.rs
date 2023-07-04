
pub mod per_block;
pub mod per_item;
pub mod blocks;


use self::{
    per_block::PerBlock,
    per_item::PerItem,
};
use crate::{
    item::{
        ItemId,
        ItemRegistry,
    },
    asset::consts::*,
};
use chunk_data::{
    AIR,
    BlockRegistry,
    BlockId,
    PerFace,
    Face,
};
use std::sync::Arc;


#[derive(Debug)]
pub struct GameData {
    pub blocks: Arc<BlockRegistry>,

    pub blocks_mesh_logic: PerBlock<BlockMeshLogic>,
    pub blocks_hitscan_logic: PerBlock<BlockHitscanLogic>,
    pub blocks_physics_logic: PerBlock<BlockPhysicsLogic>,
    pub blocks_can_place_over: PerBlock<bool>,
    pub blocks_break_logic: PerBlock<BlockBreakLogic>,

    pub bid_stone: BlockId<()>,
    pub bid_dirt: BlockId<()>,
    pub bid_grass: BlockId<()>,
    pub bid_planks: BlockId<()>,
    pub bid_brick: BlockId<()>,
    pub bid_glass: BlockId<()>,
    pub bid_log: BlockId<()>,
    pub bid_door: BlockId<blocks::door::DoorMeta>,

    pub items_mesh_index: PerItem<usize>,
    pub items_use_behavior: PerItem<Option<ItemUseBehavior>>,

    pub iid_stone: ItemId<()>,
    pub iid_dirt: ItemId<()>,
    pub iid_grass: ItemId<()>,
    pub iid_planks: ItemId<()>,
    pub iid_brick: ItemId<()>,
    pub iid_glass: ItemId<()>,
    //pub iid_log: ItemId<()>,
    //pub iid_door: ItemId<blocks::door::DoorMeta>,
    pub iid_stick: ItemId<()>,
}

#[derive(Debug)]
pub enum BlockMeshLogic {
    NoMesh,
    FullCube(BlockMeshLogicFullCube),
    Grass,
    Door,
}

#[derive(Debug, Copy, Clone)]
pub struct BlockMeshLogicFullCube {
    pub tex_indices: PerFace<usize>,
    pub transparent: bool,
}

impl BlockMeshLogic {
    pub fn basic_cube(tex_index: usize) -> Self {
        BlockMeshLogic::FullCube(BlockMeshLogicFullCube {
            tex_indices: PerFace::repeat(tex_index),
            transparent: false,
        })
    }

    pub fn basic_cube_faces(tex_indices: PerFace<usize>) -> Self {
        BlockMeshLogic::FullCube(BlockMeshLogicFullCube {
            tex_indices,
            transparent: false,
        })
    }

    pub fn basic_cube_transparent(tex_index: usize) -> Self {
        BlockMeshLogic::FullCube(BlockMeshLogicFullCube {
            tex_indices: PerFace::repeat(tex_index),
            transparent: true,
        })
    }

    pub fn obscures(&self, _face: Face) -> bool {
        match self {
            &BlockMeshLogic::NoMesh => false,
            &BlockMeshLogic::FullCube(mesh_logic) => !mesh_logic.transparent,
            &BlockMeshLogic::Grass => true,
            &BlockMeshLogic::Door => false,
        }
    }
}

#[derive(Debug)]
pub enum BlockHitscanLogic {
    Vacuous,
    BasicCube,
    Door,
}

#[derive(Debug)]
pub enum BlockBreakLogic {
    Null,
    Door,
}

#[derive(Debug)]
pub enum BlockPhysicsLogic {
    NoClip,
    BasicCube,
    Door,
}
/*
fn block_item_mesh(tex_index: usize, renderer: &Renderer) -> Mesh {
    let mut mesh_buf = MeshData::new();
    let shade = 0.5;
    mesh_buf
        .add_quad(&Quad {
            pos_start: [0.0, 0.0, 0.0].into(),
            pos_ext_1: [0.0, 1.0, 0.0].into(),
            pos_ext_2: [1.0, 0.0, 0.0].into(),
            tex_start: 0.0.into(),
            tex_extent: 1.0.into(),
            vert_colors: [[shade, shade, shade, 1.0].into(); 4],
            tex_index,
        });
    mesh_buf
        .add_quad(&Quad {
            pos_start: [1.0, 0.0, 0.0].into(),
            pos_ext_1: [0.0, 1.0, 0.0].into(),
            pos_ext_2: [0.0, 0.0, 1.0].into(),
            tex_start: 0.0.into(),
            tex_extent: 1.0.into(),
            vert_colors: [[shade, shade, shade, 1.0].into(); 4],
            tex_index,
        });
    mesh_buf
        .add_quad(&Quad {
            pos_start: [0.0, 1.0, 0.0].into(),
            pos_ext_1: [0.0, 0.0, 1.0].into(),
            pos_ext_2: [1.0, 0.0, 0.0].into(),
            tex_start: 0.0.into(),
            tex_extent: 1.0.into(),
            vert_colors: [Rgba::white(); 4],
            tex_index,
        });
    mesh_buf.upload(renderer)
}
*/

#[derive(Debug)]
pub enum ItemUseBehavior {
    Place(BlockId<()>),
}

impl GameData {
    pub fn new() -> Self {
        let mut blocks = BlockRegistry::new();

        let mut blocks_mesh_logic = PerBlock::new_no_default();
        let mut blocks_hitscan_logic = PerBlock::new(BlockHitscanLogic::BasicCube);
        let mut blocks_physics_logic = PerBlock::new(BlockPhysicsLogic::BasicCube);
        let mut blocks_can_place_over = PerBlock::new(false);
        let mut blocks_break_logic = PerBlock::new(BlockBreakLogic::Null);

        blocks_mesh_logic.set(AIR, BlockMeshLogic::NoMesh);
        blocks_hitscan_logic.set(AIR, BlockHitscanLogic::Vacuous);
        blocks_physics_logic.set(AIR, BlockPhysicsLogic::NoClip);
        blocks_can_place_over.set(AIR, true);

        let bid_stone = blocks.register();
        blocks_mesh_logic.set(bid_stone, BlockMeshLogic::basic_cube(BTI_STONE));

        let bid_dirt = blocks.register();
        blocks_mesh_logic.set(bid_dirt, BlockMeshLogic::basic_cube(BTI_DIRT));

        let bid_grass = blocks.register();
        blocks_mesh_logic.set(bid_grass, BlockMeshLogic::Grass);

        let bid_planks = blocks.register();
        blocks_mesh_logic.set(bid_planks, BlockMeshLogic::basic_cube(BTI_PLANKS));

        let bid_brick = blocks.register();
        blocks_mesh_logic.set(bid_brick, BlockMeshLogic::basic_cube(BTI_BRICK));

        let bid_glass = blocks.register();
        blocks_mesh_logic.set(bid_glass, BlockMeshLogic::basic_cube_transparent(BTI_GLASS));

        let bid_log = blocks.register();
        blocks_mesh_logic.set(bid_log, blocks::log::log_mesh_logic());

        let bid_door = blocks.register();
        blocks_mesh_logic.set(bid_door, BlockMeshLogic::Door);
        blocks_hitscan_logic.set(bid_door, BlockHitscanLogic::Door);
        blocks_physics_logic.set(bid_door, BlockPhysicsLogic::Door);
        blocks_break_logic.set(bid_door, BlockBreakLogic::Door);

        let mut items = ItemRegistry::new();

        let mut items_mesh_index = PerItem::new_no_default();
        let mut items_use_behavior = PerItem::new(None);

        let iid_stone = items.register();
        items_mesh_index.set(iid_stone, IMI_STONE);
        items_use_behavior.set(iid_stone, Some(ItemUseBehavior::Place(bid_stone)));

        let iid_dirt = items.register();
        items_mesh_index.set(iid_dirt, IMI_DIRT);
        items_use_behavior.set(iid_dirt, Some(ItemUseBehavior::Place(bid_dirt)));

        let iid_grass = items.register();
        items_mesh_index.set(iid_grass, IMI_GRASS);
        items_use_behavior.set(iid_grass, Some(ItemUseBehavior::Place(bid_grass)));

        let iid_planks = items.register();
        items_mesh_index.set(iid_planks, IMI_PLANKS);
        items_use_behavior.set(iid_planks, Some(ItemUseBehavior::Place(bid_planks)));

        let iid_brick = items.register();
        items_mesh_index.set(iid_brick, IMI_BRICK);
        items_use_behavior.set(iid_brick, Some(ItemUseBehavior::Place(bid_brick)));

        let iid_glass = items.register();
        items_mesh_index.set(iid_glass, IMI_GLASS);
        items_use_behavior.set(iid_glass, Some(ItemUseBehavior::Place(bid_glass)));

        let iid_stick = items.register();
        items_mesh_index.set(iid_stick, IMI_STICK);

        GameData {
            blocks: blocks.finalize(),

            blocks_mesh_logic,
            blocks_hitscan_logic,
            blocks_physics_logic,
            blocks_can_place_over,
            blocks_break_logic,

            bid_stone,
            bid_dirt,
            bid_grass,
            bid_planks,
            bid_brick,
            bid_glass,
            bid_log,
            bid_door,

            items_mesh_index,
            items_use_behavior,

            iid_stone,
            iid_dirt,
            iid_grass,
            iid_planks,
            iid_brick,
            iid_glass,
            iid_stick,
        }
    }
}
