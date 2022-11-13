
use crate::CiGet;
use slab::Slab;
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
}

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
