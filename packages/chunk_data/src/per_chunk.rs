
use crate::CiGet;
use std::ops::{Index, IndexMut};
use slab::{Slab, UnsafeSlabRef};
use vek::*;


/// Per-chunk storage. Often contains some per-tile storage.
/// 
/// Should be updated in synchrony with with `LoadedChunks`. Functionally,
/// could work solely on cid, without containing cc. However, storing cc
/// and performing debug equality assertions on them helps detect cases of
/// accidentally forgetting or failing to keep updated in synchrony with
/// `LoadedChunks`.
#[derive(Debug, Clone)]
pub struct PerChunk<T>(pub Slab<(Vec3<i64>, T)>);

impl<T> PerChunk<T> {
    pub fn new() -> Self {
        PerChunk(Slab::new())
    }

    /// Add a new value with the given cc and ci.
    /// 
    /// Should follow calls to `LoadedChunks::add`.
    pub fn add(&mut self, cc: Vec3<i64>, ci: usize, val: T) {
        let ci2 = self.0.insert((cc, val));
        debug_assert_eq!(ci, ci2);
    }

    /// Remove a present value with the given cc and ci.
    ///
    /// Should follow calls to `LoadedChunks::remove`.
    pub fn remove(&mut self, cc: Vec3<i64>, ci: usize) -> T {
        let (cc2, val) = self.0.remove(ci);
        debug_assert_eq!(cc, cc2);
        val
    }

    /// Get by ci, debug-assert cc is correct.
    pub fn get(&self, cc: Vec3<i64>, ci: usize) -> &T {
        let &(cc2, ref val) = &self.0[ci];
        debug_assert_eq!(cc, cc2);
        val
    }

    /// Mutably get by ci, debug-assert cc is correct.
    pub fn get_mut(&mut self, cc: Vec3<i64>, ci: usize) -> &mut T {
        let &mut (cc2, ref mut val) = &mut self.0[ci];
        debug_assert_eq!(cc, cc2);
        val
    }

    /// Get by ci, without checking cc.
    pub fn get_checkless(&self, ci: usize) -> &T {
        &self.0[ci].1
    }

    /// Mutably get by ci, without checking cc.
    pub fn get_mut_checkless(&mut self, ci: usize) -> &mut T {
        &mut self.0[ci].1
    }

    /// Mutably get by ci, debug-assert cc is correct, and return a projection of self that lets
    /// one mutably get a different entry in parallel.
    pub fn get_mut_partially(
        &mut self,
        cc: Vec3<i64>,
        ci: usize,
    ) -> (&mut T, PerChunkPartiallyBorrowed<T>)
    {
        unsafe {
            let entries_usr = self.0.unsafe_slab_ref();
            let &mut (cc2, ref mut val) = entries_usr.get_mut_unsafe(ci).unwrap();
            debug_assert_eq!(cc, cc2);
            (val, PerChunkPartiallyBorrowed {
                entries_usr,
                borrowed_ci: ci,
            })
        }
    }
}

impl<T> Default for PerChunk<T> {
    fn default() -> Self {
        Self::new()
    }
}

// TODO: rename get to ci_get to prevent silly interference thing
impl<'a, T> CiGet for &'a PerChunk<T> {
    type Output = &'a T;

    fn get(self, cc: Vec3<i64>, ci: usize) -> Self::Output {
        PerChunk::get(self, cc, ci)
    }
}

impl<'a, T> CiGet for &'a mut PerChunk<T> {
    type Output = &'a mut T;

    fn get(self, cc: Vec3<i64>, ci: usize) -> Self::Output {
        PerChunk::get_mut(self, cc, ci)
    }
}

impl<T> Index<usize> for PerChunk<T> {
    type Output = T;

    fn index(&self, i: usize) -> &T {
        &self.0[i].1
    }
}

impl<T> IndexMut<usize> for PerChunk<T> {
    fn index_mut(&mut self, i: usize) -> &mut T {
        &mut self.0[i].1
    }
}

/// Borrow of a `PerChunk<T>` in which one entry is already mutably borrowed.
///
/// Allows any one _other_ entry at a time to be borrowed simultaneously.
pub struct PerChunkPartiallyBorrowed<'a, T> {
    entries_usr: UnsafeSlabRef<'a, (Vec3<i64>, T)>,
    borrowed_ci: usize,
}

impl<'a, T> PerChunkPartiallyBorrowed<'a, T> {
    /// Mutably get by ci, debug-assert cc is correct. Panics if the provided ci is the ci that is
    /// already borrowed.
    pub fn get_mut(&mut self, cc: Vec3<i64>, ci: usize) -> &mut T {
        assert!(ci != self.borrowed_ci, "PerChunkPartiallyBorrowed collision");
        unsafe {
            let &mut (cc2, ref mut val) = self.entries_usr.get_mut_unsafe(ci).unwrap();
            debug_assert_eq!(cc, cc2);
            val
        }
    }
}


