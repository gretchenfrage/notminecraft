
use chunk_data::*;
use std::collections::VecDeque;
use vek::*;


#[derive(Debug, Clone, Default)]
pub struct BlockUpdateQueue {
    tile_queued: PerChunk<PerTileBool>,
    queue: VecDeque<TileKey>,
}

impl BlockUpdateQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, tile: TileKey) {
        if !tile.get(&self.tile_queued) {
            tile.set(&mut self.tile_queued, true);
            self.queue.push_back(tile);
        }
    }

    pub fn push_gtc(&mut self, gtc: Vec3<i64>, getter: &Getter) {
        if let Some(tile) = getter.gtc_get(gtc) {
            self.push(tile);
        }
    }

    pub fn push_gtc_and_neighbors(&mut self, gtc: Vec3<i64>, getter: &Getter) {
        self.push_gtc(gtc, getter);
        for face in FACES {
            self.push_gtc(gtc + face.to_vec(), getter);
        }
    }

    pub fn pop(&mut self) -> Option<TileKey> {
        if let Some(tile) = self.queue.pop_front() {
            tile.set(&mut self.tile_queued, false);
            Some(tile)
        } else {
            None
        }
    }

    pub fn add_chunk(&mut self, cc: Vec3<i64>, ci: usize) {
        self.tile_queued.add(cc, ci, PerTileBool::new());
    }

    pub fn remove_chunk(&mut self, cc: Vec3<i64>, ci: usize) {
        assert!(self.queue.is_empty(), "remove_chunk to non-empty block update queue");
        self.tile_queued.remove(cc, ci);
    }
}
