//! Local block coords.

use std::{
    fmt,
    convert::TryInto,
};


/// Local block coord.
///
/// Panics if coords are out of bounds. 
pub fn lbc<X, Y, Z>(x: X, y: Y, z: Z) -> Lbc
where
    X: TryInto<u16>,
    Y: TryInto<u16>,
    Z: TryInto<u16>,
{
    Lbc::new(
        x.try_into().ok().unwrap(),
        y.try_into().ok().unwrap(),
        z.try_into().ok().unwrap(),
    )
}

/// Local block coord.
///
/// Coordinate of a block within a chunk.
///
/// Represented as a `u16` bitfield, but has the semantics of a structure of
/// [x, y, z] coordinates. This has the very useful property that there is a
/// 1:1 relation between valid chunk-local coordinates and representable
/// `u16`. As such:
///
/// - An array that stores an element for each chunk-local coord can have a
///   length of `0x10000`.
/// - Iterating through the indices of this array with a u16 can be done with
///   the range `0..=0xffff`.
/// - Converting between an array index and a `Lbc` is a no-op,
///   done by wrapping / unwrapping the `u16` index.
///
/// The layout of the bit field is as such:
///
/// ```txt
/// |    Z | Y LE |    X | Y BE |
/// ```
/// 
/// Where each cell is 4 bits of the `u16`. The X and Z axis are 4-bit uints,
/// and therefore are stored contiguously. The Y field is an 8 bit uint, and
/// is split into its bit-endian (`Y BE`) and little-endian (`Y LE`) halves,
/// which are stored in different parts.
///
/// The purpose of this admittedly strange layout is that, when using the
/// inner `u16` as an array index, blocks which are nearby in 3D space are
/// nearby in the array, thus facilitating spatial locality. 
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Lbc(pub u16);

impl Lbc {
    /// Construct from <X, Y, Z> coords.
    ///
    /// Panics if coords are out of range.
    pub fn new(x: u16, y: u16, z: u16) -> Self {
        assert!(x <= 0xf, "Lbc x={} is too high", x);
        assert!(y <= 0xff, "Lbc y={} is too high", y);
        assert!(z <= 0xf, "Lbc z={} is too high", z);
        
        Lbc(
            ((y & 0xf0) >> 4)
            | (x << 4)
            | ((y & 0x0f) << 8)
            | (z << 12)
        )
    }
    
    /// Get the X coord.
    pub fn x(&self) -> u16 {
        (self.0 & 0x00f0) >> 4
    }
    
    /// Get the Y coord.
    pub fn y(&self) -> u16 {
        ((self.0 & 0x000f) << 4) | ((self.0 & 0x0f00) >> 8)
    }
    
    /// Get the Z coord.
    pub fn z(&self) -> u16 {
        (self.0 & 0xf000) >> 12
    }

    /// Set the X coord.
    ///
    /// Panics if coord is out of range.
    pub fn set_x(&mut self, x: u16) {
        assert!(x <= 0xf, "Lbc x={} is too high", x);
        self.0 &= 0xff0f;
        self.0 |= x << 4;
    }

    /// Set the Y coord.
    /// 
    /// Panics if coord is out of range.
    pub fn set_y(&mut self, y: u16) {
        assert!(y <= 0xff, "Lbc y={} is too high", y);
        self.0 &= 0xf0f0;
        self.0 |= (y & 0xf0) >> 4;
        self.0 |= (y & 0x0f) << 8;
    }

    /// Set the Z coord.
    ///
    /// Panics if coord is out of range.
    pub fn set_z(&mut self, z: u16) {
        assert!(z <= 0xf, "Lbc z={} is too high", z);
        self.0 &= 0x0fff;
        self.0 |= z << 12;
    }

    /// Get the <X, Y, Z> coords as an array.
    pub fn xyz(&self) -> [u16; 3] {
        [self.x(), self.y(), self.z()]
    }
}

macro_rules! impl_fmt_local_block_coord {
    ($t:ident, $fstr:literal)=>{
        impl fmt::$t for Lbc {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(
                    f, $fstr, 
                    self.x(), self.y(), self.z(),
                )
            }
        }
    };
}
impl_fmt_local_block_coord!(Debug, "<{:0>2},{:0>3},{:0>2}>");
impl_fmt_local_block_coord!(Display, "<{:0>2},{:0>3},{:0>2}>");
impl_fmt_local_block_coord!(LowerHex, "<{:0>1x},{:0>2x},{:0>1x}>");
impl_fmt_local_block_coord!(UpperHex, "<{:0>1X},{:0>2X},{:0>1X}>");
