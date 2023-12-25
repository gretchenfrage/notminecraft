//! Utility for triggering jobs to read chunks from the save file or generate new chunks as
//! appropriate.

use crate::{
    game_data::*,
    server::{
        channel::*,
        save_content::*,
        save_db::SaveDb,
        generate_chunk::generate_chunk,
    },
    util_abort_handle::AbortGuard,
};
use std::sync::Arc;


/// Utility for triggering jobs to read chunks from the save file or generate new chunks as
/// appropriate.
pub struct ChunkLoader {
    thread_pool: ThreadPool,
    job_ctx: Arc<JobCtx>,
}

struct JobCtx {
    game: Arc<GameData>,
    server_send: ServerSender,
    save_db: SaveDb,
}

impl ChunkLoader {
    /// Construct.
    pub fn new(
        game: Arc<GameData>,
        server_send: ServerSender,
        thread_pool: ThreadPool,
        save_db: SaveDb,
    ) -> Self {
        ChunkLoader {
            thread_pool,
            job_ctx: Arc::new(JobCtx {
                game,
                server_send,
                save_db,
            }),
        }
    }

    /// Submit a job to the thread pool to read this chunk from the save file, or generate it new
    /// if it has never been saved, and then send it back to the server loop as a `ChunkReady`
    /// event.
    pub fn trigger_load(&self, chunk_key: ChunkKey) -> AbortGuard {
        let ctx = Arc::clone(&self.job_ctx);
        let aborted = AbortGuard::new();
        // submit task
        self.thread_pool.submit(WorkPriority::Server, aborted.handle(), move |aborted| {
            // attempt read
            let result = ctx.save_db.read(chunk_key);
            match result {
                Ok(Some(chunk_val)) => {
                    // loaded
                    let event = ServerEvent::ChunkReady { chunk_key, chunk_val, saved: true };
                    ctx.server_send.send(event);
                }
                Ok(None) => {
                    // must generate
                    let chunk_val = generate_chunk(&ctx.game);
                    let event = ServerEvent::ChunkReady { chunk_key, chunk_val, saved: false };
                    ctx.server_send.send(event);
                }
                Err(e) => {
                    // we don't really have very good error recovery yet
                    error!(%e, "save file read chunk failed");
                }
            }
        });
        aborted
    }
}
