
use crate::{
    per_tile_packed::PerTileU1,
    per_tile::PerTile,
    coord::MAX_LTI,
};
use std::{
    mem::MaybeUninit,
    fmt::{self, Formatter, Debug},
};


/// Per-tile (within a chunk) storage of `Option<T>` optimized for small `T`.
///
/// Statically allocates an array for `T` and a bit-array for whether they're
/// `Some`.
///
/// Contrast with `PerTileSparse`, which also stores `Option<T>`.
/// `PerTileSparse` dynamically allocates `size_of::<T>()` bytes only for
/// `Some` elements, but incurs an overhead of 2 bytes for every tile and 2
/// more bytes for every `Some` element. Conversely, `PerTileOption` statically
/// allocates `size_of::<T>()` for every tile, but incurs an additional
/// overhead of only 1 _bit_ per tile.
///
/// As such, for `size_of::<T>() == 1`, `PerTileOption` always consumes less
/// memory than `PerTileSparse`, and for `size_of::<T>() == 2`, `PerTileOption`
/// would still consume less memory unless the values were extremely sparse.
///
/// TODO: use and update the table in `PerTileSparse`.
pub struct PerTileOption<T> {
    is_some: PerTileU1,
    content: PerTile<MaybeUninit<T>>,
}

impl<T> PerTileOption<T> {
    pub fn new() -> Self {
        PerTileOption {
            is_some: PerTileU1::new(),
            content: PerTile::new_uninit(),
        }
    }

    pub fn get(&self, lti: u16) -> Option<&T> {
        if self.is_some.get(lti) != 0 {
            let ptr = self.content[lti].as_ptr();
            Some(unsafe { &*ptr })
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, lti: u16) -> Option<&T> {
        if self.is_some.get(lti) != 0 {
            let ptr = self.content[lti].as_mut_ptr();
            Some(unsafe { &mut *ptr })
        } else {
            None
        }
    }

    pub fn set(&mut self, lti: u16, val: Option<T>) {
        if let Some(val) = val {
            self.set_some(lti, val);
        } else {
            self.set_none(lti);
        }
    }

    pub fn set_some(&mut self, lti: u16, val: T) {
        if self.is_some.get(lti) != 0 {
            unsafe { self.content[lti].assume_init_drop() };
        } else {
            self.is_some.set(lti, 1);
        }
        self.content[lti] = MaybeUninit::new(val);
    }


    pub fn set_none(&mut self, lti: u16) {
        if self.is_some.get(lti) != 0 {
            unsafe { self.content[lti].assume_init_drop() };
            self.is_some.set(lti, 0);
        }
    }
}

impl<T> Default for PerTileOption<T> {
    fn default() -> Self {
        PerTileOption::new()
    }
}

impl<T: Clone> Clone for PerTileOption<T> {
    fn clone(&self) -> Self {
        let mut clone = PerTileOption::new();
        for lti in 0..=MAX_LTI {
            if let Some(val) = self.get(lti) {
                clone.set_some(lti, val.clone());
            }
        }
        clone
    }
}

impl<T: Debug> Debug for PerTileOption<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f
            .debug_list()
            .entries((0..=MAX_LTI).map(|lti| self.get(lti)))
            .finish()
    }
}
