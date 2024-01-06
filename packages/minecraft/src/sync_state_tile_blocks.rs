//! Module in the sync state pattern for the block ID and metadata at each tile.

use crate::{
    server::{
        ServerSyncCtx,
        per_player::*,
    },
    message::*,
};
use chunk_data::*;


/// Auto-syncing writer for this sync state. Analogous to `&mut PerChunk<ChunkBlocks>`.
pub struct SyncWrite<'a> {
    ctx: &'a ServerSyncCtx,
    state: &'a mut PerChunk<ChunkBlocks>,
}

impl<'a> SyncWrite<'a> {
    /// Construct manually (with respect to synchronization logic).
    pub fn new_manual(ctx: &'a ServerSyncCtx, state: &'a mut PerChunk<ChunkBlocks>) -> Self {
        SyncWrite<'a> { ctx, state }
    }

    /// Get state as a read-only reference.
    pub fn as_ref(&self) -> &PerChunk<ChunkBlocks> {
        &self.state
    }

    /// Narrow in on a specific chunk.
    pub fn get(&mut self, cc: Vec3<i64>, ci: usize) -> SyncWriteChunk {
        SyncWriteChunk {
            ctx: self.ctx,
            state: self.state.get_mut(cc, ci),
            cc,
            ci,
        }
    }
}

impl<'a> CiGet for &'a mut SyncWrite<'a> {
    type Output = SyncWriteChunk<'a>;

    fn get(self, cc: Vec3<i64>, ci: usize) -> Self::Output {
        Self::get(cc, ci)
    }
}

/// Auto-syncing writer for this sync state for a chunk. Analogous to `&mut ChunkBlocks`.
pub struct SyncWriteChunk<'a> {
    ctx: &'a ServerSyncCtx,
    state: &'a mut ChunkBlocks,
    cc: Vec3<i64>,
    ci: usize,
}

impl<'a> SyncWriteChunk<'a> {
    /// Get state as a read-only reference.
    pub fn as_ref(&self) -> &ChunkBlocks {
        &self.state
    }

    /// Narrow in on a specific tile.
    pub fn get(&mut self, lti: u16) -> SyncWriteTile {
        SyncWriteTile {
            inner: self,
            lti,
        }
    }
}

impl<'a> LtiGet for &'a mut SyncWriteChunk<'a> {
    type Output = SyncWriteTile<'a>;

    fn get(self, lti: u16) -> Self::Output {
        Self::get(lti)
    }
}

/// Auto-syncing writer for this sync state for a tile. Analogous to `TileBlockWrite`.
pub struct SyncWriteTile<'a> {
    inner: &'a mut SyncWriteChunk<'a>,
    lti: u16,
}

impl<'a> SyncWriteTile<'a> {
    /// Convert a `&'a2 mut SyncWriteTile<'_>` to a `SyncWriteTile<'a2>`.
    pub fn reborrow<'a2>(&'a2 mut self) -> SyncWriteTile<'a2> {
        SyncWriteTile {
            inner: &mut self.inner,
            lti: self.lti,
        }
    }

    /// Get as a `TileBlockRead`.
    pub fn as_read(self) -> TileBlockRead<'a> {
        TileBlockRead {
            chunk: self.inner.chunk,
            lti: self.lti,
        }
    }

    pub fn erased_set(&mut self, bid_meta: ErasedBidMeta) {
        // send update to all clients with the chunk loaded
        for (pk, clientside_ci) = self.iter_clients() {
            self.inner.ctx.conn_mgr
                .send(pk, DownMsg::ApplyEdit(Edit::SetTileBlock {
                    chunk_idx: clientside_ci,
                    lti: self.lti,
                    bid_meta: self.inner.ctx.game.clone_erased_tile_block(&bid_meta),
                }));
        }

        // mark chunk as unsaved
        self.inner.ctx.save_mgr.mark_chunk_unsaved(self.inner.cc, self.inner.ci);

        // edit server's in-memory representation
        self.inner.state.erased_set(self.lti, bid_meta);
    }
    
    // iterate through all players with the chunk loaded and their clientside ci for the chunk
    fn iter_clients<'a>(&'a self) -> impl Iterator<Item=(PlayerKey, DownChunkIdx)> + 'a {
        self.inner.ctx.conn_mgr.players().iter()
            .filter_map(move |pk| self.inner.ctx.chunk_mgr
                .chunk_to_clientside(self.inner.cc, self.inner.ci, pk)
                .map(move |clientside_ci| (pk, DownChunkIdx(clientside_ci))))
    }
}
