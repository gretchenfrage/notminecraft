//! Storage of data per player loaded in the client.
//!
//! Not to be confused with the analogous server-side module.

use crate::{
    util_per_thing::*,
    message::DownPlayerIdx,
};
use std::ops::{Index, IndexMut};
use anyhow::*;


/// Manages the client-side set of loaded player keys.
#[derive(Debug, Clone, Default)]
pub struct PlayerKeySpace(ThingKeySpace);

/// Storage of `T` per client-side loaded player.
#[derive(Debug, Clone)]
pub struct PerPlayer<T>(PerThing<T>);

/// Key into client-side `PerPlayer<T>`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PlayerKey(ThingKey);

impl PlayerKeySpace {
    /// Construct in the default state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Call upon receiving an `AddPlayer` message from the server.
    ///
    /// Validates it and adds it to the key space, returning the allocated player key. This should
    /// be followed by adding to all `PerPlayer` structures.
    pub fn on_add_player(&mut self, player_idx: DownPlayerIdx) -> Result<PlayerKey> {
        let pk = self.0.add();
        ensure!(pk.get_idx() == player_idx.0, "server add player did not follow slab pattern");
        Ok(PlayerKey(pk))
    }

    /// Call upon receiving a `RemovePlayer` message from the server.
    ///
    /// Validates it and removes it from the key space, returning the removed player key. This
    /// should be followed by removing from all `PerPlayer` structures.
    pub fn on_remove_player(&mut self, player_idx: DownPlayerIdx) -> Result<PlayerKey> {
        let pk = self.lookup(player_idx)?;
        self.0.remove(pk.0);
        Ok(pk)
    }

    /// Look up a currently active player idx received from the server.
    ///
    /// Validates it and returns the corresponding player key, which is sort of "more hydrated". 
    pub fn lookup(&self, player_idx: DownPlayerIdx) -> Result<PlayerKey> {
        self.0.idx_to_key(player_idx.0)
            .map(PlayerKey)
            .ok_or_else(|| anyhow!("server referenced invalid player idx {}", player_idx.0))
    }

    /// Iterate through all current player keys.
    pub fn iter<'a>(&'a self) -> impl Iterator<Item=PlayerKey> + 'a {
        self.0.iter().map(PlayerKey)
    }

    /// Construct a new `PerPlayer` using `f` to populate entries for existing keys.
    pub fn new_per_player<T, F: FnMut(PlayerKey) -> T>(&self, mut f: F) -> PerPlayer<T> {
        PerPlayer(self.0.new_per(move |pk| f(PlayerKey(pk))))
    }
}

impl<T> PerPlayer<T> {
    /// Construct empty.
    pub fn new() -> Self {
        Self(Default::default())
    }

    /// Insert an entry for the given player key.
    ///
    /// This should follow a call to `PlayerKeySpace.add`.
    pub fn insert(&mut self, pk: PlayerKey, val: T) {
        self.0.insert(pk.0, val);
    }

    /// Remove the entry for the given player key.
    ///
    /// This should correspond to a call to `PlayerKeySpace.remove`.
    pub fn remove(&mut self, pk: PlayerKey) -> T {
        self.0.remove(pk.0)
    }

    /// Get by shared reference.
    pub fn get(&self, pk: PlayerKey) -> &T {
        self.0.get(pk.0)
    }

    /// Get by mutable reference.
    pub fn get_mut(&mut self, pk: PlayerKey) -> &mut T {
        self.0.get_mut(pk.0)
    }
}

impl<T> Default for PerPlayer<T> {
    fn default() -> Self {
        Self::new()
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
