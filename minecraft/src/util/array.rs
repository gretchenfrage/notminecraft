
use std::mem::MaybeUninit;


// TODO: the version of this in the stdlib is unstable

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


pub fn array_const_slice_mut<
    T,
    const LEN: usize,
>(array: &mut [T], start: usize) -> &mut [T; LEN]
{
    unsafe {
        &mut *((&mut array[start..start + LEN]) as *mut [T] as *mut [T; LEN])
    }
}
