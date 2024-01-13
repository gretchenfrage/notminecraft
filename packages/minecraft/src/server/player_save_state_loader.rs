//! Utility for triggering jobs to read players' save state from the save file.

use crate::{
    server::{
        ServerEvent,
        per_player::*,
        channel::*,
        save_content::*,
        save_db::SaveDb,
    },
    thread_pool::*,
    util_abort_handle::AbortHandle,
};
use std::sync::Arc;


/// Utility for triggering jobs to read players' save state from the save file.
pub struct PlayerSaveStateLoader {
    thread_pool: ThreadPool,
    job_ctx: Arc<JobCtx>,
}

struct JobCtx {
    server_send: ServerSender,
    save_db: SaveDb,
}

impl PlayerSaveStateLoader {
    /// Construct.
    pub fn new(server_send: ServerSender, thread_pool: ThreadPool, save_db: SaveDb) -> Self {
        PlayerSaveStateLoader {
            thread_pool,
            job_ctx: Arc::new(JobCtx {
                server_send,
                save_db,
            }),
        }
    }

    /// Submit a job to the thread pool to read this player from the save file and send the result
    /// back to the server loop as a `PlayerSaveStateReady` event, with the given abort handle.
    pub fn trigger_load(&self, pk: PlayerKey, save_key: PlayerSaveKey, aborted: AbortHandle) {
        let ctx = Arc::clone(&self.job_ctx);
        self.thread_pool.submit(WorkPriority::Server, aborted, move |aborted| {
            let result = ctx.save_db.clone().read(save_key);
            match result {
                Ok(save_val) => {
                    let event = ServerEvent::PlayerSaveStateReady { pk, save_val };
                    ctx.server_send.send(event, EventPriority::Other, Some(aborted), None);
                }
                Err(e) => {
                    // we don't really have very good error recovery yet
                    error!(%e, "save file read player failed");
                }
            }
        })
    }
}
