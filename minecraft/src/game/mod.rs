
pub mod block_update_queue;


use self::block_update_queue::BlockUpdateQueue;
use crate::{
    gui::{*, blocks::*},
    game_data::GameData,
    chunk_mesh::ChunkMesh,
};
use chunk_data::*;
use rocksdb::DB;
use vek::*;


/// All the state of a world that's replicated between client and server.
#[derive(Debug)]
pub struct Replicated {
    tile_blocks: PerChunk<ChunkBlocks>,
}

#[derive(Debug)]
pub struct Client {
    chunks: LoadedChunks,
    replicated: Replicated,
    block_updates: BlockUpdateQueue,
    tile_meshes: PerChunk<ChunkMesh>,
}

#[derive(Debug)]
pub struct Server {
    chunks: LoadedChunks,
    replicated: Replicated,
    block_updates: BlockUpdateQueue,
}

#[derive(Debug)]
pub enum Edit {
    SetBlock(Vec3<i64>, RawBlockId, ErasedBlockMeta),
}

impl Edit {
    pub fn apply(
        self,
        state: &mut Replicated,
        getter: &Getter,
    ) -> Edit {
        match self {
            Edit::SetBlock(gtc, bid, meta) => {
                let (pre_bid, pre_meta) = getter
                    .gtc_get(gtc).unwrap()
                    .get(&mut state.tile_blocks)
                    .erased_replace(bid, meta);
                Edit::SetBlock(gtc, pre_bid, pre_meta)
            }
        }
    }
}


/*
#[derive(Debug)]
pub struct Game {
    save: DB,

    chunks: LoadedChunks,
    tile_blocks: PerChunk<ChunkBlocks>,
    tile_meshes: PerChunk<ChunkMesh>,

    pos: Vec3<f32>,
    pitch: f32,
    yaw: f32,
}

impl Game {
    pub fn new(game: &GameData) -> Self {
        let save = DB::open_default("notminecraft/save")
            .expect("database open failure");

        Game {
            save,

            tile_blocks,
            tile_meshes,

            pos: Vec3::new(16.0, 64.0, 16.0),
            pitch: 0.0,
            yaw: 0.0,
        }
    }

    fn gui<'a>(
        &'a mut self,
        ctx: &'a GuiWindowContext,
    ) -> impl GuiBlock<'a, DimParentSets, DimParentSets>
    {
        layer(())
    }
}

impl GuiStateFrame for Game {
    impl_visit_nodes!();
}
*/