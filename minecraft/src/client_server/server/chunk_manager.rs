
use chunk_data::*;
use crate::{
    client_server::server::chunk_loader::{
        ChunkLoader,
        ReadyChunk,
        LoadChunkAbortHandle,
    },
    util::{
        sparse_vec::SparseVec,
        sparse_flags::SparseFlags,
    },
};
use std::{
    num::NonZeroU64,
    collections::VecDeque,
};
use vek::*;


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
// - for a loaded chunk and client, iff a chunk client interest exists for
//   them, the client has that chunk loaded.
// - for a loading chunk and client, iff a chunk client interest exists for
//   them, a corresponding flag is set in loading_interested_clients


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
    // notable invariants:
    // - entries subsets of chunks
    // TODO: could be replaced with Slab<()>
    clientside_chunks: SparseVec<LoadedChunks>,

    // for each chunk, for each client which has it loaded, clientside ci.
    // notable invariants:
    // - reverse of clientside_chunks
    clientside_cis: PerChunk<SparseVec<usize>>,

    // for each chunk, whether that chunk's current state is completely saved.
    saved: PerChunk<bool>,

    // for each chunk, the load request count for that chunk.
    // notable invariants:
    // - load_request_count[ci] > 0 || !saved[ci]
    load_request_count: PerChunk<u64>,

    // set of chunks for which their loading was requested from chunk loader.
    // notable invariants:
    // - disjoint with chunks
    // TODO: could be replaced with HashMap where the val is a new struct
    loading_chunks: LoadedChunks,

    // for each loading chunk, handle to abort the loading of it.
    loading_abort_handle: PerChunk<LoadChunkAbortHandle>,

    // for each loading chunk, the load request count for that chunk.
    loading_load_request_count: PerChunk<NonZeroU64>,

    // for each loading chunk, for each client, whether there exists a chunk
    // client interest for that chunk and client
    // notable invariants:
    // - value flag positions subset of client keys
    loading_interested_clients: PerChunk<SparseFlags>,
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
    /// it from other data structures.
    RemoveChunk {
        cc: Vec3<i64>,
        ci: usize,
    },
    /// A loaded chunk has been added to an active client and assigned for that
    /// client a clientside ci. Tell the client to add the chunk.
    AddChunkToClient {
        cc: Vec3<i64>,
        ci: usize,
        client_key: usize,
        clientside_ci: usize,
    },
    /// A loaded chunk has been removed from an active client which it
    /// previously was present in and its clientside ci for that client has
    /// been taken away. Tell the client to remove the chunk.
    RemoveChunkFromClient {
        cc: Vec3<i64>,
        ci: usize,
        client_key: usize,
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
            clientside_chunks: SparseVec::new(),
            clientside_cis: PerChunk::new(),
            saved: PerChunk::new(),
            load_request_count: PerChunk::new(),
            loading_chunks: LoadedChunks::new(),
            loading_abort_handle: PerChunk::new(),
            loading_load_request_count: PerChunk::new(),
            loading_interested_clients: PerChunk::new(),
        }
    }

    /// Add a client with no loaded chunks or chunk interests.
    ///
    /// This does _not_ require draining the effect queue.
    pub fn add_client(&mut self, client_key: usize) {
        self.clientside_chunks.set(client_key, LoadedChunks::new());
    }

    /// Remove a client. We rely on the caller to provide the current set of
    /// chunk client interests for this client.
    pub fn remove_client(
        &mut self,
        client_key: usize,
        chunk_interests: impl IntoIterator<Item=Vec3<i64>>,
    ) {
        // remove chunk interests, but don't bother updating data structures
        // for that client or producing effects to update that client.
        for cc in chunk_interests {
            self.internal_remove_chunk_client_interest(client_key, cc, false);
        }

        // remove data structures for that client
        self.clientside_chunks.remove(client_key);
    }

    /// Increment the load request count for the given cc.
    ///
    /// This does _not_ require draining the effect queue.
    pub fn incr_load_request_count(&mut self, cc: Vec3<i64>) {
        if let Some(ci) = self.chunks.getter().get(cc) {
            // already loaded, just increment count
            *self.load_request_count.get_mut(cc, ci) += 1;
        } else if let Some(loading_ci) = self.loading_chunks.getter().get(cc) {
            // already loading, just increment count
            let count = self.loading_load_request_count.get_mut(cc, loading_ci);
            *count = count.checked_add(1).unwrap();
        } else {
            // going from zero to non-zero; create it in the loading state
            let abort_handle = self.chunk_loader.request(cc);
            let loading_ci = self.loading_chunks.add(cc);
            self.loading_abort_handle.add(cc, loading_ci, abort_handle);
            self.loading_load_request_count.add(cc, loading_ci, 1.try_into().unwrap());
            self.loading_interested_clients.add(cc, loading_ci, SparseFlags::new());
        }
    }

    /// Decrement the load request count for the given cc. Must correspond to
    /// a previous direct call to incr_load_request_count.
    pub fn decr_load_request_count(&mut self, cc: Vec3<i64>) {
        if let Some(ci) = self.chunks.getter().get(cc) {
            // already loaded
            // decrement count
            let count = self.load_request_count.get_mut(cc, ci);
            debug_assert!(*count > 0, "decr_load_request_count, but it's already 0");
            *count -= 1;

            // if count reached 0 and already saved, remove it immediately.
            if *count == 0 && !self.saved.get(cc, ci) {
                self.remove_chunk(cc, ci);
            }
        } else if let Some(loading_ci) = self.loading_chunks.getter().get(cc) {
            // already loading, try to decrement count
            let count = self.loading_load_request_count.get_mut(cc, loading_ci);
            if let Some(decremented) = NonZeroU64::new(count.get() - 1) {
                // it doesn't reach 0
                *count = decremented;
            } else {
                // it does reach 0, abort loading and remove it
                self.loading_chunks.remove(cc);
                let abort_handle = self.loading_abort_handle.remove(cc, loading_ci);
                abort_handle.abort();
                self.loading_load_request_count.remove(cc, loading_ci);
                self.loading_interested_clients.remove(cc, loading_ci);
            }
        } else {
            debug_assert!(false, "decr_load_request_count, but it's not loaded or loading");
        }
    }

    /// Add a chunk client interest for the given cc and client. Must not do
    /// redundantly. Also automatically increments the load request count for
    /// that cc.
    pub fn add_chunk_client_interest(&mut self, client_key: usize, cc: Vec3<i64>) {
        // first, increment the load request count
        self.incr_load_request_count(cc);

        if let Some(ci) = self.chunks.getter().get(cc) {
            // if the chunk is already loaded, add it to the client
            self.add_chunk_to_client(cc, ci, client_key);
        } else if let Some(loading_ci) = self.loading_chunks.getter().get(cc) {
            // if the chunk is still being loaded, mark the client as
            // interested in it when it's ready
            self.loading_interested_clients.get_mut(cc, loading_ci).set(client_key);
        } else {
            // the incr_load_request_count should prevent this
            unreachable!()
        }
    }

    /// Remove the chunk client interest for the given cc and client. Must not
    /// do redundantly. Also automatically decrements the load request count
    /// for that cc.
    pub fn remove_chunk_client_interest(&mut self, client_key: usize, cc: Vec3<i64>) {
        self.internal_remove_chunk_client_interest(client_key, cc, true);
    }

    /// Call upon receiving a ready chunk event.
    pub fn on_ready_chunk(&mut self, ready_chunk: ReadyChunk) {
        let cc = ready_chunk.cc;

        // remove from loading chunks
        let loading_ci = self.loading_chunks.remove(cc);
        
        // remove from corresponding structures
        self.loading_abort_handle.remove(cc, loading_ci);
        let count = self.loading_load_request_count.remove(cc, loading_ci);
        let interested_clients = self.loading_interested_clients.remove(cc, loading_ci);

        // add to loaded chunks
        let ci = self.chunks.add(cc);
        
        // initialize in corresponding structures
        self.clientside_cis.add(cc, ci, SparseVec::new());
        self.saved.add(cc, ci, ready_chunk.saved);
        self.load_request_count.add(cc, ci, count.into());

        // tell the user to add the chunk
        self.effects.push_back(Effect::AddChunk { // TODO: could simplify this
            ready_chunk,
            ci,
        });

        // for each client interested in it, add it to that client
        for client_key in interested_clients.iter() {
            self.add_chunk_to_client(cc, ci, client_key);
        }
    }

    /// Mark a loaded chunk as saved.
    pub fn mark_saved(&mut self, cc: Vec3<i64>, ci: usize) {
        if *self.load_request_count.get(cc, ci) > 0 {
            // simply mark it as saved
            *self.saved.get_mut(cc, ci) = true;
        } else {
            // waiting for it to be saved was the only reason it was still
            // loaded, so just remove it.
            self.remove_chunk(cc, ci);
        }
    }

    /// Mark a loaded chunk as unsaved.
    ///
    /// This does _not_ require draining the effect queue.
    pub fn mark_unsaved(&mut self, cc: Vec3<i64>, ci: usize) {
        *self.saved.get_mut(cc, ci) = false;
    }

    // internal method for when it's time to remove a loaded chunk.
    fn remove_chunk(&mut self, cc: Vec3<i64>, ci: usize) {
        self.chunks.remove(cc);
        self.clientside_cis.remove(cc, ci);
        self.saved.remove(cc, ci);
        self.load_request_count.remove(cc, ci);
        self.effects.push_back(Effect::RemoveChunk { cc, ci });
    }

    // internal method for when it's time to add a loaded chunk to a client.
    fn add_chunk_to_client(&mut self, cc: Vec3<i64>, ci: usize, client_key: usize) {
        let clientside_ci = self.clientside_chunks[client_key].add(cc);
        self.clientside_cis.get_mut(cc, ci).set(client_key, clientside_ci);
        self.effects.push_back(Effect::AddChunkToClient {
            cc,
            ci,
            client_key,
            clientside_ci,
        });
    }

    // internal method to remove a chunk client interest that lets the caller
    // specify whether to bother updating the state of the client (as opposed
    // to just the state of the chunk).
    fn internal_remove_chunk_client_interest(
        &mut self,
        client_key: usize,
        cc: Vec3<i64>,
        update_client: bool,
    ) {
        if let Some(ci) = self.chunks.getter().get(cc) {
            // if the chunk is already loaded, remove it from the client.
            let clientside_ci = self.clientside_cis.get_mut(cc, ci).remove(client_key);
            if update_client {
                self.clientside_chunks[client_key].remove(cc);
                self.effects.push_back(Effect::RemoveChunkFromClient {
                    cc,
                    ci,
                    client_key,
                    clientside_ci,
                });
            }
        } else if let Some(loading_ci) = self.loading_chunks.getter().get(cc) {
            // if the chunk is still loading, un-mark the client's interest.
            self.loading_interested_clients.get_mut(cc, loading_ci).unset(client_key);
        } 

        // decrement the load request count. this will handle possibly unloading
        // the chunk from the server.
        self.decr_load_request_count(cc);
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

    /// Given a chunk, iterate through all clients which have it loaded, along
    /// with their clientside ci for the chunk.
    pub fn iter_chunk_clientsides<'c>(&'c self, cc: Vec3<i64>, ci: usize) -> impl Iterator<Item=ClientsideChunk> + 'c {
        self.clientside_cis.get(cc, ci).iter()
            .map(|(client_key, &clientside_ci)| ClientsideChunk { client_key, clientside_ci })
    }
}

/// Client key and clientside ci for a chunk.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ClientsideChunk {
    pub client_key: usize,
    pub clientside_ci: usize,
}
