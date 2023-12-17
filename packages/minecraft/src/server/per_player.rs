//! Storage of data per each player connected to the server.

use std::ops::{Index, IndexMut};
use slab::Slab;


/// Manages the allocation of `PlayerKey` in a slab pattern.

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
pub struct PlayerKeySpace {
    slab: Slab<u64>,
    ctr: u64,
}

/// Storage of `T` per player connected to the server.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
pub struct PerPlayer<T>(Slab<(T, u64)>);

/// Key into `PerPlayer<T>`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PlayerKey {
    idx: usize,
    ctr: u64,
}

impl<T> PlayerKeySpace<T> {
    /// Construct empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocate a new player key.
    ///
    /// This should be followed by adding to all `PerPlayer` structures.
    pub fn add(&mut self) -> PlayerKey {
        let ctr = self.ctr;
        let idx = self.slab.insert(ctr);
        self.ctr = self.ctr.wrapping_add(1);
        PlayerKey { idx, ctr }
    }

    /// Remove a player key.
    ///
    /// This should be matched with removing from all `PerPlayer` structures.
    pub fn remove(&mut self, key: PlayerKey) {
        let ctr2 = self.slab.remove(key.idx);
        debug_assert_eq!(ctr2, key.ctr, "PlayerKeySpace.remove ctr mismatch");
    }

    /// Iterate through all current player keys.
    pub fn iter<'a>(&'a self) -> impl Iterator<Item=PlayerKey> + 'a {
        self.slab.iter().map(|(idx, &ctr)| PlayerKey { idx, ctr })
    }

    /// Construct a new `PerPlayer` structure using `f` to populate entries for existing player
    /// keys.
    pub fn new_per_player<T, F: FnMut(PlayerKey) -> T>(&self, f: F) -> PerPlayer<T> {
        PerPlayer(self.slab.new_mapped(|idx, &ctr| (f(PlayerKey { idx, ctr }), ctr)))
    }
}

impl<T> PerPlayer<T> {
    /// Construct empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert an entry for the given player key.
    ///
    /// This should follow a call to `PlayerKeySpace.add`.
    pub fn insert(&mut self, key: PlayerKey, val: T) {
        let idx2 = self.0.insert((val, key.ctr));
        debug_assert_eq!(idx2, key.idx, "PerPlayer.insert idx mismatch");
    }

    /// Remove the entry for the given player key.
    ///
    /// This should correspond to a call to `PlayerKeySpace.remove`.
    pub fn remove(&mut self, key: PlayerKey) -> T {
        let (val, ctr2) = self.0.remove(key.idx);
        debug_assert_eq!(ctr2, key.ctr, "PerPlayer.remove ctr mismatch");
    }

    /// Get by shared reference.
    ///
    /// Panic on failure.
    pub fn get(&self, key: PlayerKey) -> &T {
        let &(ref val, ctr2) = &self.0[key.idx];
        debug_assert_eq!(ctr2, key.ctr, "PerPlayer.get ctr mismatch");
        val
    }

    /// Get by mutable reference.
    ///
    /// Panic on failure.
    pub fn get_mut(&mut self, key: PlayerKey) -> &mut T {
        let &mut (ref mut val, ctr2) = &mut self.0[key.idx];
        debug_assert_eq!(ctr2, key.ctr, "PerPlayer.get ctr mismatch");
        val
    }
}

impl<T> Index<PlayerKey> for PerPlayer<T> {
    type Output = T;

    fn index(&self, pk: PlayerKey) -> &T {
        self.get(pk)
    }
}

impl<T> IndexMut<PlayerKey> for PerPlayer<T> {
    fn index_mut(&mut self, pk: PlayerKey) -> &mut T {
        self.get_mut(pk)
    }
}
