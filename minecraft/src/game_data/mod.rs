
pub mod per_block;


use self::per_block::PerBlock;
use chunk_data::{
    AIR,
    BlockRegistry,
    BlockId,
    PerFace,
};
use std::sync::Arc;


#[derive(Debug)]
pub struct GameData {
    pub blocks: Arc<BlockRegistry>,

    pub block_obscures: PerBlock<PerFace<bool>>,
    pub block_mesh_logic: PerBlock<BlockMeshLogic>,

    pub bid_stone: BlockId<()>,
    pub bid_dirt: BlockId<()>,
}

#[derive(Debug)]
pub enum BlockMeshLogic {
    /// No mesh.
    Invisible,
    /// Basic cube with given tex idx.
    Simple(usize),
    // /// Basic cube with given per-face tex idxs.
    // SimpleFaces(PerFace<usize>),
}


impl GameData {
    pub fn new() -> Self {
        let mut blocks = BlockRegistry::new();

        let mut block_obscures = PerBlock::new();
        let mut block_mesh_logic = PerBlock::new();

        block_obscures.set(AIR, PerFace::repeat(false));
        block_mesh_logic.set(AIR, BlockMeshLogic::Invisible);

        let bid_stone = blocks.register();
        block_obscures.set(bid_stone, PerFace::repeat(true));
        block_mesh_logic.set(bid_dirt, BlockMeshLogic::Simple(0));

        let bid_dirt = blocks.register();
        block_obscures.set(bid_dirt, PerFace::repeat(true));
        block_mesh_logic.set(bid_dirt, BlockMeshLogic::Simple(1));

        GameData {
            blocks: blocks.finalize(),

            block_obscures,
            block_mesh_logic,

            bid_stone,
            bid_dirt,
        }
    }
}
