
mod callback_cell;

use self::callback_cell::CallbackCell;
use crate::util_abort_handle::AbortHandle;
use std::{
    any::Any,
    sync::Arc,
};
use tokio::sync::OwnedSemaphorePermit;
use crossbeam::queue::SegQueue;

#[derive(Clone)]
pub struct DynFlexSender(Arc<State>);

#[derive(Clone)]
pub struct DynFlexReceiver(Arc<State>);

#[derive(Default)]
struct State {
    queue: SegQueue<InnerMsg>,
    callback: CallbackCell,
}

struct InnerMsg {
    obj: Box<dyn Any + Send>,
    aborted: Option<AbortHandle>,
    permit: Option<OwnedSemaphorePermit>,
}


pub fn channel() -> (DynFlexSender, DynFlexReceiver) {
    let state_1 = Arc::new(Default::default());
    let state_2 = Arc::clone(&state_1);
    (DynFlexSender(state_1), DynFlexReceiver(state_2))
}

impl DynFlexSender {
    pub fn send(
        &self,
        obj: Box<dyn Any + Send>,
        aborted: Option<AbortHandle>,
        permit: Option<OwnedSemaphorePermit>,
    ) {
        self.0.queue.push(InnerMsg { obj, aborted, permit });
        self.0.callback.take_call();
    }
}

impl DynFlexReceiver {
    pub fn poll(&self) -> Option<Box<dyn Any + Send>> {
        loop {
            let msg = self.0.queue.pop()?;
            drop(msg.permit);
            if msg.aborted.map(|aborted| !aborted.is_aborted()).unwrap_or(true) {
                return Some(msg.obj);
            }
        }
    }

    pub fn put_callback<F: FnOnce() + Send + 'static>(&self, callback: F) {
        self.0.callback.put(callback);
    }
}
