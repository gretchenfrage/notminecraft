
use std::{
    marker::PhantomData,
    fmt::{self, Formatter, Debug},
};


const PACKED_IDX_HI_BIT: usize = 1_usize.rotate_right(1);

/// Bit-packed (1-bit bool, 63-bit u-int) tuple (assuming 64-bit CPU).
#[derive(Debug, Copy, Clone, Default)]
pub struct PackedIdx(usize);

impl PackedIdx {
    pub fn new(hi_bit: bool, idx: usize) -> Self {
        assert!((idx & PACKED_IDX_HI_BIT) == 0, "idx too large");
        PackedIdx((hi_bit as usize).rotate_right(1) | idx)
    }

    pub fn hi_bit(self) -> bool {
        (self.0 & PACKED_IDX_HI_BIT) != 0
    }

    pub fn idx(self) -> usize {
        self.0 & !PACKED_IDX_HI_BIT
    }
}


/// Some type which implements `From`+`Into``<PackedIdx>`, represented in
/// memory as just a `PackedIdx`.
pub struct PackedIdxRepr<T> {
    bits: PackedIdx,
    _p: PhantomData<T>,
}

impl<T: Into<PackedIdx>> From<T> for PackedIdxRepr<T> {
    fn from(val: T) -> Self {
        PackedIdxRepr {
            bits: val.into(),
            _p: PhantomData,
        }        
    }
}

impl<T: From<PackedIdx>> PackedIdxRepr<T> {
    pub fn unpack(self) -> T {
        T::from(self.bits)
    }
}

impl<T: Debug + From<PackedIdx>> Debug for PackedIdxRepr<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(&T::from(self.bits), f)
    }
}

impl<T> Copy for PackedIdxRepr<T> {}

impl<T> Clone for PackedIdxRepr<T> {
    fn clone(&self) -> Self {
        PackedIdxRepr {
            bits: self.bits,
            _p: PhantomData,
        }
    }
}
