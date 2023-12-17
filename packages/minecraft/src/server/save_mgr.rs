//! See `SaveMgr`.

use super::chunk_mgr;
use std::cell::Cell;


/// Manages the saving of world data.
pub struct SaveMgr {
    save: SaveFile,
    last_tick_saved: u64,
    
    chunk_saved: PerChunk<Cell<bool>>,
    unsaved_chunks: Vec<(Vec3<i64>, usize)>,

    player_saved: PerPlayer<bool>,
    unsaved_players: Vec<ClientConnKey>,
}

impl SaveMgr {
    /// Construct around a save file.
    pub fn new(save: SaveFile) -> SaveMgr {
        SaveMgr {
            save,
            last_tick_saved: 0,

            chunk_saved: PerChunk::new(),
            unsaved_chunks: Vec::new(),

            player_saved: PerPlayer::new(),
            unsaved_players: Vec::new(),
        }
    }

    /// Whether the given chunk is fully saved.
    pub fn is_chunk_saved(&self, cc: Vec3<i64>, ci: usize) -> bool {
        self.chunk_saved.get(cc, ci).get()
    }

    /// Whether the given player is fully saved.
    pub fn is_player_saved(&self, ck: ClientConnKey) -> bool {
        self.player_saved[ck].get()
    }

    /// Mark the given chunk as no longer fully saved.
    ///
    /// This should be called whenever a chunk's saveable state is mutated.
    pub fn mark_chunk_unsaved(&self, cc: Vec3<i64>, ci: usize) {
        if self.chunk_saved.get(cc, ci).replace(false) {
            self.unsaved_chunks.push((cc, ci));
        }
    }

    /// Mark the given player as no longer fully saved.
    ///
    /// This should be called whenever a player's saveable state is mutated.
    pub fn mark_player_unsaved(&self, ck: ClientConnKey) -> bool {
        if self.player_saved[ck].replace(false) {
            self.unsaved_players.push(ck);
        }
    }

    /// Call upon a chunk being loaded into the world.
    ///
    /// A chunk which was loaded from the save file starts as saved, whereas a chunk which was
    /// generated new stars as not saved.
    pub fn add_chunk(&self, cc: Vec3<i64>, ci: usize, saved: bool) {
        self.chunk_saved.add(cc, ci, Cell::new(saved));
    }

    /// Call upon a player joining the world.
    pub fn add_player(&self, ck: ClientConnKey) {
        self.player_saved.insert(ck, false);
    }

    /// Call upon a chunk being removed from the world. Assumes that it is saved.
    pub fn remove_chunk(&self, cc: Vec3<i64>, ci: usize) {
        self.chunk_saved.remove(cc, ci);
    }

    /// Call upon a player being removed from the world. Assumes that it is saved.
    pub fn remove_player(&self, ck: ClientConnKey) {
        self.player_saved.remove(ck);
    }

    pub fn maybe_save()
}

impl chunk_mgr::IsChunkSaved for SaveMgr {
    fn is_chunk_saved(&self, cc: Vec3<i64>, ci: usize) -> bool {
        Self::is_chunk_saved(self, cc, ci)
    }
}
