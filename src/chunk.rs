
use std::{
    convert::TryInto,
    fmt,
};


/// CHunk Coord.
pub fn chc<X, Y, Z>(x: X, z: Z) -> ChunkCoord
where
    X: TryInto<i32>,
    Z: TryInto<i32>,
{
    ChunkCoord::new(
        x.try_into().ok().unwrap(),
        z.try_into().ok().unwrap(),
    )
}


/// Coordinate of a chunk in the world.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ChunkCoord {
    // if the magnitude is too high, this may be invalid and caused overflow
    pub x: i32,
    pub z: i32,
}

impl ChunkCoord {
    /// Construct from components.
    pub fn new(x: i32, z: i32) -> Self {
        ChunkCoord { x, z }
    }
}


macro_rules! impl_fmt_chunk_coord {
    ($t:ident, $fstr:literal)=>{
        impl fmt::$t for ChunkCoord {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(
                    f, $fstr, 
                    self.x, self.z,
                )
            }
        }
    };
}
impl_fmt_chunk_coord!(Debug, "<{},{}>");
impl_fmt_chunk_coord!(Display, "<{},{}>");
impl_fmt_chunk_coord!(LowerHex, "<{:x},{:x}>");
impl_fmt_chunk_coord!(UpperHex, "<{:X},{:X}>");

