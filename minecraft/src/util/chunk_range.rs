
use chunk_data::*;
use std::{
    cmp::{min, max},
    fmt::Debug,
    convert::identity,
};
use vek::*;


/// A 3D range of ccs from start (inclusive) to end (inclusive).
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ChunkRange {
    pub start: Vec3<i64>,
    pub end: Vec3<i64>,
}

impl ChunkRange {
    /// Iterate through all ccs in the range.
    pub fn iter(self) -> impl Iterator<Item=Vec3<i64>> {
        (self.start.z..self.end.z)
            .flat_map(move |z| (self.start.y..self.end.y)
                .flat_map(move |y| (self.start.x..self.end.x)
                    .map(move |x| Vec3 { x, y, z })))
    }

    /// Efficiently iterate through all ccs in this range that aren't in the
    /// `rhs` range.
    pub fn iter_diff(self, rhs: ChunkRange) -> impl Iterator<Item=Vec3<i64>> + Clone {
        permute3(AXES.map(|axis| {
            let a_start = PerAxis::from(self.start)[axis];
            let a_end = max(PerAxis::from(self.end)[axis], a_start);
            let b_start = PerAxis::from(rhs.start)[axis];
            let b_end = max(PerAxis::from(rhs.end)[axis], b_start);
            let output = [
                (true, a_start..min(a_end, b_start)),
                (false, max(b_start, a_start)..min(b_end, a_end)),
                (true, max(a_start, b_end)..a_end),
            ];
            output
        })).filter_map(|permutation| {
            let Vec3 {
                x: (x_bool, x_range),
                y: (y_bool, y_range),
                z: (z_bool, z_range),
            } = permutation;
            if x_bool || y_bool || z_bool {
                Some(permute3([x_range, y_range, z_range]))
            } else {
                None
            }
        }).flat_map(identity)
    }
}

/// Convert array of 3 iterators to iterator of all their permutation vectors.
fn permute3<A, I>(iters: A) -> impl Iterator<Item=Vec3<I::Item>> + Clone
where
    A: Into<[I; 3]>,
    I: IntoIterator + Clone,
    I::IntoIter: Clone,
    I::Item: Clone,
{
    let [x_iter, y_iter, z_iter] = iters.into();
    z_iter.into_iter().flat_map(move |z| {
        let x_iter = x_iter.clone();
        y_iter.clone().into_iter().flat_map(move |y| {
            let z = z.clone();
            x_iter.clone().into_iter().map(move |x| {
                Vec3 {
                    x,
                    y: y.clone(),
                    z: z.clone(),
                }
            })
        })
    })
}
