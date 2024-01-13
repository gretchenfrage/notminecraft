
use crate::{
    message::*,
    util::chunk_range::ChunkRange,
};
use chunk_data::*;
use vek::*;


pub const LOAD_Y_START: i64 = 0;
pub const LOAD_Y_END: i64 = 2;
pub const INITIAL_LOAD_DISTANCE: i64 = 8;


pub fn char_load_range(char_state: CharState) -> ChunkRange {
    let char_cc = (char_state.pos / CHUNK_EXTENT.map(|n| n as f32)).map(|n| n.floor() as i64);
    let load_distance = char_state.load_dist as i64;
    ChunkRange {
        start: Vec3 {
            x: char_cc.x - load_distance,
            y: LOAD_Y_START,
            z: char_cc.z - load_distance,
        },
        end: Vec3 {
            x: char_cc.x + load_distance + 1,
            y: LOAD_Y_END,
            z: char_cc.z + load_distance + 1,
        },
    }
}

pub fn dist_sorted_ccs(ccs: impl IntoIterator<Item=Vec3<i64>>, pos: Vec3<f32>) -> Vec<Vec3<i64>> {
    let mut ccs = ccs.into_iter().collect::<Vec<_>>();
    fn square_dist(a: Vec3<f32>, b: Vec3<f32>) -> f32 {
        (a - b).map(|n| n * n).sum()
    }
    fn cc_square_dist(cc: Vec3<i64>, pos: Vec3<f32>) -> f32 {
        square_dist(
            (cc.map(|n| n as f32) + 0.5) * CHUNK_EXTENT.map(|n| n as f32),
            pos,
        )
    }
    ccs.sort_by(|&cc1, &cc2| cc_square_dist(cc1, pos).total_cmp(&cc_square_dist(cc2, pos)));
    ccs
}
