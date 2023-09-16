
use crate::{
    game_data::GameData,
    client_server::server::event::EventSender,
    save_file::{
        SaveFile,
        read_key,
    },
    thread_pool::{
        ThreadPool,
        ThreadPoolDomain,
    },
};
use chunk_data::{
    CHUNK_EXTENT,
    ChunkBlocks,
    ltc_to_lti,
};
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
use bracket_noise::prelude::FastNoise;
use anyhow::{Result, Error, anyhow};
use vek::*;


/// You request that a chunk be loaded. It sends a job to the thread pool. The
/// job reads the chunk from the save file or generates it for the first time,
/// then sends a `LoadChunkEvent` to the server thread. Abort handles exist.
/// Dropping this cancels the requests, but asynchronously.
#[derive(Debug)]
pub struct ChunkLoader {
    threads: ThreadPoolDomain<LoadChunkThreadState>,
}

/// Result of trying to load a chunk.
#[derive(Debug)]
pub struct LoadChunkEvent(LoadChunkEventInner);

#[derive(Debug)]
enum LoadChunkEventInner {
    ChunkReady {
        ready_chunk: ReadyChunk,
        aborted: Arc<AtomicBool>,
    },
    SaveError(Error),
    GenerationPanic,
}

impl LoadChunkEvent {
    /// The reason why we have this `get` method which can return `None`,
    /// rather than have the event type itself by `Result<ReadyChunk>`, is to
    /// prevent race conditions with abort handles. If load chunk request was
    /// aborted before this method is called, this method is guaranteed to not
    /// return `Ok(Some(_))`.
    pub fn get(self) -> Result<Option<ReadyChunk>> {
        match self.0 {
            LoadChunkEventInner::ChunkReady { ready_chunk, aborted } => {
                if aborted.load(Ordering::SeqCst) {
                    Ok(None)
                } else {
                    Ok(Some(ready_chunk))
                }
            }
            LoadChunkEventInner::SaveError(e) => Err(e),
            LoadChunkEventInner::GenerationPanic => Err(anyhow!("world generation panicked")),
        }
    }
}

/// Chunk that is ready to be loaded into the world.
#[derive(Debug)]
pub struct ReadyChunk {
    pub cc: Vec3<i64>,
    pub chunk_tile_blocks: ChunkBlocks,
    pub saved: bool,
}

/// Handle for aborting a load chunk request. Dropping this handle will not
/// automatically abort the task.
#[derive(Debug)]
pub struct LoadChunkAbortHandle {
    aborted: Arc<AtomicBool>,
}

impl LoadChunkAbortHandle {
    /// See `LoadChunkEvent.get`.
    pub fn abort(self) {
        self.aborted.store(true, Ordering::SeqCst);
    }
}

#[derive(Debug, Clone)]
struct LoadChunkThreadState {
    send_event: EventSender<LoadChunkEvent>,
    save: SaveFile,
    game: Arc<GameData>,
}

impl ChunkLoader {
    pub fn new(
        thread_pool: &ThreadPool,
        mut create_send_event: impl FnMut() -> EventSender<LoadChunkEvent>,
        save: &SaveFile,
        game: &Arc<GameData>,
    ) -> Self {
        ChunkLoader {
            threads: thread_pool.create_domain(|| LoadChunkThreadState {
                send_event: create_send_event(),
                save: save.clone(),
                game: game.clone(),
            }),
        }
    }

    pub fn request(&self, cc: Vec3<i64>) -> LoadChunkAbortHandle {
        let aborted_1 = Arc::new(AtomicBool::new(false));
        let aborted_2 = Arc::clone(&aborted_1);
        self.threads.submit(move |state| state.service_request(cc, aborted_1));
        LoadChunkAbortHandle {
            aborted: aborted_2,
        }
    }
}

impl LoadChunkThreadState {
    fn service_request(&mut self, cc: Vec3<i64>, aborted: Arc<AtomicBool>) {
        if aborted.load(Ordering::SeqCst) {
            return;
        }
        let inner = match catch_unwind(AssertUnwindSafe(|| self.load_chunk(cc))) {
            Ok(Ok(ready_chunk)) => LoadChunkEventInner::ChunkReady { ready_chunk, aborted },
            Ok(Err(e)) => LoadChunkEventInner::SaveError(e),
            Err(_) => LoadChunkEventInner::GenerationPanic,
        };
        self.send_event.send(LoadChunkEvent(inner));
    }

    fn load_chunk(&mut self, cc: Vec3<i64>) -> Result<ReadyChunk> {
        Ok(if let Some(chunk_tile_blocks) = self.save.read(read_key::Chunk(cc))? {
            ReadyChunk {
                cc,
                chunk_tile_blocks,
                saved: true,
            }
        } else {
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

            let mut chunk_tile_blocks = ChunkBlocks::new(&self.game.blocks);
            self.generate_chunk_blocks(cc, &mut chunk_tile_blocks);

            ReadyChunk {
                cc,
                chunk_tile_blocks,
                saved: false,
            }
        })
    }

    fn generate_chunk_blocks(
        &mut self,
        cc: Vec3<i64>,
        chunk_tile_blocks: &mut ChunkBlocks,
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
                        chunk_tile_blocks.set(lti, self.game.content_stone.bid_stone, ());
                    } else {
                        chunk_tile_blocks.set(lti, self.game.content_stone.bid_stone, ());
                    }
                }
            }
        }
    }
}

/*
use crate::{
    game_data::GameData,
    save_file::{
        SaveFile,
        read_key,
    },
    client_server::server::event::EventSender,
};
use chunk_data::{
    CHUNK_EXTENT,
    ChunkBlocks,
    ltc_to_lti,
};
use std::{
    thread,
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
use crossbeam_channel::{
    Sender,
    Receiver,
};
use bracket_noise::prelude::FastNoise;
use anyhow::{Result, Error, anyhow};
use vek::*;


/// Thread pool for doing the work of preparing chunks asynchronously.
#[derive(Debug)]
pub struct ChunkLoader {
    send_task: Sender<Task>,
}

#[derive(Debug)]
pub struct LoadChunkEvent(LoadChunkEventInner);

#[derive(Debug)]
enum LoadChunkEventInner {
    ChunkReady {
        ready_chunk: ReadyChunk,
        aborted: Arc<AtomicBool>,
    },
    SaveError(Error),
    GenerationPanic,
}

impl LoadChunkEvent {
    pub fn get(self) -> Result<Option<ReadyChunk>> {
        match self.0 {
            LoadChunkEventInner::ChunkReady { ready_chunk, aborted } => {
                if aborted.load(Ordering::SeqCst) {
                    Ok(None)
                } else {
                    Ok(Some(ready_chunk))
                }
            }
            LoadChunkEventInner::SaveError(e) => Err(e),
            LoadChunkEventInner::GenerationPanic => Err(anyhow!("world generation panicked")),
        }
    }
}

/// Chunk that is ready to be loaded into the world.
#[derive(Debug)]
pub struct ReadyChunk {
    pub cc: Vec3<i64>,
    pub chunk_tile_blocks: ChunkBlocks,
    pub saved: bool,
}

/// Handle for cancelling a request to the chunk loader to load a chunk.
/// If this is used to abort the request, subsequent calls to
/// `ChunkLoader.poll_ready` are guaranteed to not load the chunk, and if
/// the work of loading it has not yet been performed then time may be
/// saved by not performing that work. Dropping this handle will not
/// automatically abort the task.
#[derive(Debug)]
pub struct LoadChunkAbortHandle {
    aborted: Arc<AtomicBool>,
}

impl LoadChunkAbortHandle {
    pub fn abort(self) {
        self.aborted.store(true, Ordering::SeqCst);
    }
}

#[derive(Debug, Clone)]
struct Task {
    cc: Vec3<i64>,
    aborted: Arc<AtomicBool>,
}


impl ChunkLoader {
    pub fn new(
        send_event: EventSender<LoadChunkEvent>,
        save: &SaveFile,
        game: &Arc<GameData>,
    ) -> Self {
        let (send_task, recv_task) = crossbeam_channel::unbounded();

        let num_threads = num_cpus::get();

        for _ in 0..num_threads {
            let recv_task = Receiver::clone(&recv_task);
            let send_event = send_event.clone();
            let save = SaveFile::clone(save);
            let game = Arc::clone(&game);

            thread::spawn(move || worker_thread_body(
                recv_task,
                send_event,
                save,
                game,
            ));
        }

        ChunkLoader {
            send_task,
        }
    }

    /// Asynchronously ask the threadpool to get this chunk ready.
    pub fn request(&self, cc: Vec3<i64>) -> LoadChunkAbortHandle {
        let aborted_1 = Arc::new(AtomicBool::new(false));
        let aborted_2 = Arc::clone(&aborted_1);
        let task = Task { cc, aborted: aborted_1 };
        let send_result = self.send_task.send(task);
        if send_result.is_err() {
            error!("chunk loader task sender disconnected");
        }
        LoadChunkAbortHandle {
            aborted: aborted_2,
        }
    }
}

fn worker_thread_body(
    recv_task: Receiver<Task>,
    send_event: EventSender<LoadChunkEvent>,
    mut save: SaveFile,
    game: Arc<GameData>,
) {
    let loop_result =
        catch_unwind(AssertUnwindSafe(|| {
            while let Ok(task) = recv_task.recv() {
                if task.aborted.load(Ordering::SeqCst) {
                    continue;
                }
                let task_result = do_task(
                    task,
                    &mut save,
                    &game,
                );
                send_event.send(task_result);
            }
            trace!(
                "chunk loader task receiver disconnected, terminating \
                worker"
            );
        }));
    if let Err(_) = loop_result {
        error!("chunk loader worker panicked");
        send_event.send(LoadChunkEvent(LoadChunkEventInner::GenerationPanic));
    }
}

fn do_task(
    task: Task,
    save: &mut SaveFile,
    game: &GameData,
) -> LoadChunkEvent {
    LoadChunkEvent(match get_chunk_ready(task.cc, save, game) {
        Ok(ready_chunk) => LoadChunkEventInner::ChunkReady {
            ready_chunk,
            aborted: task.aborted,
        },
        Err(e) => LoadChunkEventInner::SaveError(e),
    })
}
*/