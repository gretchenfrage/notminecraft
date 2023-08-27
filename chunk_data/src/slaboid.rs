//! Slab-like utility.

use std::{
    ops::{Index, IndexMut},
    mem::replace,
};


/// Like a slab, but has a doubly linked free list allowing entries to be
/// inserted with a pre-determined key if desired.
#[derive(Debug, Clone)]
pub struct Slaboid<T> {
    entries: Vec<Entry<T>>,
    next_free: usize,
}

const NULL_IDX: usize = !0;

#[derive(Debug, Clone)]
enum Entry<T> {
    Present(T),
    Vacant {
        // if NULL_IDX, this is the last element in the free chain
        next_free: usize,
        // if NULL_IDX, this is the first element in the free chain
        // and thus Slaboid.next_free points to this
        prev_free: usize,
    },
}

impl<T> Slaboid<T> {
    pub fn new() -> Self {
        Slaboid {
            entries: Vec::new(),
            next_free: NULL_IDX,
        }
    }

    pub fn insert(&mut self, val: T) -> usize {
        if self.next_free != NULL_IDX {
            let key = self.next_free;
            let after = self.entries[key].next_free();
            if after != NULL_IDX {
                *self.entries[after].prev_free_mut() = NULL_IDX;
            }
            self.entries[key] = Entry::Present(val);
            key
        } else {
            let key = self.entries.len();
            self.entries.push(Entry::Present(val));
            key
        }
    }

    pub fn remove(&mut self, key: usize) -> T {
        assert!(
            matches!(self.entries[key], Entry::Present(_)),
            "attempt to remove vacant key",
        );
        match replace(
            &mut self.entries[key],
            Entry::Vacant {
                prev_free: NULL_IDX,
                next_free: self.next_free,
            },
        ) {
            Entry::Present(val) => {
                self.next_free = key;
                val
            },
            Entry::Vacant { .. } => unreachable!()
        }
    }

    pub fn vacant_key(&self) -> usize {
        if self.next_free != NULL_IDX {
            self.next_free
        } else {
            self.entries.len()
        }
    }

    pub fn contains_key(&self, key: usize) -> bool {
        self.entries
            .get(key)
            .map(|entry| matches!(entry, Entry::Present(_)))
            .unwrap_or(false)
    }

    pub fn insert_at(&mut self, key: usize, val: T) {
        if key < self.entries.len() {
            assert!(
                matches!(self.entries[key], Entry::Vacant { .. }),
                "attempt to insert at present key"
            );
            let old = replace(&mut self.entries[key], Entry::Present(val));
            if old.next_free() != NULL_IDX {
                *self.entries[old.next_free()].prev_free_mut() = key;
            }
            *if old.prev_free() != NULL_IDX {
                self.entries[old.prev_free()].next_free_mut()
            } else {
                &mut self.next_free
            } = key;
        } else {
            while key + 1 < self.entries.len() {
                let curr = self.entries.len();
                self.entries.push(Entry::Vacant {
                    prev_free: NULL_IDX,
                    next_free: self.next_free,
                });
                *self.entries[self.next_free].prev_free_mut() = curr;
                self.next_free = curr;
            }
            self.entries.push(Entry::Present(val));
        }
    }
}

impl<T> Index<usize> for Slaboid<T> {
    type Output = T;

    fn index(&self, i: usize) -> &T {
        match self.entries[i] {
            Entry::Present(ref val) => val,
            Entry::Vacant { .. } => panic!("key {} not present", i),
        }
    }
}

impl<T> IndexMut<usize> for Slaboid<T> {
    fn index_mut(&mut self, i: usize) -> &mut T {
        match self.entries[i] {
            Entry::Present(ref mut val) => val,
            Entry::Vacant { .. } => panic!("key {} not present", i),
        }
    }
}

impl<T> Entry<T> {
    fn next_free(&self) -> usize {
        match self {
            &Entry::Vacant { next_free, .. } => next_free,
            &Entry::Present(_) => unreachable!()
        }
    }

    fn prev_free(&self) -> usize {
        match self {
            &Entry::Vacant { prev_free, .. } => prev_free,
            &Entry::Present(_) => unreachable!()
        }
    }

    fn next_free_mut(&mut self) -> &mut usize {
        match self {
            &mut Entry::Vacant { ref mut next_free, .. } => next_free,
            &mut Entry::Present(_) => unreachable!()
        }
    }

    fn prev_free_mut(&mut self) -> &mut usize {
        match self {
            &mut Entry::Vacant { ref mut prev_free, .. } => prev_free,
            &mut Entry::Present(_) => unreachable!()
        }
    }
}

impl<T> Default for Slaboid<T> {
    fn default() -> Self {
        Slaboid::new()
    }
}
