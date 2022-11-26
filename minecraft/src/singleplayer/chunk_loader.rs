
use super::tile_meshing::mesh_tile;
use crate::{
    game_data::GameData,
    chunk_mesh::ChunkMesh,
};
use graphics::{
    Renderer,
    AsyncGpuVecContext,
};
use chunk_data::{
    CHUNK_EXTENT,
    ChunkBlocks,
    LoadedChunks,
    PerChunk,
    TileKey,
    ltc_to_lti,
};
use mesh_data::MeshData;
use std::{
    thread,
    any::Any,
    panic::{
        AssertUnwindSafe,
        catch_unwind,
        resume_unwind,
    },
    sync::Arc,
};
use crossbeam_channel::{
    Sender,
    Receiver,
    TryRecvError,
};
use std_semaphore::Semaphore;
//use rand_chacha::ChaCha20Rng;
use bracket_noise::prelude::FastNoise;
use vek::*;
//use rand::prelude::*;


/// Thread pool for doing the work of preparing chunks asynchronously.
#[derive(Debug)]
pub struct ChunkLoader {
    send_task: Sender<Task>,
    recv_task_result: Receiver<TaskResult>,
}

/// Chunk that is ready to be loaded into the world.
#[derive(Debug)]
pub struct ReadyChunk {
    pub cc: Vec3<i64>,
    pub chunk_tile_blocks: ChunkBlocks,
    pub chunk_tile_meshes: ChunkMesh,
}


#[derive(Debug, Clone)]
enum Task {
    GetChunkReady {
        cc: Vec3<i64>,
    },
}

enum TaskResult {
    ChunkReady(ReadyChunk),
    Panicked(Box<dyn Any + Send + 'static>),
}


impl ChunkLoader {
    pub fn new(game: &Arc<GameData>, renderer: &Renderer) -> Self {
        let (send_task, recv_task) = crossbeam_channel::unbounded();
        let (
            send_task_result,
            recv_task_result,
        ) = crossbeam_channel::unbounded();

        let num_threads = num_cpus::get();
        let gpu_upload_concurrency = 10000;

        let gpu_upload_limiter =
            Arc::new(Semaphore::new(gpu_upload_concurrency));

        for _ in 0..num_threads {
            let recv_task = Receiver::clone(&recv_task);
            let send_task_result = Sender::clone(&send_task_result);
            let game = Arc::clone(&game);
            let gpu_vec_context = renderer.create_async_gpu_vec_context();
            let gpu_upload_limiter = Arc::clone(&gpu_upload_limiter);

            thread::spawn(move || worker_thread_body(
                recv_task,
                send_task_result,
                game,
                gpu_vec_context,
                gpu_upload_limiter,
            ));
        }

        ChunkLoader {
            send_task,
            recv_task_result,
        }
    }

    /// Asynchronously ask the threadpool to get this chunk ready.
    pub fn request(&self, cc: Vec3<i64>) {
        let task = Task::GetChunkReady { cc };
        let send_result = self.send_task.send(task);
        if send_result.is_err() {
            error!("chunk loader task sender disconnected");
        }
    }

    /// Poll for a chunk which are ready to be loaded into the world without
    /// blocking.
    pub fn poll_ready(&self) -> Option<ReadyChunk> {
        match self.recv_task_result.try_recv() {
            Ok(TaskResult::ChunkReady(loaded)) => Some(loaded),
            Ok(TaskResult::Panicked(panic)) => resume_unwind(panic),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                panic!(
                    "chunk loaded task result receiver empty and disconnected"
                );
            }
        }
    }
}

fn worker_thread_body(
    recv_task: Receiver<Task>,
    send_task_result: Sender<TaskResult>,
    game: Arc<GameData>,
    gpu_vec_context: AsyncGpuVecContext,
    gpu_upload_limiter: Arc<Semaphore>,
) {
    let loop_result =
        catch_unwind(AssertUnwindSafe(|| {
            while let Ok(task) = recv_task.recv() {
                let task_result = do_task(
                    task,
                    &game,
                    &gpu_vec_context,
                    &gpu_upload_limiter,
                );
                let send_result = send_task_result.send(task_result);
                if send_result.is_err() {
                    trace!(
                        "chunk loader task result sender disconnected, \
                        terminating worker"
                    );
                    break;
                }
            }
            trace!(
                "chunk loader task receiver disconnected, terminating \
                worker"
            );
        }));
    if let Err(panic) = loop_result {
        error!("chunk loader worker panicked, sending panic to main loop");
        let send_result = send_task_result.send(TaskResult::Panicked(panic));
        if let Err(failed_to_send) = send_result {
            let panic = match failed_to_send.into_inner() {
                TaskResult::Panicked(panic) => panic,
                _ => unreachable!(),
            };
            error!(
                "chunk loader task result sender disconnected when trying to \
                send chunk loader worker panic, resuming chunk loader worker \
                panic here"
            );
            resume_unwind(panic);
        }
    }
}

fn do_task(
    task: Task,
    game: &GameData,
    gpu_vec_context: &AsyncGpuVecContext,
    gpu_upload_limiter: &Semaphore,
) -> TaskResult {
    match task {
        Task::GetChunkReady {
            cc
        } => TaskResult::ChunkReady(get_chunk_ready(
            cc,
            game,
            gpu_vec_context,
            gpu_upload_limiter,
        )),
    }
}

fn get_chunk_ready(
    cc: Vec3<i64>,
    game: &GameData,
    gpu_vec_context: &AsyncGpuVecContext,
    gpu_upload_limiter: &Semaphore,
) -> ReadyChunk {
    let mut seed = [0; 32];
    {
        let mut target = &mut seed[..];
        for n in [cc.x, cc.y, cc.z] {
            for b in n.to_le_bytes() {
                target[0] = b;
                target = &mut target[1..];
            }
        }
    }

    let mut chunk_tile_blocks = ChunkBlocks::new(&game.blocks);

    generate_chunk_blocks(cc, &mut chunk_tile_blocks, game);

    let mut chunk_tile_meshes = ChunkMesh::new();
    
    let mut chunks = LoadedChunks::new();
    let mut tile_blocks = PerChunk::new();
    let ci = chunks.add(cc);
    tile_blocks.add(cc, ci, chunk_tile_blocks);
    let getter = chunks.getter();

    let mut mesh_buf = MeshData::new();
    for z in 1..CHUNK_EXTENT.z - 1 {
        for y in 1..CHUNK_EXTENT.y - 1 {
            for x in 1..CHUNK_EXTENT.x - 1 {
                let ltc = Vec3 { x, y, z };
                let lti = ltc_to_lti(ltc);
                let tile = TileKey { cc, ci, lti };

                mesh_buf.clear();

                mesh_tile(
                    &mut mesh_buf,
                    tile,
                    &getter,
                    &tile_blocks,
                    game,
                );

                for vertex in &mut mesh_buf.vertices {
                    vertex.pos += ltc.map(|n| n as f32);
                }

                chunk_tile_meshes.set_tile_submesh(lti, &mesh_buf);
            }
        }
    }

    let chunk_tile_blocks = tile_blocks.remove(cc, ci);
    
    {
        let guard = gpu_upload_limiter.access();
        chunk_tile_meshes.patch(gpu_vec_context);
        drop(guard);
    }

    ReadyChunk {
        cc,
        chunk_tile_blocks,
        chunk_tile_meshes,
    }
}

fn generate_chunk_blocks(
    cc: Vec3<i64>,
    chunk_tile_blocks: &mut ChunkBlocks,
    game: &GameData,
) {
    /*
    let mut rng = ChaCha20Rng::from_seed(seed);
    if cc.y <= 0 {
        for lti in 0..=MAX_LTI {
            let bid =
                [
                    AIR,
                    AIR,
                    AIR,
                    AIR,
                    AIR,
                    AIR,
                    game.bid_stone,
                    game.bid_dirt,
                    game.bid_brick,
                ]
                .choose(&mut rng)
                .copied()
                .unwrap();    
            chunk_tile_blocks.set(lti, bid, ());
        }
    }
    */

    let mut noise = FastNoise::new();
    noise.set_frequency(1.0 / 75.0);

    for x in 0..CHUNK_EXTENT.x {
        for z in 0..CHUNK_EXTENT.z {
            let height =
                noise.get_noise(
                    (x + cc.x * CHUNK_EXTENT.x) as f32,
                    (z + cc.z * CHUNK_EXTENT.z) as f32
                )
                / 2.0
                * 20.0
                + 40.0
                - (cc.y * CHUNK_EXTENT.y) as f32;
            let height = height.floor() as i64;

            for y in 0..i64::min(height, CHUNK_EXTENT.y) {
                let ltc = Vec3 { x, y, z };
                let lti = ltc_to_lti(ltc);

                let depth = height - y;
                debug_assert!(depth >= 1);

                if depth == 1 {
                    chunk_tile_blocks.set(lti, game.bid_grass, ());
                } else {
                    chunk_tile_blocks.set(lti, game.bid_dirt, ());
                }
            }
        }
    }
}