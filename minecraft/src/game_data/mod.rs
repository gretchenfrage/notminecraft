
pub mod per_block;
pub mod per_item;
pub mod blocks;


use self::{
    per_block::PerBlock,
    per_item::PerItem,
};
use crate::item::{
    ItemId,
    ItemRegistry,
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

    //pub items_mesh: PerItem<Mesh>,

    pub iid_stone: ItemId<()>,
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
            transparent: false,
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
        blocks_mesh_logic.set(bid_glass, BlockMeshLogic::basic_cube(BTI_GLASS));

        let bid_log = blocks.register();
        blocks_mesh_logic.set(bid_log, blocks::log::log_mesh_logic());

        let bid_door = blocks.register();
        blocks_mesh_logic.set(bid_door, BlockMeshLogic::Door);
        blocks_hitscan_logic.set(bid_door, BlockHitscanLogic::Door);
        blocks_physics_logic.set(bid_door, BlockPhysicsLogic::Door);
        blocks_break_logic.set(bid_door, BlockBreakLogic::Door);

        let mut items = ItemRegistry::new();

        //let mut items_mesh = PerItem::new_no_default();

        let iid_stone = items.register();
        //items_mesh.set(iid_stone, )

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

            //items_mesh,

            iid_stone,
        }
    }
}
