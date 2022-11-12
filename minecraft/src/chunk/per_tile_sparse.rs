
use crate::chunk::{
    coord::{
        MAX_LTI,
        NUM_LTIS,
    },
    per_tile::PerTile,
};


/// Sparse per-tile storage.
///
/// A chunk contains 2^16 tiles, and so a normal `PerTile<T>` stores a `T` for
/// every tile, in an array.
///
/// A `PerTileSparse<T>` gives the abstraction of storing an `Option<T>` for
/// each tile, but does so sparsely, using a vector that can grow and shrink
/// based on the number of `Some` elements, and storing a static table of `u16`
/// indexes.
///
/// A `PerTile<T>` always uses 2^16 * `size_of::<T>()` bytes. A
/// `PerTileSparse<T>` has a constant overhead of 2^16 * 2 bytes, and then
/// consumes `size_of::<T>() + 2` bytes per `Some` element.
///
/// For example, a `PerTile<u64>` would consume 512 KiB, whereas a
/// `PerTileSparse<u64>` would consume (128 + 640f) KiB, wherein f is the
/// fraction of the tiles that are `Some`. This means that, so long as fewer
/// than 60% of the tiles are `Some`, memory has been saved.
///
/// Generally, where M = `size_of::<T>()`, it is worth it to use
/// `PerTileSparse` if the fraction of tiles that are `Some` is less than
/// (M - 2) / (M + 2).
///
/// Here's a table where,for various sizes of per-tile data, up to that percent
/// of tiles could be `Some` and `PerTileSparse` would still save memory:
///
/// | size | percent |
/// |------|---------|
/// | 0    | -100%   |
/// | 1    | -33%    |
/// | 2    | 0%      |
/// | 3    | 20%     |
/// | 4    | 33%     |
/// | 5    | 43%     |
/// | 6    | 50%     |
/// | 7    | 55%     |
/// | 8    | 60%     |
/// | 9    | 64%     |
/// | 10   | 67%     |
/// | 11   | 69%     |
/// | 12   | 71%     |
/// | 13   | 73%     |
/// | 14   | 75%     |
/// | 15   | 76%     |
/// | 16   | 78%     |
/// | 32   | 89%     |
/// | 64   | 94%     |
/// | 128  | 97%     |
#[derive(Debug, Clone)]
pub struct PerTileSparse<T> {
    // an idx MAX_LTI denotes None
    // UNLESS
    // vec.len() == NUM_LTIS
    idxs: PerTile<u16>,
    vec: Vec<VecItem<T>>,
}

#[derive(Debug, Clone)]
struct VecItem<T> {
    lti: u16,
    val: T,
}

impl<T> PerTileSparse<T> {
    pub fn new() -> Self {
        PerTileSparse {
            idxs: PerTile::repeat(MAX_LTI),
            vec: Vec::new(),
        }
    }

    fn get_idx(&self, lti: u16) -> Option<usize> {
        let idx = self.idxs[lti];
        if idx != MAX_LTI || self.vec.len() == NUM_LTIS {
            Some(idx as usize)
        } else {
            None
        }
    }

    pub fn get(&self, lti: u16) -> Option<&T> {
        self.get_idx(lti)
            .map(|idx| &self.vec[idx].val)
    }

    pub fn get_mut(&mut self, lti: u16) -> Option<&mut T> {
        self.get_idx(lti)
            .map(|idx| &mut self.vec[idx].val)
    }

    pub fn iter_some<'s>(&'s self) -> impl Iterator<Item=(u16, &'s T)> + 's {
        self.vec
            .iter()
            .map(|item| (item.lti, &item.val))
    }

    pub fn iter_some_mut<'s>(&'s mut self) -> impl Iterator<Item=(u16, &'s mut T)> + 's {
        self.vec
            .iter_mut()
            .map(|item| (item.lti, &mut item.val))
    }

    pub fn set(&mut self, lti: u16, val: Option<T>) {
        if let Some(val) = val {
            self.set_some(lti, val);
        } else {
            self.set_none(lti);
        }
    }

    pub fn set_some(&mut self, lti: u16, val: T) {
        if let Some(idx) = self.get_idx(lti) {
            self.vec[idx].val = val;
        } else {
            let idx = self.vec.len() as u16;
            self.vec.push(VecItem { lti, val });
            self.idxs[lti] = idx;
        }
    }

    pub fn set_none(&mut self, lti: u16) {
        if let Some(idx) = self.get_idx(lti) {            
            // swap-remove and update idx of element we replace with
            if idx + 1 == self.vec.len() {
                // trivial case
                self.vec.pop().unwrap();
            } else {
                // non-trivial case
                let replace_with = self.vec.pop().unwrap();
                self.idxs[replace_with.lti] = idx as u16;
                self.vec[idx] = replace_with;
            }

            // and null idx
            self.idxs[lti] = MAX_LTI;
        }
    }
}

impl<T> Default for PerTileSparse<T> {
    fn default() -> Self {
        PerTileSparse::new()
    }
}
