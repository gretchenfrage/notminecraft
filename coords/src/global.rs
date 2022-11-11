//! Global block coords.

use crate::{
    local::Lbc,
    chunk::Chc,
};
use std::{
    convert::TryInto,
    fmt,
};


/// Global block coord. 
pub fn gbc<X, Y, Z>(x: X, y: Y, z: Z) -> Gbc
where
    X: TryInto<i32>,
    Y: TryInto<u8>,
    Z: TryInto<i32>,
{
    Gbc::new(
        x.try_into().ok().unwrap(),
        y.try_into().ok().unwrap(),
        z.try_into().ok().unwrap(),
    )
}


/// Global block coord. 
///
/// Coordinate of a block anywhere in the world.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Gbc {
    pub x: i32,
    pub z: i32,
    pub y: u8,
}

impl Gbc {
    /// Construct from components. 
    pub fn new(x: i32, y: u8, z: i32) -> Self {
        Gbc { x, y, z }
    }

    /// Get the chunk-local part of this block coordinate. 
    pub fn to_local(&self) -> Lbc {
        Lbc(
            ((self.y as u16 & 0xf0) >> 4)
            | (((self.x & 0x0000000f) as u16) << 4)
            | ((self.y as u16 & 0x0f) << 8)
            | (((self.z & 0x0000000f) as u16) << 12)
        )
    }
    
    /// Get the coord of the chunk this block coordinate is in. 
    pub fn to_chunk(&self) -> Chc {
        Chc::new((self.x & !0xf) / 16, (self.z & !0xf) / 16)
    }

    /// Construct from a coordinate of a chunk and a coordinate of a block
    /// relative to that chunk. 
    pub fn from_parts(chunk: Chc, local: Lbc) -> Self {
        let x_chunk_part = chunk.x
            .checked_mul(16)
            .unwrap_or_else(|| panic!("Chc x={} out of range", chunk.x));
        let z_chunk_part = chunk.z
            .checked_mul(16)
            .unwrap_or_else(|| panic!("Chc z={} out of range", chunk.z));
        Self::new(
            x_chunk_part | local.x() as i32,
            local.y() as u8,
            z_chunk_part | local.z() as i32,
        )
    }

    /// Split this block coordinate into the coordinate of the chunk it's in
    /// and the coordinate of the block relative to that chunk.
    pub fn to_parts(&self) -> (Chc, Lbc) {
        (self.to_chunk(), self.to_local())
    }
}



macro_rules! impl_fmt_global_block_coord {
    ($t:ident, $fstr:literal)=>{
        impl fmt::$t for Gbc {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(
                    f, $fstr, 
                    self.x, self.y, self.z,
                )
            }
        }
    };
}
impl_fmt_global_block_coord!(Debug, "<{},{},{}>");
impl_fmt_global_block_coord!(Display, "<{},{},{}>");
impl_fmt_global_block_coord!(LowerHex, "<{:x},{:x},{:x}>");
impl_fmt_global_block_coord!(UpperHex, "<{:X},{:X},{:X}>");
