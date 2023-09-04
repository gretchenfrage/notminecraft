
use std::ops::{
    Index,
    IndexMut,
};


#[derive(Debug, Clone)]
pub struct SparseVec<T>(pub Vec<Option<T>>);

impl<T> SparseVec<T> {
    pub fn new() -> Self {
        SparseVec(Vec::new())
    }

    pub fn set(&mut self, index: usize, elem: T) {
        while self.0.len() <= index {
            self.0.push(None);
        }
        self.0[index] = Some(elem)
    }

    pub fn remove(&mut self, index: usize) -> T {
        self.0[index].take().unwrap()
    }

    pub fn iter(&self) -> impl Iterator<Item=(usize, &T)> {
        self.0.iter()
            .enumerate()
            .filter_map(|(i, opt)| opt.as_ref().map(|elem| (i, elem)))
    }
}

impl<T> Index<usize> for SparseVec<T> {
    type Output = T;

    fn index(&self, i: usize) -> &T {
        self.0[i].as_ref().unwrap()
    }
}

impl<T> IndexMut<usize> for SparseVec<T> {
    fn index_mut(&mut self, i: usize) -> &mut T {
        self.0[i].as_mut().unwrap()
    }
}
