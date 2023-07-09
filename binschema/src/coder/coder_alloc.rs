
use crate::coder::coder::StackFrame;
use std::mem::forget;


/// Reusable allocation for `CoderState`.
#[derive(Debug)]
pub struct CoderStateAlloc {
    ptr: *mut (),
    capacity: usize, // capacity unit is StackFrame
}

impl CoderStateAlloc {
    pub fn new() -> Self {
        Self::from_stack(Vec::new())
    }

    pub(super) fn from_stack(mut stack: Vec<StackFrame<'_>>) -> Self {
        stack.clear();
        let ptr = stack.as_mut_ptr() as *mut ();
        let capacity = stack.capacity();
        forget(stack);
        CoderStateAlloc { ptr, capacity }
    }

    pub(super) fn into_stack<'a>(self) -> Vec<StackFrame<'a>> {
        unsafe {
            let stack = Vec::from_raw_parts(
                self.ptr as *mut StackFrame<'a>,
                0,
                self.capacity,
            );
            forget(self);
            stack
        }
    }
}

impl Drop for CoderStateAlloc {
    fn drop(&mut self) {
        unsafe {
            drop(Vec::from_raw_parts(
                self.ptr as *mut StackFrame<'_>,
                0,
                self.capacity,
            ));
        }
    }
}

impl Default for CoderStateAlloc {
    fn default() -> Self {
        CoderStateAlloc::new()
    }
}

impl Clone for CoderStateAlloc {
    fn clone(&self) -> Self {
        CoderStateAlloc::new()
    }
}
