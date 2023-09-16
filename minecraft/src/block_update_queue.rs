
use chunk_data::{
    PerChunk,
    PerTileU1,
    Getter,
    TileKey,
};
use std::collections::VecDeque;
use vek::*;


/// Queue of block updates to do at certain gtcs.
#[derive(Debug, Clone)]
pub struct BlockUpdateQueue {
    tile_queued: PerChunk<PerTileU1>,
    queue: VecDeque<TileKey>,
}

impl BlockUpdateQueue {
    pub fn new() -> Self {
        BlockUpdateQueue {
            tile_queued: PerChunk::new(),
            queue: VecDeque::new(),
        }
    }

    /// Enqueue a block update at the given gtc, if that tile is loaded into
    /// the world, and there's not already a block update enqueued for that
    /// tile.
    pub fn enqueue(&mut self, gtc: Vec3<i64>, getter: &Getter) {
        if let Some(tile) = getter.gtc_get(gtc) {
            self.enqueue_tile_key(tile);
        }
    }

    /// Enqueue a block update at the given tile key, it having been pre-looked
    /// up and confirmed to currently exist ni the world.
    pub fn enqueue_tile_key(&mut self, tile: TileKey) {
        if tile.get(&self.tile_queued) == 0 {
            tile.set(&mut self.tile_queued, 1);
            self.queue.push_back(tile);
        }
    }

    pub fn pop(&mut self) -> Option<TileKey> {
        if let Some(tile) = self.queue.pop_front() {
            tile.set(&mut self.tile_queued, 0);
            Some(tile)
        } else {
            None
        }
    }

    /// See `chunk_data::PerChunk::add_chunk`.
    pub fn add_chunk(&mut self, cc: Vec3<i64>, ci: usize) {
        self.tile_queued.add(cc, ci, PerTileU1::new());
    }

    /// See `chunk_data::PerChunk::remove_chunk`.
    #[allow(dead_code)]
    pub fn remove_chunk(&mut self, cc: Vec3<i64>, ci: usize) {
        assert!(
            self.queue.is_empty(),
            "remove_chunk to non-empty block update queue",
        );
        self.tile_queued.remove(cc, ci);
    }
}
