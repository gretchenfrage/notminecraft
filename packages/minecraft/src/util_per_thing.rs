//! Macro for declaring a system of types to manage the allocation of keys for some thing in a slab
//! pattern and store some value for each such key.

use std::ops::{Index, IndexMut};
use slab::Slab;


/// Generalized backing structure. Manager the allocation of `ThingKey` in a slab pattern.
[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
pub struct ThingKeySpace {
    slab: Slab<u64>,
    ctr: u64,
}

/// Generalized backing structure. Storage of `T` per thing.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
pub struct PerThing<T>(Slab<(T, u64)>);

/// Generalized backing structure. Key into `PerThing`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ThingKey {
    idx: usize,
    ctr: u64,
}

impl<T> ThingKeySpace<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self) -> ThingKey {
        let ctr = self.ctr;
        let idx = self.slab.insert(ctr);
        self.ctr = self.ctr.wrapping_add(1);
        ThingKey { idx, ctr }
    }

    pub fn remove(&mut self, key: ThingKey) {
        let ctr2 = self.slab.remove(key.idx);
        debug_assert_eq!(ctr2, key.ctr, "ThingKeySpace.remove ctr mismatch");
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item=ThingKey> + 'a {
        self.slab.iter().map(|(idx, &ctr)| ThingKey { idx, ctr })
    }

    pub fn new_per<T, F: FnMut(ThingKey) -> T>(&self, f: F) -> PerThing<T> {
        PerThing(self.slab.new_mapped(|idx, &ctr| (f(ThingKey { idx, ctr }), ctr)))
    }
}

impl<T> PerThing<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, key: ThingKey, val: T) {
        let idx2 = self.0.insert((val, key.ctr));
        debug_assert_eq!(idx2, key.idx, "PerThing.insert idx mismatch");
    }

    pub fn remove(&mut self, key: ThingKey) -> T {
        let (val, ctr2) = self.0.remove(key.idx);
        debug_assert_eq!(ctr2, key.ctr, "PerThing.remove ctr mismatch");
    }

    pub fn get(&self, key: ThingKey) -> &T {
        let &(ref val, ctr2) = &self.0[key.idx];
        debug_assert_eq!(ctr2, key.ctr, "PerThing.get ctr mismatch");
        val
    }

    pub fn get_mut(&mut self, key: ThingKey) -> &mut T {
        let &mut (ref mut val, ctr2) = &mut self.0[key.idx];
        debug_assert_eq!(ctr2, key.ctr, "PerThing.get ctr mismatch");
        val
    }
}

/// Declare a system of types to manage the allocation of keys for some thing in a slab pattern and
/// store some value for each such key.
macro_rules! per_thing_types {
    ($key_space:ident, $per_thing:ident, $key:ident)=>{
        #[doc=concat!("Manages the allocation of `", stringify!($key), "` in a slab pattern.")]
        #[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
        pub struct $key_space()($crate::util_per_thing::ThingKeySpace);

        #[doc=concat!("Storage of `T` per active `", stringify!($key), "`.")]
        #[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
        pub struct $per_thing<T>($crate::util_per_thing::PerThing<T>);

        #[doc=concat!("Key into `", $per_thing, "<T>`.")]
        #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub struct $key($crate::util_per_thing::ThingKey);

        impl<T> $key_space<T> {
            /// Construct empty.
            pub fn new() -> Self {
                Self($crate::util_per_thing::PerThing<T>::new())
            }

            #[doc=concat!("Allocate a new key.\n\nThis should be followed by adding to all `", stringify!($per_thing), "` structures.")]
            pub fn add(&mut self) -> PlayerKey {
                $crate::util_per_thing::PerThing<T>::add(&mut self.0)
            }

            #[doc=concat!("Remove a key.\n\nThis should be matched with removing from all `", stringify!($per_thing), "` structures.")]
            pub fn remove(&mut self, key: PlayerKey) {
                $crate::util_per_thing::PerThing<T>::remove(&mut self.0, key)
            }

            /// Iterate through all current keys.
            pub fn iter<'a>(&'a self) -> impl Iterator<Item=PlayerKey> + 'a {
                $crate::util_per_thing::PerThing<T>::iter(&self.0)
            }

            /// Construct a new `PerPlayer` structure using `f` to populate entries for existing player
            /// keys.
            #[doc=concat!("Construct a new `", stringify!($per_thing), "` structures using `f` to populating entries for existing keys.")]
            pub fn new_per<T, F: FnMut($) -> T>(&self, f: F) -> $per_thing<T> {
                $per_thing($crate::util_per_thing::PerThing<T>::new_per(&self.0, |key| f($key(key))))
            }
        }

        impl<T> $per_thing<T> {
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
    };
}
