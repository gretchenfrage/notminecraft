//! See `SaveMgr`.

use crate::{
    server::{
        per_player::*,
        save_content::*,
        save_db::SaveDb,
        channel::*,
        chunk_mgr,
    },
    thread_pool::*,
    util_abort_handle::AbortGuard,
};
use chunk_data::*;
use std::{
    cell::RefCell,
    sync::Arc,
    mem::take,
};
use slab::Slab;

/// How long to wait between save operations.
const TICKS_BETWEEN_SAVES: u64 = 200;


/// Manages the saving of world data.
pub struct SaveMgr {
    // handle to the thread pool
    thread_pool: ThreadPool,
    // shared state needed to complete a save operation thread pool job
    save_job_ctx: Arc<SaveJobCtx>,
    // state for tracking when the last save operation completed
    last_saved: LastSaved,
    // refcell-guarded state for tracking which things are saved
    tracking: RefCell<TrackingState>,
    // chunks which were loaded, then unloaded, but not yet saved
    unflushed_chunks: HashMap<ChunkKey, ChunkVal>,
    // players which were loaded, then unloaded, but not yet saved
    unflushed_players: HashMap<PlayerKey, PlayerVal>,
}

// shared state needed to complete a save operation threadpool job
struct SaveJobCtx {
    // sender handle to the server event channel
    server_send: ServerSender,
    // handle to the save database
    save_db: SaveDb,
}

// state for tracking when the last save operation completed
enum LastSaved {
    // last save operation completed at this tick
    AtTick(u64),
    // a save operation is currently pending
    InProgress(AbortGuard),
}

// refcell-guarded state for tracking which things are saved
#[derive(Default)]
struct TrackingState {
    // collection of loaded chunks which are unsaved
    unsaved_chunks: Slab<(Vec3<i64>, usize)>,
    // for each chunk, if unsaved, the index of its entry in unsaved_chunks
    chunk_unsaved_idx: PerChunk<Option<usize>>,

    // collection of loaded players which are unsaved
    unsaved_players: Slab<JoinedPlayerKey>,
    // for each player, if unsaved, the index of its entry in unsaved_players
    player_unsaved_idx: PerPlayer<Option<usize>>,
}

/// An in-progress operation of saving the world to the save file.
///
/// Returned from `SaveMgr` to request that the caller use the `SaveOp` to provide the necessary
/// save state then actuate the save operation. The caller should:
///
/// 1. Drain `SaveOp.should_save` until it is empty.
/// 2. For each entry, assemble the appropriate `SaveEntry` from the data currently in the world
///    and push it to `SaveOp.will_save`.
/// 3. Call `SaveOp.submit`.
pub struct SaveOp<'a> {
    save_mgr: &'a mut SaveMgr,
    pub should_save: Vec<ShouldSave>,
    pub will_save: Vec<SaveEntry>,
    submitted: bool,
}

/// A currently loaded part of the world which needs saving. See `SaveOp`.
#[derive(Debug, Clone)]
pub enum ShouldSave {
    /// A chunk. Corresponds to `SaveEntry::Chunk`.
    Chunk {
        cc: Vec3<i64>,
        ci: usize,
    },
    /// A player. Corresponds to `SaveEntry::Player`.
    Player {
        pk: JoinedPlayerKey,
    },
}

impl SaveMgr {
    /// Construct.
    pub fn new(server_send: ServerSender, save_db: SaveDb, thread_pool: ThreadPool) -> Self {
        SaveMgr {
            thread_pool,
            save_job_ctx: Arc::new(SaveJobCtx {
                server_send,
                save_db,
            }),
            last_saved: LastSaved::AtTick(0),
            tracking: Default::default(),
            unflushed_chunks: Default::default(),
            unflushed_players: Default::default(),
        }
    }

    /// Attempt to take a chunk entry from the "unflushed" cache.
    ///
    /// This should be tried before actually querying the save file database.
    pub fn take_unflushed_chunk(&mut self, key: &ChunkKey) -> Option<ChunkVal> {
        self.unflushed_chunks.remove(key)
    }

    /// Attempt to take a player entry from the "unflushed" cache.
    ///
    /// This should be tried before actually querying the save file database.
    pub fn take_unflushed_player(&mut self, key: &PlayerKey) -> Option<PlayerVal> {
        self.unflushed_players.remove(key)
    }

    /// Call upon a chunk being added to the world.
    pub fn add_chunk(&self, cc: Vec3<i64>, ci: usize, saved: bool) {
        let mut tracking = self.tracking.borrow_mut();
        let unsaved_idx =
            if saved {
                Some(tracking.unsaved_chunks.insert((cc, ci)))
            } else {
                None
            };
        tracking.chunk_unsaved_idx.insert(cc, ci, unsaved_idx);
    }

    /// Call upon a player being added to the world.
    pub fn add_player(&self, pk: JoinedPlayerKey, saved: bool) {
        let mut tracking = self.tracking.borrow_mut();
        let unsaved_idx =
            if saved {
                Some(tracking.unsaved_players.insert(pk))
            } else {
                None
            };
        tracking.player_unsaved_idx.insert(pk, unsaved_idx);
    }

    /// Mark the given chunk as unsaved.
    pub fn mark_chunk_unsaved(&self, cc: Vec3<i64>, ci: usize) {
        let mut tracking = self.tracking.borrow_mut();
        if tracking.chunk_unsaved_idx.get(cc, ci).is_none() {
            let unsaved_idx = tracking.unsaved_chunks.insert((cc, ci));
            *tracking.chunk_unsaved_idx.get_mut(cc, ci) = Some(unsaved_idx);
        }
    }

    /// Mark the given player as unsaved.
    pub fn mark_player_unsaved(&self, pk: JoinedPlayerKey) {
        let mut tracking = self.tracking.borrow_mut();
        if tracking.player_unsaved_idx[pk].is_none() {
            let unsaved_idx = tracking.unsaved_players.insert(pk);
            tracking.player_unsaved_idx[pk] = Some(unsaved_idx);
        }
    }

    /// Call upon the given chunk being removed from the world.
    ///
    /// If the chunk is currently unsaved, the provided save file chunk key/val gets put in the
    /// unflushed cache, and will be saved in the next save operation unless removed before that.
    pub fn remove_chunk(
        &mut self,
        cc: Vec3<i64>,
        ci: usize,
        save_key: ChunkKey,
        save_val: ChunkVal,
    ) {
        let unsaved_idx = self.tracking.get_mut().chunk_unsaved_idx.remove(cc, ci);
        if let Some(unsaved_idx) = unsaved_idx {
            self.tracking.get_mut().unsaved_chunks.remove(unsaved_idx);
            self.unflushed_chunks.insert(save_key, save_val);
        }
    }

    /// Call upon the given player being removed from the world.
    ///
    /// If the player is currently unsaved, the provided save file player key/val gets put in the
    /// unflushed cache, and will be saved in the next save operation unless removed before that.
    pub fn remove_player(
        &mut self,
        pk: JoinedPlayerKey,
        save_key: PlayerKey,
        save_val: PlayerVal,
    ) {
        let unsaved_idx = self.tracking.get_mut().player_unsaved_idx.remove(pk);
        if let Some(unsaved_idx) = unsaved_idx {
            self.tracking.get_mut().unsaved_players.remove(unsaved_idx);
            self.unflushed_players.insert(save_key, save_val);
        }
    }

    /// Begin a save operation if appropriate. See `SaveOp`.
    #[must_use]
    pub fn maybe_save(&mut self, tick: u64) -> Option<SaveOp> {
        // if should save
        if match &self.last_saved {
            &LastSaved::AtTick(tick2) => tick >= tick2 + TICKS_BETWEEN_SAVES,
            &LastSaved::InProgress(_) => false,
        } {
            // for everything marked as unsaved, mark as clean and add to should_save
            let mut should_save = Vec::new();
            for (cc, ci) in self.tracking.get_mut().unsaved_chunks.drain() {
                *self.tracking.get_mut().chunk_unsaved_idx.get_mut(cc, ci) = None;
                should_save.push(ShouldSave::Chunk { cc, ci });
            }
            for pk in self.tracking.get_mut().unsaved_players.drain() {
                self.tracking.get_mut().player_unsaved_idx[pk] = None;
                should_save.push(ShouldSave::Player { pk })
            }

            // transfer the unflushed cache into will_save
            let mut will_save = Vec::new();
            for (chunk_key, chunk_val) in self.unflushed_chunks.drain() {
                will_save.push(SaveEntry::Chunk(chunk_key, chunk_val));
            }
            for (player_key, player_val) in self.unflushed_players.drain() {
                will_save.push(SaveEntry::Player(player_key, player_val));
            }

            // return the op
            Some(SaveOp {
                save_mgr: self,
                should_save,
                will_save,
                submitted: false,
            })
        } else {
            None
        }
    }

    /// Call upon receiving a save op done event.
    pub fn on_save_op_done(&mut self, tick: u64) {
        self.last_saved = LastSaved::AtTick(tick);
    }
}

impl<'a> SaveOp<'a> {
    pub fn submit(mut self) {
        // fail fast stuff
        debug_assert!(self.should_save.is_empty(), "save op should save not drained");
        self.submitted = true;
        
        // submit the save operation to the threadpool
        let ctx = Arc::clone(&self.save_mgr.save_job_ctx);
        let entries = take(&mut self.will_save);
        let aborted = AbortGuard::new();
        self.save_mgr.thread_pool.submit(WorkPriority::Server, aborted.handle(), move |_| {
            // do the saving
            let result = ctx.save_db.write(entries);
            if let Err(e) = result {
                // we don't really very good error recovery yet
                error!(%e, "save file write failed");
            } else {
                // send the successful result back to the save mgr
                ctx.server_send.send(ServerEvent::SaveOpDone, EventPriority::Other, None, None);
            }
        });

        // make sure to store the abort guard
        self.save_mgr.last_saved = LastSaved::InProgress(aborted);
    }
}

impl<'a> Drop for SaveOp<'a> {
    fn drop(&mut self) {
        if !self.submitted {
            error!("save op dropped without submitting");
        }
    }
}
