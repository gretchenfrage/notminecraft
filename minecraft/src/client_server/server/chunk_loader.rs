
use crate::game_data::GameData;
use chunk_data::{
    CHUNK_EXTENT,
    ChunkBlocks,
    ltc_to_lti,
};
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
use bracket_noise::prelude::FastNoise;
use vek::*;


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
    pub fn new(game: &Arc<GameData>) -> Self {
        let (send_task, recv_task) = crossbeam_channel::unbounded();
        let (
            send_task_result,
            recv_task_result,
        ) = crossbeam_channel::unbounded();

        let num_threads = num_cpus::get();

        for _ in 0..num_threads {
            let recv_task = Receiver::clone(&recv_task);
            let send_task_result = Sender::clone(&send_task_result);
            let game = Arc::clone(&game);

            thread::spawn(move || worker_thread_body(
                recv_task,
                send_task_result,
                game,
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
) {
    let loop_result =
        catch_unwind(AssertUnwindSafe(|| {
            while let Ok(task) = recv_task.recv() {
                let task_result = do_task(
                    task,
                    &game,
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
) -> TaskResult {
    match task {
        Task::GetChunkReady {
            cc
        } => TaskResult::ChunkReady(get_chunk_ready(
            cc,
            game,
        )),
    }
}

fn get_chunk_ready(
    cc: Vec3<i64>,
    game: &GameData,
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

    ReadyChunk {
        cc,
        chunk_tile_blocks,
    }
}

fn generate_chunk_blocks(
    cc: Vec3<i64>,
    chunk_tile_blocks: &mut ChunkBlocks,
    game: &GameData,
) {
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
