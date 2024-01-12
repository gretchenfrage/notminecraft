//! Channel for sending events to the server loop.

use crate::{
    util_abort_handle::AbortHandle,
    server::ServerEvent,
};
use std::{
    time::Instant,
    sync::Arc,
};
use parking_lot::{Mutex, Condvar};
use tokio::sync::OwnedSemaphorePermit;
use crossbeam::queue::SegQueue;


/// Priority level. Variants decrease in priority.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(usize)]
pub enum EventPriority {
    /// Control events, sent to the server from some sort of administrative context.
    Control = 0,
    /// Messages received from the network, to be processed fast to stay responsive.
    Network = 1,
    /// Other messages that can be processed more leisurely.
    Other = 2,
}

// number of priority levels
const LEVELS: usize = 3;


/// Create the channel for sending events to the server loop.
///
/// The server consists of the primary "server thread" loop, which spends its time processing
/// events, sleeping, and triggering side effects including creating and sending events to various
/// forms of asynchronous helper thread systems, and those asynchronous helper thread systems,
/// which do asynchronous work such as disk and network IO and then send the results back to the
/// server in the form of events. This channel is the means by which these events are sent back to
/// the server.
///
/// This channel supports some features that are not available in a stock mpsc implementation,
/// including:
///
/// - Different priority levels for events, and the ability to poll with a certain necessary
///   priority.
/// - The ability to associate with a sent event an abort handle such that whether the event was
///   aborted is checked upon receipt and discarded if so. By doing this check upon receipt it is
///   possible to rely on it to avoid certain race conditions.
/// - The ability to associate with a sent event a receipt notification handle such that the handle
///   is notified when the event is taken by the receiver, thus allowing event source-specific
///   backpressure.
///
/// Within a priority levels, event delivery is FIFO.
pub fn channel() -> (ServerSender, ServerReceiver) {
    let state_0 = Arc::new(State::default());
    let state_1 = Arc::clone(&state_0);
    (ServerSender(state_0), ServerReceiver(state_1))
}

/// Sending half of server event channel.
#[derive(Clone)]
pub struct ServerSender(Arc<State>);

/// Receiving half of server event channel.
#[derive(Clone)]
pub struct ServerReceiver(Arc<State>);

// channel inner state
#[derive(Default)]
struct State {
    // queue for each priority level
    queues: [SegQueue<InnerMsg>; LEVELS],
    // mutex to track the size of each queue
    sizes: Mutex<[usize; LEVELS]>,
    // condvar to monitor changes to sizes
    sizes_cvar: Condvar,
}

// internal message sent across queue
struct InnerMsg {
    event: ServerEvent,
    aborted: Option<AbortHandle>,
    permit: Option<OwnedSemaphorePermit>,
}

impl ServerSender {
    /// Send an event on the given priority level.
    ///
    /// If `aborted` is provided, the receiver will discard the event upon taking from the queue
    /// if it has been marked as aborted when taken from the queue.
    ///
    /// If `permit` is provided, it will be dropped once taken from the queue, even if aborted.
    /// This can be used to implement fine-grained backpressure.
    pub fn send(
        &self,
        event: ServerEvent,
        priority: EventPriority,
        aborted: Option<AbortHandle>,
        permit: Option<OwnedSemaphorePermit>,
    ) {
        self.0.queues[priority as usize].push(InnerMsg { event, aborted, permit });
        self.0.sizes.lock()[priority as usize] += 1;
        self.0.sizes_cvar.notify_one();
    }
}

impl ServerReceiver {
    /// Attempt to receive an event.
    ///
    /// Higher priority events will be taken before lower priority ones.
    ///
    /// If `block_until` is provided, may block until that instant if waiting for event to become
    /// available. Otherwise, will never block.
    ///
    /// If `priority_lteq` is provided, will only take events with priority less than or equal to
    /// that value (as in, of that priority or a greater priority, since the ordering is
    /// backwards).
    pub fn recv(
        &self,
        block_until: Option<Instant>,
        priority_lteq: Option<EventPriority>,
    ) -> Option<ServerEvent> {
        loop {
            let mut sizes = self.0.sizes.lock();
            let found = loop {
                let bound = priority_lteq.map(|p| p as usize + 1).unwrap_or(LEVELS);
                if let Some(found) = (0..bound).find(|&i| sizes[i] > 0) {
                    break found;
                } else if let Some(deadline) = block_until {
                    let result = self.0.sizes_cvar.wait_until(&mut sizes, deadline);
                    if result.timed_out() {
                        return None;
                    }
                } else {
                    return None;
                }
            };
            sizes[found] -= 1;
            drop(sizes);

            let InnerMsg { event, aborted, permit } = self.0.queues[found].pop().unwrap();
            drop(permit);
            if aborted.map(|aborted| !aborted.is_aborted()).unwrap_or(true) {
                return Some(event)
            }
        }
    }
}
