//! Network connection implementation for in-memory transport between client and internal server.

use super::*;
use crate::{
    server::ServerEvent,
    client::{
        channel::{
            ClientSender,
            EventPriority as ClientEventPriority,
        },
        network::NetworkEvent as ClientNetworkEvent,
        ClientEvent,
    },
};
use std::{
    sync::Arc,
    fmt::{self, Formatter, Debug},
    time::Instant,
    write,
};
use parking_lot::Mutex;

// TODO: get send buffer policy enforcement working again

// slab entry inner type for in-mem connections
pub(super) struct SlabEntry(Arc<Mutex<Option<AliveState>>>);

// connection inner type for in-mem connections
pub(super) struct Connection {
    alive: Arc<Mutex<Option<AliveState>>>,
    // instant to relativize timestamps to, for consistency with when networked
    server_t0: Instant,
}

/// Client connection to the network server that just sends messages over in-memory queues within
/// the same process, avoiding both network and serialization costs.
pub struct InMemClient {
    // shared state
    shared: Arc<Mutex<Option<AliveState>>>,
    // instant to relativize timestamps to, for consistency with when networked
    server_t0: Instant,
}

// shared lockable state that's kept iff the in-mem connection is alive
struct AliveState {
    // network server shared state
    ns_shared: Arc<NetworkServerSharedState>,
    // connection index
    conn_idx: usize,
    // sender for down-going client network event
    client_send: ClientSender,
}

// create an in-mem client for the server
pub(super) fn create(
    ns_shared: &Arc<NetworkServerSharedState>,
    client_send: ClientSender,
) -> InMemClient {
    let server_t0 = Instant::now();

    let shared = Arc::new(Mutex::new(None));
    let slab_entry = super::SlabEntry::InMem(SlabEntry(Arc::clone(&shared)));
    let connection = super::Connection(ConnectionInner::InMem(Connection {
        alive: Arc::clone(&shared),
        server_t0,
    }));
    let conn_idx = create_conn(ns_shared, slab_entry, connection);

    if let Some(conn_idx) = conn_idx {
        // normal case
        let mut shared_lock = shared.lock();
        *shared_lock = Some(AliveState {
            ns_shared: Arc::clone(ns_shared),
            conn_idx,
            client_send,
        });
        drop(shared_lock);
    } else {
        // this case happens if the whole network server has been dropped
        client_send.send(
            ClientEvent::Network(ClientNetworkEvent::Closed(Some("server closed".to_owned()))),
            ClientEventPriority::Network,
            None,
            None,
        );
    }

    InMemClient { shared, server_t0 }
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
        let alive_lock = self.alive.lock();
        if let &Some(ref alive_state) = &*alive_lock {
            alive_state.client_send.send(
                ClientEvent::Network(ClientNetworkEvent::Received(msg)),
                ClientEventPriority::Network,
                None,
                None,
            );
        } else {
            trace!("server send msg to closed in-mem connection");
        }
    }

    // see outer type
    pub(super) fn server_t0(&self) -> Instant {
        self.server_t0
    }

    // see outer type
    pub(super) fn kill(&self) {
        kill(&self.alive, true, Some("connection closed by server"));
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
    pub fn est_server_t0(&self) -> Instant {
        self.server_t0
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
        if let Some(close_frame) = tell_client_dead {
            alive_state.client_send.send(
                ClientEvent::Network(ClientNetworkEvent::Closed(Some(close_frame.to_owned()))),
                ClientEventPriority::Network,
                None,
                None,
            );
        }
    }
    *alive_lock = None;
}

impl Debug for Connection {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let conn_idx = self.alive.lock().as_ref().map(|alive| alive.conn_idx);
        if let Some(conn_idx) = conn_idx {
            write!(f, "conn_idx: {}", conn_idx)?;
        } else {
            f.write_str("dead")?;
        }
        write!(f, " server_t0: {:?}", self.server_t0)
    }
}
