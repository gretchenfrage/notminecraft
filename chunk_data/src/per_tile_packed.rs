
use crate::coord::NUM_LTIS;
use std::alloc::{
    alloc,
    Layout,
};


/// Per-tile (within a chunk) bit-packed storage of sub-byte u-ints.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PerTilePacked<
    const BYTES: usize,
    const MASK: u8,
>(pub Box<[u8; BYTES]>);

impl<
    const BYTES: usize,
    const MASK: u8,
> PerTilePacked<BYTES, MASK> {
    /// Construct with all zeroes.
    pub fn new() -> Self {
        let array = unsafe {
            let layout = Layout::array::<u8>(BYTES).unwrap();
            let ptr = alloc(layout) as *mut u8;

            for i in 0..BYTES {
                *ptr.add(i) = 0;
            }

            Box::from_raw(ptr as *mut [u8; BYTES])
        };
        PerTilePacked(array)
    }

    /// Get the value at some index.
    pub fn get(&self, lti: u16) -> u8 {
        let shift = (lti as usize % (NUM_LTIS / BYTES)) * (8 * BYTES / NUM_LTIS);
        let field = self.0[lti as usize / (NUM_LTIS / BYTES)];
        (field & (MASK << shift)) >> shift
    }

    /// Set the value at some index. Panics if value out of range.
    pub fn set(&mut self, lti: u16, val: u8) {
        assert!((val & !MASK) == 0, "val out of range");

        let shift = (lti as usize % (NUM_LTIS / BYTES)) * (8 * BYTES / NUM_LTIS);
        let field = &mut self.0[lti as usize / (NUM_LTIS / BYTES)];
        *field = (*field & !(MASK << shift)) | (val << shift);
    }
}

impl<
    const BYTES: usize,
    const MASK: u8,
> Default for PerTilePacked<BYTES, MASK> {
    fn default() -> Self {
        PerTilePacked::new()
    }
}


/// Per-tile (within a chunk) bit-packed storage of `u4`.
///
/// Consumes 2^15 bytes = 32 KiB.
pub type PerTileU4 = PerTilePacked<{NUM_LTIS / 2}, 0b1111>;

/// Per-tile (within a chunk) bit-packed storage of `u2`.
///
/// Consumes 2^14 bytes = 16 KiB.
pub type PerTileU2 = PerTilePacked<{NUM_LTIS / 4}, 0b11>;

/// Per-tile (within a chunk) bit-packed storage of `u1`.
///
/// Consumes 2^13 bytes = 8 KiB.
pub type PerTileU1 = PerTilePacked<{NUM_LTIS / 8}, 0b1>;


/// Wrapper around `PerTileU1`.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PerTileBool(pub PerTileU1);

impl PerTileBool {
    /// Construct with all false.
    pub fn new() -> Self {
        PerTileBool(PerTileU1::new())
    }

    /// Get the value at some index.
    pub fn get(&self, lti: u16) -> bool {
        self.0.get(lti) != 0
    }

    /// Set the value at some index. Panics if value out of range.
    pub fn set(&mut self, lti: u16, val: bool) {
        self.0.set(lti, if val { 1 } else { 0 })
    }
}
