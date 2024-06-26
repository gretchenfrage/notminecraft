
use std::{
    mem::{MaybeUninit, size_of},
    fmt::{self, Formatter, Debug},
};


#[derive(Copy, Clone)]
pub struct ErasedOptionRef(MaybeUninit<usize>);

impl ErasedOptionRef {
    pub fn new() -> Self {
        ErasedOptionRef(MaybeUninit::uninit())
    }
    
    pub fn as_opt_ref<'o, 's, T>(&'s mut self) -> &'s mut Option<&'o T> {
        debug_assert_eq!(size_of::<Option<&'o T>>(), size_of::<usize>());
        unsafe {
            let ptr = self.0.as_mut_ptr() as *mut Option<&'o T>;
            ptr.write(None);
            &mut *ptr
        }
    }
    
    pub fn as_opt_mut_ref<'o, 's, T>(&'s mut self) -> &'s mut Option<&'o mut T> {
        debug_assert_eq!(size_of::<Option<&'o mut T>>(), size_of::<usize>());
        unsafe {
            let ptr = self.0.as_mut_ptr() as *mut Option<&'o mut T>;
            ptr.write(None);
            &mut *ptr
        }
    }
}

impl Default for ErasedOptionRef {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for ErasedOptionRef {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("ErasedOptionRef")
    }
}
