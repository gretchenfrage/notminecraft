
use std::{
    marker::PhantomData,
    fmt::{self, Formatter, Debug},
};


type BaseType = u32;


/// Index type.
///
/// Abstracts over the system's pointer size versus the index size we're using.
/// We may use a smaller size to save memory. 
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
pub struct Idx(BaseType);

impl Idx {
    pub fn new(idx: usize) -> Self {
        Idx(BaseType::try_from(idx).expect("idx too large"))
    }

    pub fn usize(self) -> usize {
        self.0 as usize
    }
}


const PACKED_IDX_HI_BIT: BaseType = BaseType::rotate_right(1, 1);

/// Bit-packed (1-bit bool, (size - 1)-bit idx) tuple.
#[derive(Debug, Copy, Clone, Default)]
pub struct PackedIdx(BaseType);

impl PackedIdx {
    pub fn new(hi_bit: bool, idx: Idx) -> Self {
        assert!((idx.0 & PACKED_IDX_HI_BIT) == 0, "idx too large");
        PackedIdx((hi_bit as BaseType).rotate_right(1) | idx.0)
    }

    pub fn hi_bit(self) -> bool {
        (self.0 & PACKED_IDX_HI_BIT) != 0
    }

    pub fn idx(self) -> Idx {
        Idx(self.0 & !PACKED_IDX_HI_BIT)
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
