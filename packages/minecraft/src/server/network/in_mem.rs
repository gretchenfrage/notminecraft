//! Network connection implementation for in-memory transport between client and internal server.

use super::*;
use crate::{
    server::ServerEvent,
    dyn_flex_channel::{self, *},
    client::connection::ConnectionEvent,
};
use std::{
    sync::Arc,
    fmt::{self, Formatter, Debug},
    write,
};
use parking_lot::Mutex;

// TODO: get send buffer policy enforcement working again

// slab entry inner type for in-mem connections
pub(super) struct SlabEntry(Arc<Mutex<Option<AliveState>>>);

// connection inner type for in-mem connections
pub(super) struct Connection(Arc<Mutex<Option<AliveState>>>);

/// Client connection to the network server that just sends messages over in-memory queues within
/// the same process, avoiding both network and serialization costs.
pub struct InMemClient {
    // shared state
    shared: Arc<Mutex<Option<AliveState>>>,
    // receiver for down-going client ConnectionEvent
    recv_down: DynFlexReceiver,
}

// shared lockable state that's kept iff the in-mem connection is alive
struct AliveState {
    // network server shared state
    ns_shared: Arc<NetworkServerSharedState>,
    // connection index
    conn_idx: usize,
    // sender for down-going client ConnectionEvent
    send_down: DynFlexSender,
}

// create an in-mem client for the server
pub(super) fn create(ns_shared: &Arc<NetworkServerSharedState>) -> InMemClient {
    let (send_down, recv_down) = dyn_flex_channel::channel();
    let shared = Arc::new(Mutex::new(None));
    let slab_entry = super::SlabEntry::InMem(SlabEntry(Arc::clone(&shared)));
    let connection = super::Connection(ConnectionInner::InMem(Connection(Arc::clone(&shared))));
    let conn_idx = create_conn(ns_shared, slab_entry, connection);

    if let Some(conn_idx) = conn_idx {
        // normal case
        let mut shared_lock = shared.lock();
        *shared_lock = Some(AliveState {
            ns_shared: Arc::clone(ns_shared),
            conn_idx,
            send_down,
        });
        drop(shared_lock);
    } else {
        // this case happens if the whole network server has been dropped
        send_down.send(
            Box::new(ConnectionEvent::Closed(Some("server closed".to_owned()))),
            None,
            None,
        );
    }

    InMemClient { shared, recv_down }
}

impl SlabEntry {
    // called upon network shutdown
    pub(super) fn shutdown(&self) {
        kill(&self.0, false, Some("server closed"));
    }
}

impl Connection {
    // see outer type
    pub(super) fn send(&self, msg: DownMsg) {
        let alive_lock = self.0.lock();
        if let &Some(ref alive_state) = &*alive_lock {
            alive_state.send_down.send(Box::new(ConnectionEvent::Received(msg)), None, None);
        } else {
            trace!("server send msg to closed in-mem connection");
        }
    }

    // see outer type
    pub(super) fn kill(&self) {
        kill(&self.0, true, Some("connection closed by server"));
    }
}

impl InMemClient {
    /// See corresponding method on client `Connection`.
    pub fn send(&self, msg: UpMsg) {
         let alive_lock = self.shared.lock();
        if let &Some(ref alive_state) = &*alive_lock {
            alive_state.ns_shared.server_send.send(
                ServerEvent::Network(NetworkEvent::Message(alive_state.conn_idx, msg)),
                EventPriority::Network,
                None,
                None,
            );
        }
    }

    /// See corresponding method on client `Connection`.
    pub fn receiver(&self) -> &DynFlexReceiver {
        &self.recv_down
    }
}

impl Drop for InMemClient {
    fn drop(&mut self) {
        kill(&self.shared, true, None);
    }
}

// kill the in-mem connection, if not already killed. pretty much synchronization-safe.
fn kill(
    shared: &Arc<Mutex<Option<AliveState>>>,
    tell_server_dead: bool,
    tell_client_dead: Option<&str>,
) {
    let mut alive_lock = shared.lock();
    if let &Some(ref alive_state) = &*alive_lock {
        if tell_server_dead {
            destroy_conn(&alive_state.ns_shared, alive_state.conn_idx);
        }
        if let Some(tell_client_dead) = tell_client_dead {
            alive_state.send_down.send(
                Box::new(ConnectionEvent::Closed(Some(tell_client_dead.to_owned()))),
                None,
                None,
            );
        }
    }
    *alive_lock = None;
}

impl Debug for Connection {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let conn_idx = self.0.lock().as_ref().map(|alive| alive.conn_idx);
        if let Some(conn_idx) = conn_idx {
            write!(f, "conn_idx: {}", conn_idx)
        } else {
            f.write_str("dead")
        }
    }
}
