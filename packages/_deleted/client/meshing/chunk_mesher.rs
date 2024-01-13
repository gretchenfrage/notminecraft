
use crate::{
    game_data::GameData,
    thread_pool::{
        ThreadPool,
        ThreadPoolDomain,
    },
    chunk_mesh::ChunkMesh,
    client::meshing::tile_meshing::mesh_tile,
};
use chunk_data::*;
use mesh_data::MeshData;
use graphics::AsyncGpuVecContext;
use std::{
    panic::{
        AssertUnwindSafe,
        catch_unwind,
    },
    sync::{
        atomic::{
            AtomicBool,
            Ordering,
        },
        Arc,
    },
};
use vek::*;
use crossbeam_channel::{
    Sender,
    Receiver,
    unbounded,
    TryRecvError,
};



// TODO: it should be possible to deduplicate some of this logic with chunk_loader?

/// Like the server's `ChunkLoader`, but used on the client side to
/// asynchronously mesh initially loaded chunks.
#[derive(Debug)]
pub struct ChunkMesher {
    threads: ThreadPoolDomain<ThreadState>,
    recv_response: Receiver<Response>,
}

#[derive(Debug)]
enum Response {
    Meshed {
        meshed_chunk: MeshedChunk,
        aborted: Arc<AtomicBool>,
    },
    Panicked,
}

impl ChunkMesher {
    pub fn new(
        thread_pool: &ThreadPool,
        mut create_gpu_vec_ctx: impl FnMut() -> AsyncGpuVecContext,
        game: &Arc<GameData>,
    ) -> Self {
        let (send_response, recv_response) = unbounded();
        ChunkMesher {
            threads: thread_pool.create_domain(|| ThreadState {
                send_response: send_response.clone(),
                gpu_vec_ctx: create_gpu_vec_ctx(),
                game: game.clone(),

                chunks: LoadedChunks::new(),
                tile_blocks: PerChunk::new(),
                tile_meshes: PerChunk::new(),
                mesh_buf: MeshData::new(),
            }),
            recv_response,
        }
    }

    pub fn request(
        &self,
        cc: Vec3<i64>,
        ci: usize,
        chunk_blocks: ChunkBlocks,
    ) -> MeshChunkAbortHandle {
        let aborted_1 = Arc::new(AtomicBool::new(false));
        let aborted_2 = Arc::clone(&aborted_1);
        self.threads.submit(move |state| state.service_request(cc, ci, chunk_blocks, aborted_1), 1);
        MeshChunkAbortHandle { aborted: aborted_2 }
    }

    pub fn try_recv(&self) -> Option<MeshedChunk> {
        loop {
            match self.recv_response.try_recv() {
                Ok(Response::Meshed {
                    meshed_chunk,
                    aborted,
                }) => if !aborted.load(Ordering::SeqCst) { return Some(meshed_chunk) },
                Ok(Response::Panicked) => panic!("chunk meshing panicked"),
                Err(TryRecvError::Empty) => return None,
                Err(TryRecvError::Disconnected) => panic!("chunk mesher response channel disconnected"),
            };
        }
    }
}

#[derive(Debug)]
pub struct MeshedChunk {
    pub cc: Vec3<i64>,
    pub ci: usize,
    pub mesh: ChunkMesh,
}

#[derive(Debug)]
pub struct MeshChunkAbortHandle {
    aborted: Arc<AtomicBool>,
}

impl MeshChunkAbortHandle {
    pub fn abort(self) {
        self.aborted.store(true, Ordering::SeqCst);
    }
}

#[derive(Debug)]
struct ThreadState {
    // actual state
    send_response: Sender<Response>,
    gpu_vec_ctx: AsyncGpuVecContext,
    game: Arc<GameData>,
    
    // basically just reusable buffers
    chunks: LoadedChunks,
    tile_blocks: PerChunk<ChunkBlocks>,
    tile_meshes: PerChunk<ChunkMesh>,
    mesh_buf: MeshData,
}

impl ThreadState {
    fn service_request(
        &mut self,
        cc: Vec3<i64>,
        ci: usize,
        chunk_blocks: ChunkBlocks,
        aborted: Arc<AtomicBool>,
    ) {
        if aborted.load(Ordering::SeqCst) {
            return;
        }
        let response = match catch_unwind(AssertUnwindSafe(|| self.mesh_chunk(cc, chunk_blocks))) {
            Ok(mesh) => Response::Meshed {
                meshed_chunk: MeshedChunk { cc, ci, mesh },
                aborted,
            },
            Err(_) => Response::Panicked,
        };
        let _ = self.send_response.send(response);
    }

    fn mesh_chunk(&mut self, cc: Vec3<i64>, chunk_blocks: ChunkBlocks) -> ChunkMesh {
        // insert chunk blocks into fake world
        let ci = self.chunks.add(cc);
        debug_assert_eq!(ci, 0);
        self.tile_blocks.add(cc, ci, chunk_blocks);
        self.tile_meshes.add(cc, ci, ChunkMesh::new());
        let getter = self.chunks.getter_pre_cached(cc, ci);

        // mesh each tile in the chunk _except_ ones bordering on other chunks
        for z in 1..CHUNK_EXTENT.z - 1 {
            for y in 1..CHUNK_EXTENT.y - 1 {
                for x in 1..CHUNK_EXTENT.x - 1{
                    let lti = ltc_to_lti(Vec3 { x, y, z });
                    self.mesh_buf.clear();
                    mesh_tile(
                        &mut self.mesh_buf,
                        TileKey { cc, ci, lti },
                        &getter,
                        &self.tile_blocks,
                        &self.game,
                    );
                    self.mesh_buf.translate(lti_to_ltc(lti).map(|n| n as f32));
                    self.tile_meshes.get_mut(cc, ci).set_tile_submesh(lti, &self.mesh_buf);
                }
            }
        }
        
        // reset the fake world and extract back out the chunk mesh
        self.chunks.remove(cc);
        self.tile_blocks.remove(cc, ci);
        let mut mesh = self.tile_meshes.remove(cc, ci);

        // upload the data to the GPU and wait for that to happen
        mesh.patch(&self.gpu_vec_ctx);

        // done
        mesh
    }
}
