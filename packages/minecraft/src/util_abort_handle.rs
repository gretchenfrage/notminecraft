//! System for marking requests as aborted when a guard is dropped.

use std::sync::{
    Arc,
    atomic::{
        AtomicBool,
        Ordering,
    },
};


/// Handle for marking some request as aborted. Really just a `Arc<AtomicBool>`.
#[derive(Default, Debug, Clone)]
pub struct AbortHandle(Arc<AtomicBool>);

impl AbortHandle {
    /// Construct not aborted.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check whether aborted.
    pub fn is_aborted(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }

    /// Mark as aborted.
    pub fn abort(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
}


/// Wrapper around `AbortHandle` which aborts when dropped.
#[derive(Default, Debug)]
#[must_use]
pub struct AbortGuard(AbortHandle);

impl AbortGuard {
    /// Construct not aborted.
    pub fn new() -> Self {
        Self::default()
    }

    /// Clone a new connected `AbortHandle`.
    pub fn new_handle(&self) -> AbortHandle {
        self.0.clone()
    }

    /// Mark as aborted (which is also done when dropped).
    pub fn abort(&self) {
        self.0.abort();
    }
}

impl Drop for AbortGuard {
    fn drop(&mut self) {
        self.0.abort();
    }
}
