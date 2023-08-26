
use std::{
    rc::{Rc, Weak},
    cell::RefCell,
    ops::{Deref, DerefMut},
    mem,
};


/// Pool for reusing `Vec<u8>` in network code.
#[derive(Debug, Default, Clone)]
pub struct BufferPool {
    pool: Rc<RefCell<Vec<Vec<u8>>>>,
}

fn inner_return(
    mut buf: Vec<u8>,
    pool: &Rc<RefCell<Vec<Vec<u8>>>>,
) {
    if buf.capacity() > 0 {
        buf.clear();
        pool.borrow_mut.push(buf)
    }
}

impl BufferPool {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn produce(&self) -> PooledBuffer {
        PooledBuffer {
            buf: self.pool.borrow_mut().pop().unwrap_or_default(),
            pool: self.pool.downgrade(),
        }
    }

    pub fn return(&self, buf: Vec<u8>) {
        inner_return(buf, &self.pool)
    }
}

/// Deref wrapper around a `Vec<u8>` that returns it into the buffer pool
/// when dropped.
#[derive(Debug)]
pub struct PooledBuffer {
    buf: Vec<u8>,
    pool: Weak<RefCell<Vec<Vec<u8>>>>,
}

impl PooledBuffer {
    pub fn take(mut self) -> Vec<u8> {
        mem::take(&mut self.buf)
    }
}

impl Into<Vec<u8>> for PooledBuffer {
    fn into(self) -> Vec<u8> {
        self.take()
    }
}

impl Deref for PooledBuffer {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.buf
    }
}

impl DerefMut for PooledBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buf
    }
}

impl Drop for PooledBuffer {
    fn drop(&mut self) {
        if let Some(pool) = self.pool.upgrade() {
            inner_return(mem::take(&mut self.buf), &pool);
        }
    }
}
