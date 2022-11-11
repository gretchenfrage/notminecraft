//! Global tile coords.

use crate::{
    local::Ltc,
    chunk::Chc,
};
use std::{
    convert::TryInto,
    fmt,
};


/// Global tile coord. 
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


/// Global tile coord. 
///
/// Coordinate of a tile anywhere in the world.
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

    /// Get the chunk-local part of this tile coordinate. 
    pub fn to_local(&self) -> Ltc {
        Ltc(
            ((self.y as u16 & 0xf0) >> 4)
            | (((self.x & 0x0000000f) as u16) << 4)
            | ((self.y as u16 & 0x0f) << 8)
            | (((self.z & 0x0000000f) as u16) << 12)
        )
    }
    
    /// Get the coord of the chunk this tile coordinate is in. 
    pub fn to_chunk(&self) -> Chc {
        Chc::new((self.x & !0xf) / 16, (self.z & !0xf) / 16)
    }

    /// Construct from a coordinate of a chunk and a coordinate of a tile
    /// relative to that chunk. 
    pub fn from_parts(chunk: Chc, local: Ltc) -> Self {
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

    /// Split this tile coordinate into the coordinate of the chunk it's in
    /// and the coordinate of the tile relative to that chunk.
    pub fn to_parts(&self) -> (Chc, Ltc) {
        (self.to_chunk(), self.to_local())
    }
}



macro_rules! impl_fmt_global_tile_coord {
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
impl_fmt_global_tile_coord!(Debug, "<{},{},{}>");
impl_fmt_global_tile_coord!(Display, "<{},{},{}>");
impl_fmt_global_tile_coord!(LowerHex, "<{:x},{:x},{:x}>");
impl_fmt_global_tile_coord!(UpperHex, "<{:X},{:X},{:X}>");
