
pub mod message;
pub mod server;


/*
/// Messages to client.
#[derive(Debug)]
pub enum ClientMsg {
    LoadChunk(ClientMsgLoadChunk),
    SetTileBlock(ClientMsgSetTileBlock),
}

#[derive(Debug)]
pub struct ClientMsgLoadChunk {
    pub cc: Vec3<i64>,
    pub ci: usize,
    pub tile_blocks: ChunkBlocks,
}

#[derive(Debug)]
pub struct ClientMsgSetTileBlock {
    pub ci: usize,
    pub lti: u16,
    pub bid: RawBlockId,
    // no metadata for now
}


/// Messages to server.
#[derive(Debug)]
pub enum ServerMsg {
    SetTileBlock(ServerMsgSetTileBlock),
}

#[derive(Debug)]
pub struct ServerMsgSetTileBlock {
    // cannot use ci because is asynchronous with changes to loaded chunks
    pub gtc: Vec3<i64>,
    pub bid: RawBlockId,
    // no metadata for now
}


pub fn run_server() {
    
}
*/



/*
#[derive(Debug)]
pub struct Server {
    chunks: LoadedChunks,
    tile_blocks: PerChunk<ChunkBlocks>,
    block_updates: BlockUpdateQueue,
}

#[derive(Debug)]
pub struct Client {
    chunks: LoadedChunks,
    tile_blocks: PerChunk<ChunkBlocks>,
    block_updates: BlockUpdateQueue,
    tile_meshes: PerChunk<ChunkMesh>,
}

#[derive(Debug)]
pub enum Edit {
    SetBlock {
        ci: usize,
        lti: u16,
        bid: RawBlockId,
        meta: ErasedBlockMeta,
    },
}

impl Edit {
    pub fn apply(
        self,
        state: &mut Client,
    ) -> Edit {
        match self {
            Edit::SetBlock { ci, lti, bid, meta } => {
                let (
                    pre_bid,
                    pre_meta,
                ) = state.tile_blocks[ci].erased_replace(lti, bid, meta);
                Edit::SetBlock {
                    ci,
                    lti,
                    bid: pre_bid,
                    meta: pre_meta,
                }
            }
        }
    }
}
*/

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