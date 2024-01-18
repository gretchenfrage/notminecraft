//! Wrapper around `LoadedChunks` for managing the client-side space of loaded chunks.

use crate::message::DownChunkIdx;
use chunk_data::*;
use anyhow::*;
use vek::*;


/// Wrapper around `LoadedChunks` for managing the client-side space of loaded chunks.
///
/// The raw `LoadedChunks` provides functionality for efficiently managing the relationship between
/// chunk coordinates and chunk indices. This wrapper introduces the additional complications of
/// the server sending the client messages with these indices, and the possibility of these indices
/// received from the server being invalid.
#[derive(Debug, Clone, Default)]
pub struct ClientLoadedChunks {
    inner: LoadedChunks,
    just_added: Option<(Vec3<i64>, usize)>,
}

impl ClientLoadedChunks {
    /// Construct in the default state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a getter. See `LoadedChunks.getter`.
    pub fn getter(&self) -> Getter {
        self.inner.getter()
    }

    /// Call upon receiving an `AddChunk` message from the server.
    ///
    /// Validates it and adds it to the chunk space. This should be followed by a call to
    /// `on_add_chunk_result`, to get the allocated ci and a getter with the chunk pre-cached, then
    /// by adding that to all `PerChunk` structures. The reason to split it up like that is because
    /// this takes `&mut self`, whereas `on_add_chunk_result` only takes `&self`.
    pub fn on_add_chunk(
        &mut self,
        chunk_idx: DownChunkIdx,
        cc: Vec3<i64>,
    ) -> Result<()> {
        let ci = self.inner.try_add(cc)
            .map_err(|e| match e {
                AddChunkError::AlreadyLoaded => anyhow!("server add chunk with cc collision"),
                AddChunkError::TooManyChunks => anyhow!("server added illegally many chunks"),
            })?;
        ensure!(ci == chunk_idx.0, "server add chunk did not follow slab pattern");
        self.just_added = Some((cc, ci));
        Ok(())
    }

    /// Get the output of the call to `on_add_chunk` that was just made. See `on_add_chunk`.
    pub fn on_add_chunk_result(&self) -> (usize, Getter) {
        let (cc, ci) = self.just_added.expect("invalid time to call just_added");
        (ci, self.inner.getter_pre_cached(cc, ci))
    }

    /// Call upon receiving a `RemoveChunk` message from the server.
    ///
    /// Validates it and removes it from the chunk space, returning the removed (cc, ci) pair. This
    /// should be followed by removing from all `PerChunk` structures.
    pub fn on_remove_chunk(&mut self, chunk_idx: DownChunkIdx) -> Result<(Vec3<i64>, usize)> {
        self.just_added = None;
        let cc = self.inner.ci_to_cc(chunk_idx.0)
            .ok_or_else(|| anyhow!("server remove invalid chunk idx {}", chunk_idx.0))?;
        self.inner.remove(cc);
        Ok((cc, chunk_idx.0))
    }

    /// Look up a currently active chunk idx received from the server.
    ///
    /// Validates it and returns the corresponding (cc, ci) pair, which is sort of "more hydrated",
    /// as well as a getter with the chunk pre-cached.
    pub fn lookup(&self, chunk_idx: DownChunkIdx) -> Result<(Vec3<i64>, usize, Getter)> {
        let cc = self.inner.ci_to_cc(chunk_idx.0)
            .ok_or_else(|| anyhow!("server referenced invalid chunk idx {}", chunk_idx.0))?;
        Ok((cc, chunk_idx.0, self.inner.getter_pre_cached(cc, chunk_idx.0)))
    }

    /// Iterate through all currently loaded chunks, including for each its (cc, ci) pair and a
    /// getter with the chunk pre-cached.
    pub fn iter<'a>(&'a self) -> impl Iterator<Item=(Vec3<i64>, usize, Getter)> + 'a {
        self.inner.iter_with_getters()
    }

    /// Construct a new `PerChunk` using `f` to populate entries for existing chunks.
    pub fn new_per_chunk<T, F>(&self, mut f: F) -> PerChunk<T>
    where
        F: FnMut(Vec3<i64>, usize, Getter) -> T,
    {
        self.inner.new_per_chunk_mapped(move |cc, ci| {
            f(cc, ci, self.inner.getter_pre_cached(cc, ci))
        })
    }
}
