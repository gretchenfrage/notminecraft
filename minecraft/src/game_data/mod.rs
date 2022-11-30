
pub mod per_block;


use self::per_block::PerBlock;
use chunk_data::{
    AIR,
    FACES,
    BlockRegistry,
    BlockId,
    PerFace,
    Face,
};
use std::sync::Arc;


#[derive(Debug)]
pub struct GameData {
    pub blocks: Arc<BlockRegistry>,

    pub block_obscures: PerBlock<PerFace<bool>>,
    pub block_mesh_logics: PerBlock<BlockMeshLogic>,
    pub block_can_place_over: PerBlock<bool>,
    pub block_on_break: PerBlock<BlockOnBreak>,

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
    /// No mesh.
    Invisible,
    /// Basic cube with given tex idx.
    Simple(usize),
    /// Basic cube with given per-face tex idxs.
    SimpleFaces(PerFace<usize>),
    /// Grass. Hehe.
    Grass,
    /// Door. Hehe.
    Door,
}

#[derive(Debug)]
pub enum BlockOnBreak {
    Null,
    Door,
}

// block tex indexes:

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

        let mut block_obscures = PerBlock::new(PerFace::repeat(true));
        let mut block_mesh_logics = PerBlock::new_no_default();
        let mut block_can_place_over = PerBlock::new(false);
        let mut block_on_break = PerBlock::new(BlockOnBreak::Null);

        block_obscures.set(AIR, PerFace::repeat(false));
        block_mesh_logics.set(AIR, BlockMeshLogic::Invisible);
        block_can_place_over.set(AIR, true);

        let bid_stone = blocks.register();
        block_mesh_logics.set(bid_stone, BlockMeshLogic::Simple(BTI_STONE));

        let bid_dirt = blocks.register();
        block_mesh_logics.set(bid_dirt, BlockMeshLogic::Simple(BTI_DIRT));

        let bid_grass = blocks.register();
        block_mesh_logics.set(bid_grass, BlockMeshLogic::Grass);

        let bid_planks = blocks.register();
        block_mesh_logics.set(bid_planks, BlockMeshLogic::Simple(BTI_PLANKS));

        let bid_brick = blocks.register();
        block_mesh_logics.set(bid_brick, BlockMeshLogic::Simple(BTI_BRICK));

        let bid_glass = blocks.register();
        block_mesh_logics.set(bid_glass, BlockMeshLogic::Simple(BTI_GLASS));

        let bid_log = blocks.register();
        block_mesh_logics
            .set(
                bid_log,
                BlockMeshLogic::SimpleFaces(FACES.map(|face| match face {
                    Face::PosY | Face::NegY => BTI_LOG_TOP,
                    _ => BTI_LOG_SIDE,
                })),
            );

        let bid_door = blocks.register();
        block_obscures.set(bid_door, PerFace::repeat(false));
        block_mesh_logics.set(bid_door, BlockMeshLogic::Door);
        block_on_break.set(bid_door, BlockOnBreak::Door);

        GameData {
            blocks: blocks.finalize(),

            block_obscures,
            block_mesh_logics,
            block_can_place_over,
            block_on_break,

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
