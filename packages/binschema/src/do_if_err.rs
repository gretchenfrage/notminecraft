//! Extension trait to Result.

pub trait DoIfErr: Sized {
    fn do_if_err<F: FnOnce()>(self, f: F) -> Self;
}

impl<I, E> DoIfErr for Result<I, E> {
    fn do_if_err<F: FnOnce()>(self, f: F) -> Self {
        if self.is_err() {
            f();
        }
        self
    }
}
