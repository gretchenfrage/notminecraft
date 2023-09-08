
use std::ops::{
    Index,
    IndexMut,
};


#[derive(Debug, Clone)]
pub struct SparseVec<T> {
    vec: Vec<Option<T>>,
    size: usize,
}

impl<T> SparseVec<T> {
    pub fn new() -> Self {
        SparseVec {
            vec: Vec::new(),
            size: 0,
        }
    }

    pub fn set(&mut self, index: usize, elem: T) {
        while self.vec.len() <= index {
            self.vec.push(None);
        }
        if self.vec[index].is_none() {
            self.size += 1;
        }
        self.vec[index] = Some(elem)
    }

    pub fn remove(&mut self, index: usize) -> T {
        let result = self.vec[index].take().unwrap();
        self.size -= 1;
        result
    }

    pub fn iter(&self) -> impl Iterator<Item=(usize, &T)> {
        self.vec.iter()
            .enumerate()
            .filter_map(|(i, opt)| opt.as_ref().map(|elem| (i, elem)))
    }
    /*
    pub fn size(&self) -> usize {
        self.size
    }

    pub fn set_avoids_unnecessary_growth(&self, index: usize) -> bool {
        debug_assert!(self.size <= self.vec.len());
        if self.size < self.vec.len() {
            index
        }
    }
    */
}

impl<T> Index<usize> for SparseVec<T> {
    type Output = T;

    fn index(&self, i: usize) -> &T {
        self.vec[i].as_ref().unwrap()
    }
}

impl<T> IndexMut<usize> for SparseVec<T> {
    fn index_mut(&mut self, i: usize) -> &mut T {
        self.vec[i].as_mut().unwrap()
    }
}
