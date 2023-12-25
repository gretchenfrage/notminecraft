//! See `ChunkMgr`.

mod client_add_chunk_manager;

use self::client_add_chunk_manager::ClientAddChunkMgr;
use crate::server::{
    chunk_loader::{
        ChunkLoader,
        ReadyChunk,
        LoadChunkAbortHandle,
    },
    per_connection::*,
};
use chunk_data::*;
use std::{
    num::NonZeroU64,
    collections::{
        VecDeque,
        HashMap,
        hash_map,
    },
};
use vek::*;
use slab::Slab;


/// Manages chunks and their loading and unloading into the server and its clients.
///
/// After calling any methods on `ChunkMgr` that take `&mut self`, events should be taken from
/// `ChunkMgr.effects` and processed until exhausted, unless the specific method specifies that
/// this is not necessary.
///
/// This uses concepts of "load request counts" and "chunk client interest". Every chunk, loaded or
/// not, has a load request count, defaulting to 0, wherein a non-zero load request count
/// represents the desire for whatever reason to have that chunk loaded on the server. A chunk
/// client interest is a relation that can exist between a chunk, loaded or not, and a client,
/// representing the desire for that chunk to be loaded for that client. Chunk client interests are
/// sort of like a subset of load requests, in that the load request count for a chunk is
/// incremented and decremented automatically as chunk client interests for it are created and
/// destroyed.
#[derive(Default)]
pub struct ChunkMgr {
    pub effects: VecDeque<ChunkMgrEffect>,
    // server-side space of chunks
    chunks: LoadedChunks,
    // for each player, that client's client-side space of chunks 
    player_clientside_chunks: PerPlayer<Slab<()>>,
    // for each chunk, for each player, clientside ci, if the client has the chunk loaded.
    // some sort of inverse of clientside_chunks.
    chunk_player_clientside_ci: PerChunk<PerPlayer<Option<usize>>>,
    // for each player, sub-manager for the limited rate at which chunks can be added to it
    player_add_chunk_mgr: PerPlayer<ClientAddChunkMgr>,
    // for each chunk, the load request count for that chunk.
    chunk_load_request_count: PerChunk<NonZeroU64>,
    // chunks which are pending being loaded
    loading_chunks: HashMap<Vec3<i64>, LoadingChunk>,
}

// chunk pending being loaded by the chunk loader
struct LoadingChunk {
    // abort guard for the request to load the chunk and trigger a ChunkReady event
    aborted: AbortGuard,
    // load request count for the chunk (which exists for both loaded and unloaded chunks)
    load_request_count: NonZeroU64,
    // for each player, whether there exists a chunk client interest for that chunk and client
    player_interest: PerPlayer<bool>,
}

/// Effect flowing from the `ChunkMgr` to the rest of the server.
#[derive(Debug)]
pub enum ChunkMgrEffect {
    /// Set in motion the process of loading or generating the chunk's state, so that
    /// `on_chunk_ready` is called in the future, unless aborted.
    RequestLoad {
        save_key: ChunkSaveKey,
        aborted: AbortHandle,
    },
    /// Chunk has entered the loaded state and been assigned a ci. Initialize it in other data
    /// structures.
    AddChunk {
        cc: Vec3<i64>,
        ci: usize,
        save_val: ChunkSaveVal,
        saved: bool,
    },
    /// Chunk has left the loaded state and its ci has been taken away. Remove it from other data
    /// structures. If the system was used correctly it should have already been removed from all
    /// clients.
    RemoveChunk {
        cc: Vec3<i64>,
        ci: usize,
    },
    /// A loaded chunk has been added to an active client and assigned for that client a clientside
    /// ci. Load the chunk onto the client.
    AddChunkToClient {
        cc: Vec3<i64>,
        ci: usize,
        pk: PlayerKey,
        clientside_ci: usize,
    },
    /// A loaded chunk has been removed from an active client which it previously was present in
    /// and its clientside ci for that client has been taken away. Tell the client to remove the
    /// chunk.
    RemoveChunkFromClient {
        cc: Vec3<i64>,
        ci: usize,
        pk: PlayerKey: ClientConnKey,
        clientside_ci: usize,
    },
}

impl ChunkMgr {
    /// Construct.
    pub fn new(chunk_loader: ChunkLoader) -> Self {
        Self::default()
    }

    /// Get the space of fully loaded chunks on the server.
    pub fn chunks(&self) -> &LoadedChunks {
        &self.chunks
    }

    /// Get the clientside ci for a given chunk and client, if the chunk is loaded in that client.
    pub fn chunk_to_clientside(&self, cc: Vec3<i64>, ci: usize, pk: PlayerKey) -> Option<usize> {
        self.chunk_player_clientside_ci.get(cc, ci)[ck]
    }

    /// Call upon a player being added to the world. Initializes it with no chunk client interests.
    ///
    /// This does _not_ require draining the effect queue.
    pub fn add_player(&mut self, pk: PlayerKey) {
        // initialize player state with defaults
        self.player_clientside_chunks.insert(pk, Default::default());
        self.player_add_chunk_mgr.insert(pk, ClientAddChunkMgr::new(&self.chunks));

        for (cc, ci) in self.chunks.iter() {
            self.chunk_player_clientside_ci.get_mut(cc, ci).insert(pk, None);
        }
    }

    /// Call upon a player being removed from the world. We rely on the caller to provide the
    /// current set of chunk client interests for this client.
    pub fn remove_player(
        &mut self,
        pk: PlayerKey,
        chunk_interests: impl IntoIterator<Item=Vec3<i64>>,
        players: &PlayerKeySpace,
    ) {
        // remove chunk interests, but without maintaining that player's per-player state in the
        // course of doing so, since we are about to remove that anyways
        for cc in chunk_interests {
            self.internal_remove_chunk_client_interest(pk, cc, players, false);
        }

        // remove player state
        self.player_clientside_chunks.remove(pk);
        self.player_add_chunk_mgr.remove(pk);

        for (cc, ci) in self.chunks.iter() {
            self.chunk_player_clientside_ci.get_mut(cc, ci).remove(pk);
        }

        for loading_chunk in self.loading_chunks.values_mut() {
            loading_chunk.interest.remove(pk);
        }
    }

    /// Increment the load request count for the given cc.
    pub fn incr_load_request_count(&mut self, cc: Vec3<i64>, players: &PlayerKeySpace) {
        if let Some(ci) = self.chunks.getter().get(cc) {
            // already loaded, just increment count
            let count = self.load_request_count.get_mut(cc, ci);
            *count = count.checked_add(1).unwrap();
        } else {
            match self.loading_chunks.entry(cc) {
                hash_map::Entry::Occupied(mut entry) => {
                    // already loading, just increment count
                    let count = &mut entry.get_mut().load_request_count;
                    *count = count.checked_add(1).unwrap();
                }
                hash_map::Entry::Vacant(entry) => {
                    // going from zero to non-zero; create it in the loading state
                    let aborted_1 = AbortGuard::new()
                    let aborted_2 = aborted_1.handle();
                    entry.insert(LoadingChunk {
                        aborted: aborted_1,
                        load_request_count: 1.try_into().unwrap(),
                        interest: players.new_mapped_per_player(|_| false),
                    });

                    // trigger it to be loaded
                    self.effects.push_back(ChunkMgrEffect::RequestLoad {
                        save_key: ChunkSaveKey { cc },
                        aborted: aborted_2,
                    });
                }
            }
        }
    }

    /// Decrement the load request count for the given cc. Must correspond to a previous direct
    /// call to incr_load_request_count.
    pub fn decr_load_request_count(&mut self, cc: Vec3<i64>, players: &PlayerKeySpace) {
        if let Some(ci) = self.chunks.getter().get(cc) {
            // chunk is loaded, try to decrement count
            let count = self.load_request_count.get_mut(cc, ci);
            if let Some(decremented) = NonZeroU64::new(count.get() - 1) {
                // it doesn't reach 0
                *count = decremented;
            } else {
                // it does reach 0, so remove it
                self.remove_chunk(cc, ci, players);
            }
        } else {
            // get
            let mut entry =
                match self.loading_chunks.entry(cc) {
                    hash_map::Entry::Occupied(entry) => entry,
                    hash_map::Entry::Vacant(_) => {
                        debug_assert!(false, "decr_load_request_count, but it's not loaded or loading");
                        return;
                    }
                };

            // chunk is loading, try to decrement count
            let count = &mut entry.get_mut().load_request_count;
            if let Some(decremented) = NonZeroU64::new(count.get() - 1) {
                // it doesn't reach 0
                *count = decremented;
            } else {
                // it does reach 0, remove it and abort loading
                let loading_chunk = entry.remove();
                loading_chunk.abort_handle.abort();
            }
        }
    }

    /// Add a chunk client interest for the given cc and player. Must not do redundantly.
    /// Automatically increments the load request count for that cc.
    pub fn add_chunk_client_interest(
        &mut self,
        pk: PlayerKey,
        cc: Vec3<i64>,
        players: &PlayerKeySpace,
    ) {
        // first, increment the load request count
        self.incr_load_request_count(cc, players);

        if let Some(ci) = self.chunks.getter().get(cc) {
            // if the chunk is already loaded, add it to the client, modulo add chunk to client
            // rate limiting
            self.maybe_add_chunk_to_client(cc, ci, pk);
        } else {
            // if the chunk is still being loaded, mark the client as interested in it when it's
            // ready.
            //
            // the previous incr_load_request_count call should ensure the entry is present.
            self.loading_chunks.get_mut(&cc).unwrap().interest[pk] = true;
        }
    }

    /// Remove the chunk client interest for the given cc and player. Must not do redundantly.
    /// Automatically decrements the load request count for that cc.
    pub fn remove_chunk_client_interest(
        &mut self,
        pk: PlayerKey,
        cc: Vec3<i64>,
        players: &PlayerKeySpace,
    ) {
        self.internal_remove_chunk_client_interest(pk, cc, players, true);
    }

    /// Permit `amount` additional "add chunk to client" operations to occur to the client.
    pub fn increase_client_add_chunk_budget(&mut self, pk: PlayerKey, amount: u32) {
        self.add_chunk_mgr[pk].increase_budget(amount);
        while let Some((cc, ci)) = self.add_chunk_mgr[pk].poll_queue() {
            self.add_chunk_to_client(cc, ci, pk);
        }
    }

    /// Call upon the result of a previously triggered chunk loading oepration being ready, unless
    /// aborted.
    pub fn on_chunk_ready(
        &mut self,
        save_key: ChunkSaveKey,
        save_val: ChunkSaveVal,
        saved: bool,
        players: &PlayerKeySpace,
    ) {
        // prepare
        let ChunkSaveKey { cc } = save_key;

        // remove from loading chunks
        let loading_chunk = self.loading_chunks.remove(&cc).unwrap();

        // add to loaded chunks
        let ci = self.chunks.add(cc);
        
        // initialize in corresponding structures
        self.chunk_player_clientside_ci.add(cc, ci, players.new_mapped_per_player(|_| None));
        self.chunk_load_request_count.add(cc, ci, loading_chunk.load_request_count);
        for pk in players.iter() {
            self.player_add_chunk_mgr[pk].on_add_chunk(cc, ci);
        }

        // add the chunk to the rest of the server.
        //
        // it's maybe a bit strange how we're just directly threading through save val into the
        // effect here and only here, but I think it does feel like less surprising of an API.
        self.effects.push_back(ChunkMgrEffect::AddChunk { cc, ci, save_val, saved });

        // for each client interested in it, add it to that client, modulo add chunk to client rate
        // limiting
        for pk in players.iter() {
            if loading_chunk.interest[pk] {
                self.maybe_add_chunk_to_client(cc, ci, pk);
            }
        }
    }

    // internal method for when it's time to remove a loaded chunk. assumes not loaded for any
    // players.
    fn remove_chunk(&mut self, cc: Vec3<i64>, ci: usize, players: &PlayerKeySpace) {
        self.chunks.remove(cc);
        self.chunk_player_clientside_ci.remove(cc, ci);
        self.chunk_load_request_count.remove(cc, ci);
        for pk in players.iter() {
            self.player_add_chunk_mgr[pk].on_remove_chunk(cc, ci);
        }
        self.effects.push_back(ChunkMgrEffect::RemoveChunk { cc, ci });
    }

    // internal method for when it's time to add a loaded chunk to a client, or enqueue it to be
    // added to the client once the client's add chunk rate limitation permits it.
    fn maybe_add_chunk_to_client(&mut self, cc: Vec3<i64>, ci: usize, pk: PlayerKey) {
        if self.add_chunk_mgr[pk].maybe_add_chunk_to_client(cc, ci) {
            self.add_chunk_to_client(cc, ci, pk);
        }
    }

    // internal method for when it's time to add a loaded chunk to a client.
    fn add_chunk_to_client(&mut self, cc: Vec3<i64>, ci: usize, pk: PlayerKey) {
        let clientside_ci = self.player_clientside_chunks[pk].insert(());
        self.chunk_player_clientside_ci.get_mut(cc, ci)[pk] = Some(clientside_ci);
        self.effects.push_back(ChunkMgrEffect::AddChunkToClient { cc, ci, pk, clientside_ci });
    }

    // internal method to remove a chunk client interest that lets the caller specify whether to
    // bother updating the state of the client (as opposed to just the state of the chunk).
    // update_client is passed as false when this is triggered by that client disconnecting.
    fn internal_remove_chunk_client_interest(
        &mut self,
        pk: PlayerKey,
        cc: Vec3<i64>,
        players: &PlayerKeySpace,
        update_client: bool,
    ) {
        if update_client {
            if let Some(ci) = self.chunks.getter().get(cc) {
                // if the chunk was already loaded and added to the client
                let clientside_ci = self.chunk_player_clientside_ci.get_mut(cc, ci)[pk].take();

                if let Some(clientside_ci) = clientside_ci {
                    // then remove it from the client
                    self.clientside_chunks[pk].remove(clientside_ci);
                    self.effects.push_back(ChunkMgrEffect::RemoveChunkFromClient {
                        cc,
                        ci,
                        pk,
                        clientside_ci,
                    });
                } else {
                    // elsewise, it must be pending in the queue of chunks to be added to the
                    // client when rate limits permit it, so remove it from that queue
                    self.add_chunk_mgr[pk].remove_from_queue(cc, ci);
                }
            } else if let Some(loading_chunk) = self.loading_chunks.get_mut(&cc) {
                // if the chunk is still loading, simply un-mark the client's interest.
                loading_chunk.interest[pk] = false;
            }
        }

        // decrement the load request count. this will handle possibly unloading the chunk from the
        // server or aborting a pending load request.
        self.decr_load_request_count(cc, players);
    }
}
