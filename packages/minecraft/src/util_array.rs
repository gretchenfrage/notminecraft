//! Utilities for manipulating constant-length arrays.

use std::{
    mem::MaybeUninit,
    ptr::drop_in_place,
};


// TODO: the version of this in the stdlib is unstable

pub fn array_each<
    'a,
    T,
    const LEN: usize,
>(array: &'a [T; LEN]) -> [&'a T; LEN]
{
    unsafe {
        let mut array2: MaybeUninit<[&'a T; LEN]> = MaybeUninit::uninit();
        for (i, item) in array.iter().enumerate() {
            *(array2.as_mut_ptr() as *mut &'a T).add(i) = item;
        }
        array2.assume_init()
    }
}

pub fn array_each_mut<
    'a,
    T,
    const LEN: usize,
>(array: &'a mut [T; LEN]) -> [&'a mut T; LEN]
{
    unsafe {
        let mut array2: MaybeUninit<[&'a mut T; LEN]> = MaybeUninit::uninit();
        for (i, item) in array.iter_mut().enumerate() {
            *(array2.as_mut_ptr() as *mut &'a mut T).add(i) = item;
        }
        array2.assume_init()
    }
}

pub fn array_const_slice<
    T,
    const LEN: usize,
>(array: &[T], start: usize) -> &[T; LEN]
{
    unsafe {
        &*((&array[start..start + LEN]) as *const [T] as *const [T; LEN])
    }
}

pub fn array_const_slice_mut<
    T,
    const LEN: usize,
>(array: &mut [T], start: usize) -> &mut [T; LEN]
{
    unsafe {
        &mut *((&mut array[start..start + LEN]) as *mut [T] as *mut [T; LEN])
    }
}

/// Safely building a constant size array.
pub struct ArrayBuilder<T, const N: usize> {
    array: MaybeUninit<[T; N]>,
    i: usize, // index of next element to initialize
}

impl<T, const N: usize> ArrayBuilder<T, N> {
    pub fn new() -> Self {
        ArrayBuilder {
            array: MaybeUninit::uninit(),
            i: 0,
        }
    }

    pub fn push(&mut self, elem: T) {
        unsafe {
            assert!(self.i < N, "push to fully filled ArrayBuilder");
            (self.array.as_mut_ptr() as *mut T).add(self.i).write(elem);
            self.i += 1;
        }
    }

    pub fn build(mut self) -> [T; N] {
        unsafe {
            assert!(self.i == N, "unfinished ArrayBuilder.build");
            self.i = 0; // do this so destructor won't drop any elements
            self.array.as_ptr().read()
        }
    }
}

impl<T, const N: usize> Drop for ArrayBuilder<T, N> {
    fn drop(&mut self) {
        unsafe {
            // drop only initialized elements
            for j in 0..self.i {
                drop_in_place((self.array.as_mut_ptr() as *mut T).add(j));
            }
        }
    }
}

pub fn array_from_fn<F: FnMut(usize) -> T, T, const N: usize>(mut f: F) -> [T; N] {
    let mut array = ArrayBuilder::new();
    for i in 0..N {
        array.push(f(i));
    }
    array.build()
}

pub fn array_default<T: Default, const N: usize>() -> [T; N] {
    array_from_fn(|_| T::default())
}

/*

    doesn't compile on stable yet


pub fn array_chain<
    T,
    const A_LEN: usize,
    const B_LEN: usize,
>(a: [T; A_LEN], b: [T; B_LEN]) -> [T; A_LEN + B_LEN]
{
    unsafe {
        let mut target: MaybeUninit<[T; A_LEN + B_LEN]> = MaybeUninit::uninit();
        ptr::copy(
            &a as *const [T; A_LEN] as *const T,
            target.as_mut_ptr() as *mut T,
            A_LEN,
        );
        ptr::copy(
            &b as *const [T; B_LEN] as *const T,
            (target.as_mut_ptr() as *mut T).add(A_LEN),
            B_LEN,
        );
        mem::forget(a);
        mem::forget(b);
        target.assume_init()
    }
}
*/
