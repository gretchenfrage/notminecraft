
use crate::chunk::{
    coord::{
        MAX_LTI,
        NUM_LTIS,
    },
    per_tile::PerTile,
};


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
