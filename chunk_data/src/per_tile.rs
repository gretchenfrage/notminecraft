
use crate::coord::NUM_LTIS;
use std::{
    ops::{
        Index,
        IndexMut,
    },
    iter::{
        repeat,
        FromIterator,
    },
    alloc::{
        alloc,
        Layout,
    },
    panic::{
        catch_unwind,
        resume_unwind,
        AssertUnwindSafe,
    },
    ptr::drop_in_place,
};


/// Per-tile (within a chunk) storage of `T` via an array.
///
/// Implements `FromIterator`, as well as `Index`/`IndexMut<u16>`.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PerTile<T>(pub Box<[T; NUM_LTIS]>);

impl<T> PerTile<T> {
    /// Construct with 2^16 clones of `val`.
    pub fn repeat(val: T) -> Self
    where
        T: Clone,
    {
        repeat(val).take(NUM_LTIS).collect()
    }
}

impl<T: Default> Default for PerTile<T> {
    fn default() -> Self {
        (0..NUM_LTIS).map(|_| T::default()).collect()
    }
}

/// Panics if iterator contains too many or too few items.
impl<T> FromIterator<T> for PerTile<T> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item=T>,
    {
        let mut iter = iter.into_iter();

        unsafe fn partial_drop<T>(ptr: *mut T, i: usize) {
            for j in (0..i).rev() {
                drop_in_place(&mut *ptr.add(j));
            }
        }

        let array = unsafe {
            let layout = Layout::array::<T>(NUM_LTIS).unwrap();
            let ptr = alloc(layout) as *mut T;

            for i in 0..NUM_LTIS {
                let next = catch_unwind(AssertUnwindSafe(|| iter.next()));
                let item = match next {
                    Ok(Some(item)) => item,
                    Ok(None) => {
                        partial_drop(ptr, i);
                        panic!("not enough items in iter");
                    },
                    Err(e) => {
                        partial_drop(ptr, i);
                        resume_unwind(e);
                    },
                };

                *ptr.add(i) = item;
            }

            Box::from_raw(ptr as *mut [T; NUM_LTIS])
        };

        assert!(iter.next().is_none(), "too many items in iter");
        PerTile(array)
    }
}

impl<T> Index<u16> for PerTile<T> {
    type Output = T;

    fn index(&self, i: u16) -> &T {
        &self.0[i as usize]
    }
}

impl<T> IndexMut<u16> for PerTile<T> {
    fn index_mut(&mut self, i: u16) -> &mut T {
        &mut self.0[i as usize]
    }
}
