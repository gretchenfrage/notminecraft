

mod client_add_chunk_manager;


use self::client_add_chunk_manager::ClientAddChunkManager;
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


// manages changes to the set of loaded chunks.
//
// external ontology:
// - chunks: the set of chunks currently fully loaded in the server, each with
//   a (serverside) ci.
//
// - clients: the set of clients currently logged in to the server, each with a
//   client key.
//
// - clientside chunk: each client has a set of chunks that are currently
//   loaded in that client. it is a subset of (serverside) chunks. it is a
//   different space of cis, so when the server is talking to a client about a
//   chunk it must translate from (serverside) ci to clientside ci.
//
// - load request count: each cc has a load request count, defaulting to 0. a
//   non-zero load request count indicates the desire for the chunk to be
//   in the loaded state, and a load request count of zero indicates the desire
//   for the chunk not to be in the loaded state. however, this transition is
//   not necessarily immediate in either direction.
//
// - chunk client interest: each client has some set of ccs it is interested
//   in having loaded. ccs can be added and removed from this set. when they
//   are, the load request count of that cc is incremented/decremented as well
//   automatically. the presence of a chunk client interest will cause that
//   chunk to be loaded into that client once it's loaded into the server, and
//   the absence of one will cause that chunk to be removed from the client if
//   was loaded.
//
// misc invariants:
// - for a loading chunk and client, iff a chunk client interest exists for
//   them, a corresponding flag is set in loading_interest


/// Manages the sets of loaded chunks for the server and its clients. Side
/// effects that the rest of the server should process are added to an internal
/// effects queue. After calling any methods on `ChunkManager` that takes
/// `&mut self`, events should be taken from `ChunkManager.effects` and
/// processed until exhausted, unless the specific method specifies that this
/// is not necessary.
pub struct ChunkManager {
    pub effects: VecDeque<Effect>,

    // responsible for asynchronously servicing requests to read a chunk from
    // the save file or generate it if it was never saved.
    chunk_loader: ChunkLoader,

    // set of chunks that are fully loaded
    chunks: LoadedChunks,

    // for each client, set of chunks that are loaded for that client.
    clientside_chunks: PerClientConn<Slab<()>>,

    // for each chunk, for each client which has it loaded, clientside ci.
    // notable invariants:
    // - reverse of clientside_chunks
    clientside_cis: PerChunk<PerClientConn<Option<usize>>>,

    // for each client, sub-manager for the limited rate at which additional
    // chunks can be added to it.
    add_chunk_mgr: PerClientConn<ClientAddChunkManager>,

    // for each chunk, whether that chunk's current state is completely saved.
    saved: PerChunk<bool>,

    // for each chunk, the load request count for that chunk.
    // notable invariants:
    // - load_request_count[ci] > 0 || !saved[ci]
    load_request_count: PerChunk<u64>,

    // set of chunks for which their loading was requested from chunk loader.
    // notable invariants:
    // - disjoint with chunks
    loading_chunks: HashMap<Vec3<i64>, LoadingChunk>,
}

// chunk pending being loaded by the chunk loader.
struct LoadingChunk {
    // handle to abort the loading of this chunk.
    abort_handle: LoadChunkAbortHandle,

    // the load request count for this chunk.
    load_request_count: NonZeroU64,

    // for each client, whether there exists a chunk client interest for
    // this chunks and that client.
    interest: PerClientConn<bool>,
}

#[derive(Debug)]
pub enum Effect {
    /// Chunk has entered the loaded state and been assigned a ci. Initialize
    /// it in other data structures.
    AddChunk {
        ready_chunk: ReadyChunk,
        ci: usize,
    },
    /// Chunk has left the loaded state and its ci has been taken away. Remove
    /// it from other data structures. If the system was used correctly it
    /// should have already been removed from all clients.
    RemoveChunk {
        cc: Vec3<i64>,
        ci: usize,
    },
    /// A loaded chunk has been added to an active client and assigned for that
    /// client a clientside ci. Tell the client to add the chunk.
    AddChunkToClient {
        cc: Vec3<i64>,
        ci: usize,
        ck: ClientConnKey,
        clientside_ci: usize,
    },
    /// A loaded chunk has been removed from an active client which it
    /// previously was present in and its clientside ci for that client has
    /// been taken away. Tell the client to remove the chunk.
    RemoveChunkFromClient {
        cc: Vec3<i64>,
        ci: usize,
        ck: ClientConnKey,
        clientside_ci: usize,
    },
}

impl ChunkManager {
    /// Construct with no chunks and no clients.
    pub fn new(chunk_loader: ChunkLoader) -> Self {
        ChunkManager {
            effects: VecDeque::new(),
            chunk_loader,
            chunks: LoadedChunks::new(),
            clientside_chunks: PerClientConn::new(),
            clientside_cis: PerChunk::new(),
            add_chunk_mgr: PerClientConn::new(),
            saved: PerChunk::new(),
            load_request_count: PerChunk::new(),
            loading_chunks: HashMap::new(),
        }
    }

    /// Add a client with no loaded chunks or chunk interests.
    ///
    /// This does _not_ require draining the effect queue.
    pub fn add_client(&mut self, ck: ClientConnKey) {
        self.clientside_chunks.insert(ck, Slab::new());
        self.add_chunk_mgr.insert(ck, ClientAddChunkManager::new(&self.chunks));

        for (cc, ci) in self.chunks.iter() {
            self.clientside_cis.get_mut(cc, ci).insert(ck, None);
        }
    }

    /// Remove a client. We rely on the caller to provide the current set of
    /// chunk client interests for this client.
    pub fn remove_client(
        &mut self,
        ck: ClientConnKey,
        chunk_interests: impl IntoIterator<Item=Vec3<i64>>,
        conn_states: &ConnStates,
    ) {
        // remove chunk interests, but don't bother updating data structures
        // for that client or producing effects to update that client.
        for cc in chunk_interests {
            self.internal_remove_chunk_client_interest(ck, cc, conn_states, false);
        }

        // remove data structures for that client
        self.clientside_chunks.remove(ck);
        self.add_chunk_mgr.remove(ck);
        
        for (cc, ci) in self.chunks.iter() {
            self.clientside_cis.get_mut(cc, ci).remove(ck);
        }

        for loading_chunk in self.loading_chunks.values_mut() {
            loading_chunk.interest.remove(ck);
        }
    }

    /// Increment the load request count for the given cc.
    ///
    /// This does _not_ require draining the effect queue.
    pub fn incr_load_request_count(
        &mut self,
        cc: Vec3<i64>,
        conn_states: &ConnStates,
    ) {
        if let Some(ci) = self.chunks.getter().get(cc) {
            // already loaded, just increment count
            *self.load_request_count.get_mut(cc, ci) += 1;
        } else {
            match self.loading_chunks.entry(cc) {
                hash_map::Entry::Occupied(mut entry) => {
                    // already loading, just increment count
                    let count = &mut entry.get_mut().load_request_count;
                    *count = count.checked_add(1).unwrap();
                }
                hash_map::Entry::Vacant(entry) => {
                    // going from zero to non-zero; create it in the loading state
                    let abort_handle = self.chunk_loader.request(cc);
                    entry.insert(LoadingChunk {
                        abort_handle,
                        load_request_count: 1.try_into().unwrap(),
                        interest: conn_states.new_mapped_per_client(|_| false),
                    });
                }
            }
        }
    }

    /// Decrement the load request count for the given cc. Must correspond to
    /// a previous direct call to incr_load_request_count.
    pub fn decr_load_request_count(&mut self, cc: Vec3<i64>, conn_states: &ConnStates) {
        if let Some(ci) = self.chunks.getter().get(cc) {
            // already loaded
            // decrement count
            let count = self.load_request_count.get_mut(cc, ci);
            debug_assert!(*count > 0, "decr_load_request_count, but it's already 0");
            *count -= 1;

            // if count reached 0 and already saved, remove it immediately.
            // (otherwise, will be removed when saved if count is still 0).
            if *count == 0 && !self.saved.get(cc, ci) {
                self.remove_chunk(cc, ci, conn_states);
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

            // already loading, try to decrement count
            let count = &mut entry.get_mut().load_request_count;
            if let Some(decremented) = NonZeroU64::new(count.get() - 1) {
                // it doesn't reach 0
                *count = decremented;
            } else {
                // it does reach 0, abort loading and remove it
                let loading_chunk = entry.remove();
                loading_chunk.abort_handle.abort();
            }
        }
    }

    /// Add a chunk client interest for the given cc and client. Must not do
    /// redundantly. Also automatically increments the load request count for
    /// that cc.
    pub fn add_chunk_client_interest(
        &mut self,
        ck: ClientConnKey,
        cc: Vec3<i64>,
        conn_states: &ConnStates,
    ) {
        // first, increment the load request count
        self.incr_load_request_count(cc, conn_states);

        if let Some(ci) = self.chunks.getter().get(cc) {
            // if the chunk is already loaded, add it to the client, modulo
            // add chunk to client rate limiting
            self.maybe_add_chunk_to_client(cc, ci, ck);
        } else {
            // if the chunk is still being loaded, mark the client as
            // interested in it when it's ready
            //
            // the incr_load_request_count should ensure the entry is present
            self.loading_chunks.get_mut(&cc).unwrap().interest[ck] = true;
        }
    }

    /// Permit `amount` additional "add chunk to client" operations to occur to
    /// the client.
    pub fn increase_client_add_chunk_budget(&mut self, ck: ClientConnKey, amount: u32) {
        self.add_chunk_mgr[ck].increase_budget(amount);
        while let Some((cc, ci)) = self.add_chunk_mgr[ck].poll_queue() {
            self.add_chunk_to_client(cc, ci, ck);
        }
    }

    /// Remove the chunk client interest for the given cc and client. Must not
    /// do redundantly. Also automatically decrements the load request count
    /// for that cc.
    pub fn remove_chunk_client_interest(
        &mut self,
        ck: ClientConnKey,
        cc: Vec3<i64>,
        conn_states: &ConnStates,
    ) {
        self.internal_remove_chunk_client_interest(ck, cc, conn_states, true);
    }

    /// Call upon receiving a ready chunk event.
    pub fn on_ready_chunk(&mut self, ready_chunk: ReadyChunk, conn_states: &ConnStates) {
        let cc = ready_chunk.cc;

        // remove from loading chunks
        let loading_chunk = self.loading_chunks.remove(&cc).unwrap();

        // add to loaded chunks
        let ci = self.chunks.add(cc);
        
        // initialize in corresponding structures
        self.clientside_cis.add(cc, ci, conn_states.new_mapped_per_client(|_| None));
        self.saved.add(cc, ci, ready_chunk.saved);
        self.load_request_count.add(cc, ci, loading_chunk.load_request_count.into());
        for ck in conn_states.iter_client() {
            self.add_chunk_mgr[ck].on_add_chunk(cc, ci);
        }

        // tell the user to add the chunk
        self.effects.push_back(Effect::AddChunk {
            // since AddChunk is only produced by on_ready_chunk, and is always
            // produced exactly once by on_ready_chunk, we could just make
            // on_ready_chunk return this value directly. however, using the
            // queue system makes the API more consistent.
            ready_chunk,
            ci,
        });

        // for each client interested in it, add it to that client, modulo add
        // chunk to client rate limiting
        for ck in conn_states.iter_client() {
            if loading_chunk.interest[ck] {
                self.maybe_add_chunk_to_client(cc, ci, ck);
            }
        }
    }

    /// Mark a loaded chunk as saved.
    pub fn mark_saved(&mut self, cc: Vec3<i64>, ci: usize, conn_states: &ConnStates) {
        if *self.load_request_count.get(cc, ci) > 0 {
            // simply mark it as saved
            *self.saved.get_mut(cc, ci) = true;
        } else {
            // waiting for it to be saved was the only reason it was still
            // loaded, so just remove it.
            self.remove_chunk(cc, ci, conn_states);
        }
    }

    /// Mark a loaded chunk as unsaved.
    ///
    /// This does _not_ require draining the effect queue.
    pub fn mark_unsaved(&mut self, cc: Vec3<i64>, ci: usize) {
        *self.saved.get_mut(cc, ci) = false;
    }

    // internal method for when it's time to remove a loaded chunk.
    fn remove_chunk(&mut self, cc: Vec3<i64>, ci: usize, conn_states: &ConnStates) {
        self.chunks.remove(cc);
        self.clientside_cis.remove(cc, ci);
        self.saved.remove(cc, ci);
        self.load_request_count.remove(cc, ci);
        for ck in conn_states.iter_client() {
            self.add_chunk_mgr[ck].on_remove_chunk(cc, ci);
        }
        self.effects.push_back(Effect::RemoveChunk { cc, ci });
    }

    // internal method for when it's time to add a loaded chunk to a client,
    // or enqueue it to be added to the client once the client's add chunk
    // rate limitation permits it.
    fn maybe_add_chunk_to_client(&mut self, cc: Vec3<i64>, ci: usize, ck: ClientConnKey) {
        if self.add_chunk_mgr[ck].maybe_add_chunk_to_client(cc, ci) {
            self.add_chunk_to_client(cc, ci, ck);
        }
    }

    // internal method for when it's time to add a loaded chunk to a client.
    fn add_chunk_to_client(&mut self, cc: Vec3<i64>, ci: usize, ck: ClientConnKey) {
        let clientside_ci = self.clientside_chunks[ck].insert(());
        self.clientside_cis.get_mut(cc, ci)[ck] = Some(clientside_ci);
        self.effects.push_back(Effect::AddChunkToClient {
            cc,
            ci,
            ck,
            clientside_ci,
        });
    }

    // internal method to remove a chunk client interest that lets the caller
    // specify whether to bother updating the state of the client (as opposed
    // to just the state of the chunk). update_client is passed as false when
    // this is triggered by that client disconnecting.
    fn internal_remove_chunk_client_interest(
        &mut self,
        ck: ClientConnKey,
        cc: Vec3<i64>,
        conn_states: &ConnStates,
        update_client: bool,
    ) {
        if update_client {
            if let Some(ci) = self.chunks.getter().get(cc) {
                // if the chunk is already loaded, remove it from the client if
                // it was added to the client
                let clientside_ci = self.clientside_cis.get_mut(cc, ci)[ck].take();

                if let Some(clientside_ci) = clientside_ci {
                    // if indeed it was added to the client, remove it from the
                    // client
                    self.clientside_chunks[ck].remove(clientside_ci);
                    self.effects.push_back(Effect::RemoveChunkFromClient {
                        cc,
                        ci,
                        ck,
                        clientside_ci,
                    });
                } else {
                    // elsewise, it must be pending in the queue of chunks to
                    // be added to the client when rate limits permit it, so
                    // remove it from that queue
                    self.add_chunk_mgr[ck].remove_from_queue(cc, ci);
                }
            } else if let Some(loading_chunk) = self.loading_chunks.get_mut(&cc) {
                // if the chunk is still loading, un-mark the client's interest.
                loading_chunk.interest[ck] = false;
            }
        }

        // decrement the load request count. this will handle possibly unloading
        // the chunk from the server or aborting a pending load request.
        self.decr_load_request_count(cc, conn_states);
    }

    /// Get the set of fully loaded chunks in the server.
    pub fn chunks(&self) -> &LoadedChunks {
        &self.chunks
    }

    /// Convenience method for `self.chunks().getter()`.
    pub fn getter(&self) -> Getter {
        self.chunks().getter()
    }

    /// Check whether a given chunk is marked as saved.
    pub fn is_saved(&self, cc: Vec3<i64>, ci: usize) -> bool {
        *self.saved.get(cc, ci)
    }

    /// Convenience method to iterate through all chunks which are marked as
    /// not saved.
    pub fn iter_unsaved<'c>(&'c self) -> impl Iterator<Item=(Vec3<i64>, usize, Getter<'c>)> + 'c {
        self.chunks().iter_with_getters()
            .filter(|&(cc, ci, _)| !self.is_saved(cc, ci))
    }

    /// Get the clientside ci for a given chunk and client if the chunk is
    /// loaded in that client.
    pub fn clientside_ci(&self, cc: Vec3<i64>, ci: usize, ck: ClientConnKey) -> Option<usize> {
        self.clientside_cis.get(cc, ci)[ck]
    }
}
