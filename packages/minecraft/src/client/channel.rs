//! Channel for sending asynchronous events to the client.

use crate::{
    util_abort_handle::AbortHandle,
    util_callback_cell::CallbackCell,
    client::ClientEvent,
    gui::GuiUserEventNotify,
};
use std::sync::Arc;
use tokio::sync::OwnedSemaphorePermit;
use crossbeam::{
    queue::SegQueue,
    sync::Parker,
};


/// Priority level. Variants decrease in priority.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(usize)]
pub enum EventPriority {
    /// Control events, processed basically immediately.
    Control = 0,
    /// Messages received from the server, to be processed fast to stay responsive.
    Network = 1,
    /// Other messages that can be processed more leisurely.
    Other = 2,
}

// number of priority levels
const LEVELS: usize = 3;


/// Create the channel for sending asynchronous events to the client.
pub fn channel() -> (ClientSender, ClientReceiver) {
    let state_0 = Arc::new(State::default());
    let state_1 = Arc::clone(&state_0);
    (ClientSender(state_0), ClientReceiver(state_1))
}

/// Sending half of client asynchronous event channel.
#[derive(Clone)]
pub struct ClientSender(Arc<State>);

/// Receiving half of client asynchronous event channel.
#[derive(Clone)]
pub struct ClientReceiver(Arc<State>);

// channel inner state
#[derive(Default)]
struct State {
    // queue for each priority level
    queues: [SegQueue<InnerMsg>; LEVELS],
    // callback cell for notifying when item is inserted
    callback: CallbackCell,
}

// internal message sent across queue
struct InnerMsg {
    event: ClientEvent,
    aborted: Option<AbortHandle>,
    permit: Option<OwnedSemaphorePermit>,
}

impl ClientSender {
    /// Send an event.
    pub fn send(
        &self,
        event: ClientEvent,
        priority: EventPriority,
        aborted: Option<AbortHandle>,
        permit: Option<OwnedSemaphorePermit>,
    ) {
        self.0.queues[priority as usize].push(InnerMsg { event, aborted, permit });
        self.0.callback.take_call();
    }
}

impl ClientReceiver {
    /// Poll for an event.
    pub fn poll(&self) -> Option<ClientEvent> {
        for queue in &self.0.queues {
            while let Some(InnerMsg { event, aborted, permit }) = queue.pop() {
                drop(permit);
                if aborted.map(|aborted| !aborted.is_aborted()).unwrap_or(true) {
                    return Some(event)
                }
            }
        }
        None
    }

    /// Put the callback to be run once after next time an event is sent into the channel.
    pub fn put_callback<F: FnOnce() + Send + 'static>(&self, callback: F) {
        self.0.callback.put(callback);
    }

    /// Convenience method poll and block until an event is found.
    pub fn poll_blocking(&self) -> ClientEvent {
        if let Some(event) = self.poll() {
            return event;
        }
        let parker = Parker::new();
        loop {
            let unparker = parker.unparker().clone();
            self.put_callback(move || unparker.unpark());
            if let Some(event) = self.poll() {
                return event;
            }
            parker.park();
        }
    }

    /// Convenience method to poll within a gui state frame context.
    pub fn poll_gui(&self, gui_notify: &GuiUserEventNotify) -> Option<ClientEvent> {
        if let Some(event) = self.poll() {
            Some(event)
        } else {
            let gui_notify = gui_notify.clone();
            self.put_callback(move || gui_notify.notify());
            self.poll()
        }
    }
}
