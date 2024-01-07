//! Network connection implementation for in-memory transport between client and internal server.

use super::{
    send_buffer_policy_enforcer::SendBufferPolicyEnforcer,
    *,
};
use crate::server::{
    channel::*,
    ServerEvent,
};
use std::sync::{
    atomic::{
        AtomicBool,
        Ordering,
    },
    Arc,
};
use crossbeam::queue::SegQueue;
use parking_lot::Mutex;
use anyhow::*;


// slab entry inner type for in-mem connections
pub(super) struct SlabEntry(Arc<InMemShared>);

// connection inner type for in-mem connections
pub(super) struct Connection(Arc<InMemShared>);

/// Client connection to the network server that just sends messages over in-memory queues within
/// the same process, avoiding both network and serialization costs. Closes the connection if
/// dropped.
pub struct InMemClient {
    // shared state
    shared: Arc<InMemShared>,
    // we put this "client-side" to help catch bugs, but in a strict sense it's not actually
    // necessary for in-mem connections at all.
    sbpe: SendBufferPolicyEnforcer,
}

// shared state for an open in-mem connection
#[derive(Default)]
struct InMemShared {
    // lockable alive state, if alive. mutex serializes connection index life cycle.
    alive_state: Mutex<Option<AliveState>>,
    // sender for down messages
    down_queue: SegQueue<DownMsg>,
    // whether connection has been killed. lags behind the alive mutex in an eventually consistent
    // sense, so is kind of like the client-side perspective of whether alive.
    killed: AtomicBool,
}

// state that's kept iff the in-mem connection is alive
struct AliveState {
    // network server shared state
    ns_shared: Arc<NetworkServerSharedState>,
    // connection index
    conn_idx: usize,
}

// create an in-mem client for the server
pub(super) fn create(server: &mut NetworkServer) -> InMemClient {
    let shared = Arc::new(InMemShared::default());
    let slab_entry = super::SlabEntry::InMem(SlabEntry(Arc::clone(&shared)));
    let connection = super::Connection(ConnectionInner::InMem(Connection(Arc::clone(&shared))));
    // unwrap safety: we have &mut server -> server isn't dropped -> should return some
    let conn_idx = create_conn(&server.shared, slab_entry, connection).unwrap();

    let mut alive_lock = shared.alive_state.lock();
    *alive_lock = Some(AliveState {
        ns_shared: Arc::clone(&server.shared),
        conn_idx,
    });
    drop(alive_lock);

    InMemClient {
        shared,
        sbpe: SendBufferPolicyEnforcer::default(),
    }
}

impl SlabEntry {
    // called upon network shutdown
    pub(super) fn shutdown(&self) {
        self.0.killed.store(true, Ordering::Relaxed);
        *self.0.alive_state.lock() = None;
    }
}

impl Connection {
    // see outer type
    pub(super) fn send(&self, msg: DownMsg) {
        let _ = self.0.down_queue.push(msg);
    }

    // see outer type
    pub(super) fn kill(&self) {
        kill(&self.0);
    }
}

impl InMemClient {
    /// Send message to the server.
    ///
    /// It is undefined whether the queueing of these messages occurs in the server or in the
    /// client in this case because they are using the same memory.
    pub fn send(&self, msg: UpMsg) {
        let alive_lock = self.shared.alive_state.lock();
        if let Some(alive_state) = alive_lock.as_ref() {
            alive_state.ns_shared.server_send.send(
                ServerEvent::Network(NetworkEvent::Message(conn_idx, msg)),
                EventPriority::Network,
                None,
                None,
            );
        }
    }

    /// Poll for a message received from the server.
    /// 
    /// Errors if the server has shut down or closed this connection.
    pub fn poll(&self) -> Result<Option<DownMessage>> {
        if self.shared.killed.load(Ordering::Relaxed) {
            bail!("server killed in-mem connection");
        } else {
            Ok(self.shared.down_queue.pop())
        }
    }
}

impl Drop for InMemClient {
    fn drop(&mut self) {
        kill(&self.0);
    }
}

// kill the in-mem connection, if not already killed. pretty much synchronization-safe.
fn kill(shared: &InMemShared) {
    shared.killed.store(true, Ordering::Relaxed);
    let mut alive_lock = shared.alive_state.lock();
    if let Some(alive_state) = alive_lock.as_ref() {
        destroy_conn(&alive_state.ns_shared, alive_state.conn_idx);
    }
    *alive_lock = None;
}
