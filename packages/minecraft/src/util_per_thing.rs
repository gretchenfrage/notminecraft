//! Generalized backing structures for types which allocate keys in a slab pattern and store a
//! value for each key.

use std::ops::{Index, IndexMut};
use slab::Slab;


/// Generalized backing structure. Manages the allocation of `ThingKey` in a slab pattern.
#[derive(Debug, Clone, Default)]
pub struct ThingKeySpace {
    slab: Slab<u64>,
    ctr: u64,
}

/// Generalized backing structure. Storage of `T` per thing.
#[derive(Debug, Clone)]
pub struct PerThing<T>(Slab<(T, u64)>);

/// Generalized backing structure. Key into `PerThing`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ThingKey {
    idx: usize,
    ctr: u64,
}

impl ThingKeySpace {
    /// Construct empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocate a key.
    pub fn add(&mut self) -> ThingKey {
        let ctr = self.ctr;
        let idx = self.slab.insert(ctr);
        self.ctr = self.ctr.wrapping_add(1);
        ThingKey { idx, ctr }
    }

    /// Remove a key.
    pub fn remove(&mut self, key: ThingKey) {
        let ctr2 = self.slab.remove(key.idx);
        debug_assert_eq!(ctr2, key.ctr, "ThingKeySpace.remove ctr mismatch");
    }

    /// Iterate through keys.
    pub fn iter<'a>(&'a self) -> impl Iterator<Item=ThingKey> + 'a {
        self.slab.iter().map(|(idx, &ctr)| ThingKey { idx, ctr })
    }

    /// Construct a new `PerThing` using `f` to populate entries for existing keys.
    pub fn new_per<T, F: FnMut(ThingKey) -> T>(&self, mut f: F) -> PerThing<T> {
        PerThing(self.slab.new_mapped(|idx, &ctr| (f(ThingKey { idx, ctr }), ctr)))
    }
}

impl<T> PerThing<T> {
    /// Construct empty.
    pub fn new() -> Self {
        Self(Default::default())
    }

    /// Insert value for given key.
    ///
    /// Should follow a call to `ThingKeySpace.add`.
    pub fn insert(&mut self, key: ThingKey, val: T) {
        let idx2 = self.0.insert((val, key.ctr));
        debug_assert_eq!(idx2, key.idx, "PerThing.insert idx mismatch");
    }

    /// Remove value for given key.
    ///
    /// Should correspond to a call to `ThingKeySpace.remove`..
    pub fn remove(&mut self, key: ThingKey) -> T {
        let (val, ctr2) = self.0.remove(key.idx);
        debug_assert_eq!(ctr2, key.ctr, "PerThing.remove ctr mismatch");
        val
    }

    /// Get by shared reference.
    ///
    /// May panic on failure if debug assertions enabled.
    pub fn get(&self, key: ThingKey) -> &T {
        let &(ref val, ctr2) = &self.0[key.idx];
        debug_assert_eq!(ctr2, key.ctr, "PerThing.get ctr mismatch");
        val
    }

    /// Get by mutable reference.
    ///
    /// May panic on failure if debug assertions enabled.
    pub fn get_mut(&mut self, key: ThingKey) -> &mut T {
        let &mut (ref mut val, ctr2) = &mut self.0[key.idx];
        debug_assert_eq!(ctr2, key.ctr, "PerThing.get ctr mismatch");
        val
    }
}

impl<T> Index<ThingKey> for PerThing<T> {
    type Output = T;

    fn index(&self, key: ThingKey) -> &Self::Output {
        self.get(key)
    }
}

impl<T> IndexMut<ThingKey> for PerThing<T> {
    fn index_mut(&mut self, key: ThingKey) -> &mut Self::Output {
        self.get_mut(key)
    }
}

impl<T> Default for PerThing<T> {
    fn default() -> Self {
        Self::new()
    }
}
