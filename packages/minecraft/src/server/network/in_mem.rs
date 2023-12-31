//! Network connection implementation for in-memory transport between client and internal server.

use super::*;
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


// connection inner type for in-mem connections
pub(super) struct Connection(Arc<State>);

/// Client connection to the network server that just sends messages over in-memory queues within
/// the same process, avoiding both network and serialization costs. Closes the connection if
/// dropped.
pub struct InMemClient(Arc<State>);

// shared state for an open in-mem connection
struct InMemShared {
    // network server shared state
    ns_shared: Arc<NetworkServerSharedState>,
    // in-mem connection's connection index, or none if connection has been closed
    conn_idx: Mutex<Option<usize>>,
    // sender for down messages
    down_queue: SegQueue<DownMsg>,
    // whether the server has killed the connection. lags behind whether conn_idx is None--the
    // conn_idx mutex synchronizes the server killing the connection with the client sending to the
    // connection, such that that message network events won't be sent after remove connection
    // events.
    killed: AtomicBool,
}

// create an in-mem client for the server
pub(super) fn create(server: &mut NetworkServer) -> InMemClient {
    let shared_1 = Arc::new(InMemShared {
        ns_shared: Arc::clone(&server.shared),
        conn_idx: Mutex::new(None),
        down_queue: SegQueue::new(),
        killed: AtomicBool::new(false),
    });
    let shared_2 = Arc::clone(&shared_1);

    let conn_idx =
        create_conn(
            &server.shared,
            super::SlabEntry::InMem,
            super::Connection(ConnectionInner(Connection(shared_1)))
        )
        .unwrap(); // we have &mut server -> server isn't dropped -> should return some

    let mut conn_idx_lock = shared_2.conn_idx.lock();
    *conn_idx_lock = Some(conn_idx);
    drop(lock);
    
    InMemClient(shared_2)
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
        let conn_idx_lock = self.0.conn_idx.lock();
        if let Some(conn_idx) = *conn_idx_lock {
            self.0.ns_shared.server_send.send(
                ServerEvent::Network(NetworkEvent::Message(conn_idx, msg)),
                EventPriority::Network,
                None,
                None,
            );
        }
        drop(conn_idx_lock);
    }

    /// Poll for a message received from the server.
    /// 
    /// Errors if the server has shut down or closed this connection.
    pub fn poll(&self) -> Result<Option<DownMessage>> {
        if self.0.killed.load(Ordering::Relaxed) {
            Err(anyhow!("server killed in-mem connection"))
        } else {
            Ok(self.0.down_queue.pop())
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
    let mut conn_idx_lock = shared.conn_idx.lock();
    if let Some(conn_idx) = *conn_idx_lock {
        destroy_conn(&shared.ns_shared, conn_idx);
    }
    *conn_idx_lock = None;
    drop(conn_idx_lock);
}
