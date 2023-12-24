//! Storage of data per each player connected to the server.
//!
//! The server has a concept both of players in general, and players which have joined the game.
//! Joined players is a subset of palyers in general. This module presents an API to reflect that
//! and allow both operating on and storing values for all players in general or only for joined
//! players.

use crate::util_per_thing::*;
use std::ops::{Index, IndexMut};


/// Manages the allocation of `PlayerKey` and `JoinedPlayerKey` in slab patterns.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
pub struct PlayerKeySpace {
    // space of players
    players: ThingKeySpace,
    // space of joined players
    joined_players: ThingKeySpace,
    // for each player, its joined player key, if it's joined
    player_jpk: PerThing<Option<ThingKey>>,
    // for each joined player, its player key
    joined_player_pk: PerThing<ThingKey>,
}

/// Storage of `T` per player connected to the server.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
pub struct PerPlayer<T>(PerThing<T>);

/// Storage of `T` per player connected to the server which has joined the game.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
pub struct PerJoinedPlayer<T>(PerThing<T>);

/// Key into `PerPlayer<T>`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PlayerKey(ThingKey);

/// Key into both `PerJoinedPlayer<T>` and `PerPlayer<T>`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct JoinedPlayerKey {
    // key within space of players
    pk: ThingKey,
    // key within space of joined players
    jpk: ThingKey,
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
        let pk = self.players.add();
        self.player_jpk.insert(pk, None);
        PlayerKey(pk)
    }

    /// Mark a current player as joined, allocating it a joined player key.
    ///
    /// This must not be done more than once per player. This should be matched with adding to all
    /// `PerJoinedPlayer` structures.
    pub fn join(&mut self, pk: PlayerKey) -> JoinedPlayerKey {
        debug_assert!(self.player_jpk[pk.0].is_none());
        let jpk = self.joined_players.add();
        self.player_jpk[pk.0] = Some(jpk);
        self.joined_player_pk[pk.0].insert(pk);
        JoinedPlayerKey { pk, jpk }
    }

    /// Remove a player key. If there is a corresponding joined player key, remove that too and
    /// return it.
    ///
    /// This should be matched with removing from all `PerPlayer` structures, and from all
    /// `PerJoinedPlayer` structures if some is returned.
    pub fn remove(&mut self, pk: PlayerKey) -> Option<JoinedPlayerKey> {
        self.players.remove(pk.0);
        let jpk = self.player_jpk.remove(pk.0);
        if let Some(jpk) = jpk {
            self.joined_players.remove(jpk);
            self.joined_player_pk.remove(jpk);
        }
        jpk.map(|jpk| JoinedPlayerKey { pk, jpk })
    }

    /// Get the corresponding joined player key to a player key if there is one.
    pub fn to_jpk(&self, pk: PlayerKey) -> Option<JoinedPlayerKey> {
        self.player_jpk[pk.0].map(|jpk| JoinedPlayerKey { pk, jpk })
    }

    /// Iterate through all current player keys.
    pub fn iter<'a>(&'a self) -> impl Iterator<Item=PlayerKey> + 'a {
        self.players.iter().map(PlayerKey)
    }

    /// Iterate through all current joined player keys.
    pub fn iter_joined<'a>(&'a self) -> impl Iterator<Item=JoinedPlayerKey> + 'a {
        self.joined_players.iter().map(move |jpk| JoinedPlayerKey {
            pk: self.joined_player_pk[jpk],
            jpk,
        })
    }

    /// Construct a new `PerPlayer` using `f` to populate entries for existing keys.
    pub fn new_per_player<T, F: FnMut(PlayerKey) -> T>(&self, f: F) -> PerPlayer<T> {
        PerPlayer(self.players.new_per(move |pk| f(PlayerKey(pk)))))
    }

    /// Construct a new `PerJoinedPlayer` using `f` to populate entries for existing keys.
    pub fn new_per_joined_player<T, F>(&self, f: F) -> PerJoinedPlayer<T>
    where
        F: FnMut(JoinedPlayerKey) -> T,
    {
        PerJoinedPlayer(self.joined_players.new_per(move |pk| {
            let jpk = self.joined_player_pk[pk];
            f(JoinedPlayerKey { pk, jpk })
        }))
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

impl<T> PerJoinedPlayer<T> {
    /// Construct empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert an entry for the given player key.
    ///
    /// This should follow a call to `PlayerKeySpace.join`.
    pub fn insert(&mut self, pk: JoinedPlayerKey, val: T) {
        self.0.insert(pk.jpk, val)
    }

    /// Remove the entry for the given player key.
    ///
    /// This should correspond to a call to `PlayerKeySpace.remove`.
    pub fn remove(&mut self, pk: JoinedPlayerKey) -> T {
        self.0.remove(pk.jpk)
    }

    /// Get by shared reference.
    pub fn get(&self, pk: JoinedPlayerKey) -> &T {
        self.0.get(pk.jpk)
    }

    /// Get by mutable reference.
    pub fn get_mut(&mut self, pk: JoinedPlayerKey) -> &mut T {
        self.0.get_mut(pk.jpk)
    }
}

impl JoinedPlayerKey {
    /// Extract the general player key.
    pub fn to_pk(self) -> PlayerKey {
        PlayerKey(self.pk)
    }
}

impl Into<PlayerKey> for JoinedPlayerKey {
    fn into(self) -> PlayerKey {
        self.to_pk()
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

impl<T> Index<JoinedPlayerKey> for PerJoinedPlayer<T> {
    type Output = T;

    fn index(&self, pk: JoinedPlayerKey) -> &T {
        self.get(pk)
    }
}

impl<T> IndexMut<JoinedPlayerKey> for PerJoinedPlayer<T> {
    fn index_mut(&mut self, pk: JoinedPlayerKey) -> &mut T {
        self.get_mut(pk)
    }
}

impl<T> Index<PlayerKey> for PerJoinedPlayer<T> {
    type Output = T;

    fn index(&self, pk: PlayerKey) -> &T {
        self.get(pk.to_pk())
    }
}

impl<T> IndexMut<PlayerKey> for PerJoinedPlayer<T> {
    fn index_mut(&mut self, pk: PlayerKey) -> &mut T {
        self.get_mut(pk.to_pk())
    }
}
