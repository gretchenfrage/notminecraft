
use crate::chunk::CiGet;
use slab::Slab;
use vek::*;


#[derive(Debug, Clone)]
pub struct PerChunk<T>(pub Slab<(Vec3<i64>, T)>);

impl<T> PerChunk<T> {
    pub fn add(&mut self, cc: Vec3<i64>, ci: usize, val: T) {
        let ci2 = self.0.insert((cc, val));
        debug_assert_eq!(ci, ci2);
    }

    pub fn remove(&mut self, cc: Vec3<i64>, ci: usize) -> T {
        let (cc2, val) = self.0.remove(ci);
        debug_assert_eq!(cc, cc2);
        val
    }

    pub fn get(&self, cc: Vec3<i64>, ci: usize) -> &T {
        let &(cc2, ref val) = &self.0[ci];
        debug_assert_eq!(cc, cc2);
        val
    }

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
