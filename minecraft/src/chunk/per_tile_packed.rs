
use crate::chunk::coord::NUM_LTIS;
use std::alloc::{
    alloc,
    Layout,
};


/// Bit-packed array of sub-byte uints for every tile in a chunk, indexed by
/// lti.
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


/// Bit-packed array of u4 for every tile in a chunk, indexed by lti.
pub type PerTileU4 = PerTilePacked<{NUM_LTIS / 2}, 0b1111>;

/// Bit-packed array of u2 for every tile in a chunk, indexed by lti.
pub type PerTileU2 = PerTilePacked<{NUM_LTIS / 4}, 0b11>;

/// Bit-packed array of u1 for every tile in a chunk, indexed by lti.
pub type PerTileU1 = PerTilePacked<{NUM_LTIS / 8}, 0b1>;
