
pub mod per_block;
pub mod blocks;


use self::per_block::PerBlock;
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
    // TODO blocks_physics_logic

    pub blocks_can_place_over: PerBlock<bool>,
    pub blocks_break_logic: PerBlock<BlockBreakLogic>,

    pub bid_stone: BlockId<()>,
    pub bid_dirt: BlockId<()>,
    pub bid_grass: BlockId<()>,
    pub bid_planks: BlockId<()>,
    pub bid_brick: BlockId<()>,
    pub bid_glass: BlockId<()>,
    pub bid_log: BlockId<()>,
    pub bid_door: BlockId<DoorMeta>,
}

#[derive(Debug)]
pub enum BlockMeshLogic {
    NoMesh,
    BasicCube(usize),
    BasicCubeFaces(PerFace<usize>),
    BasicCubeTransparent(usize),
    Grass,
    Door,
}

impl BlockMeshLogic {
    pub fn obscures(&self, _face: Face) -> bool {
        match self {
            &BlockMeshLogic::NoMesh => false,
            &BlockMeshLogic::BasicCube(_) => true,
            &BlockMeshLogic::BasicCubeFaces(_) => true,
            &BlockMeshLogic::BasicCubeTransparent(_) => false,
            &BlockMeshLogic::Grass => true,
            &BlockMeshLogic::Door => false,
        }
    }
}

#[derive(Debug)]
pub enum BlockHitscanLogic {
    Vacuous,
    BasicCube,
}

#[derive(Debug)]
pub enum BlockBreakLogic {
    Null,
    Door,
}

// block tex indexes (BTIs):

pub const BTI_STONE: usize = 0;
pub const BTI_DIRT: usize = 1;
pub const BTI_GRASS_SIDE: usize = 2;
pub const BTI_GRASS_TOP: usize = 3;
pub const BTI_PLANKS: usize = 4;
pub const BTI_BRICK: usize = 5;
pub const BTI_GLASS: usize = 6;
pub const BTI_LOG_SIDE: usize = 7;
pub const BTI_LOG_TOP: usize = 8;
pub const BTI_DOOR_UPPER: usize = 9;
pub const BTI_DOOR_LOWER: usize = 10;

impl GameData {
    pub fn new() -> Self {
        let mut blocks = BlockRegistry::new();

        let mut blocks_mesh_logic = PerBlock::new_no_default();
        let mut blocks_hitscan_logic = PerBlock::new(BlockHitscanLogic::BasicCube);
        let mut blocks_can_place_over = PerBlock::new(false);
        let mut blocks_break_logic = PerBlock::new(BlockBreakLogic::Null);

        blocks_mesh_logic.set(AIR, BlockMeshLogic::NoMesh);
        blocks_hitscan_logic.set(AIR, BlockHitscanLogic::Vacuous);
        blocks_can_place_over.set(AIR, true);

        let bid_stone = blocks.register();
        blocks_mesh_logic.set(bid_stone, BlockMeshLogic::BasicCube(BTI_STONE));

        let bid_dirt = blocks.register();
        blocks_mesh_logic.set(bid_dirt, BlockMeshLogic::BasicCube(BTI_DIRT));

        let bid_grass = blocks.register();
        blocks_mesh_logic.set(bid_grass, BlockMeshLogic::Grass);

        let bid_planks = blocks.register();
        blocks_mesh_logic.set(bid_planks, BlockMeshLogic::BasicCube(BTI_PLANKS));

        let bid_brick = blocks.register();
        blocks_mesh_logic.set(bid_brick, BlockMeshLogic::BasicCube(BTI_BRICK));

        let bid_glass = blocks.register();
        blocks_mesh_logic.set(bid_glass, BlockMeshLogic::BasicCube(BTI_GLASS));

        let bid_log = blocks.register();
        blocks_mesh_logic.set(bid_log, blocks::log::log_mesh_logic());

        let bid_door = blocks.register();
        blocks_mesh_logic.set(bid_door, BlockMeshLogic::Door);
        blocks_break_logic.set(bid_door, BlockBreakLogic::Door);

        GameData {
            blocks: blocks.finalize(),

            blocks_mesh_logic,
            blocks_hitscan_logic,
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
        }
    }
}


#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct DoorMeta {
    pub part: DoorPart,
    pub dir: DoorDir,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum DoorPart {
    Upper,
    Lower,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum DoorDir {
    PosX,
    NegX,
    PosZ,
    NegZ,
}

impl DoorDir {
    pub fn to_face(self) -> Face {
        match self {
            DoorDir::PosX => Face::PosX,
            DoorDir::NegX => Face::NegX,
            DoorDir::PosZ => Face::PosZ,
            DoorDir::NegZ => Face::NegZ,
        }
    }
}

#[test]
fn door_is_inline() {
    assert!(std::mem::size_of::<DoorMeta>() <= 2);
}
