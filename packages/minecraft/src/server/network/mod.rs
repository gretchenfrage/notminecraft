//! Handle to the tokio system for handling network IO with clients.

use crate::{
    game_data::*,
    server::{
        channel::*,
        ServerEvent,
    },
    util_time::ServerRelTime,
    message::*,
};
#[cfg(feature = "client")]
use crate::client::channel::ClientSender;
use std::{
    sync::Arc,
    fmt::Debug,
    time::Instant,
};
use parking_lot::Mutex;
use slab::Slab;
use tokio::{
    runtime::Handle,
    task::AbortHandle,
};

mod send_buffer_policy_enforcer;
mod ws;
#[cfg(feature = "client")]
mod in_mem;

pub use tokio::net::ToSocketAddrs;
#[cfg(feature = "client")]
pub use self::in_mem::InMemClient;


/// Main handle to the tokio system for handling network IO with clients.
///
/// Sends network events to the server loop. Serializes changes to the space of network
/// connections. Shuts down all network tasks when dropped, although won't necessarily send
/// `RemoveConnection` events in that case, because it's assumed the whole server is dropping too.
pub struct NetworkServer(NetworkServerHandle);

/// Secondary handle to the tokio system for handling network IO with clients.
///
/// Does not keep network server alive nor shut it down when dropped--see `NetworkServer` for
/// that. Can be used to open the server to new connections.
#[derive(Clone)]
pub struct NetworkServerHandle(Arc<NetworkServerSharedState>);

// network server state shared between main handle and other tasks
struct NetworkServerSharedState {
    // sender handle to the server channel
    server_send: ServerSender,
    // lockable shared state
    lockable: Mutex<NetworkServerLockableState>,
}

// network server state guarded by the mutex that synchronizes changes to the space of connections
struct NetworkServerLockableState {
    // whether the network server has been shut down as a whole. if this happens, new connections
    // should not be created. remove connection events aren't necessary in this state either.
    shut_down: bool,
    // slab that allocates connection indices and tracks handles for shutting down all connections
    // if the network server as a whole is shut down.
    slab: Slab<SlabEntry>,
    // handles to abort tasks which accept new connections, so as to stop them if the network
    // server as a whole is shut down
    bind_abort_handles: Vec<AbortHandle>,
}

// entry within the connection slab. handles for shutting down the connection if the network server
// as a whole has been shut down.
enum SlabEntry {
    Ws(ws::SlabEntry),
    #[cfg(feature = "client")]
    InMem(in_mem::SlabEntry),
}

/// Handle to the network IO connection with a single client.
///
/// This handle is only used for transmitting data to the client. Data received from the client is
/// placed into a `NetworkEvent` so as to be serialized through the centralized server event
/// channel.
///
/// Dropping this handle does not itself close the network connection.
#[derive(Debug)]
pub struct Connection(ConnectionInner);

#[derive(Debug)]
enum ConnectionInner {
    Ws(ws::Connection),
    #[cfg(feature = "client")]
    InMem(in_mem::Connection),
}

/// Some discrete network input event happened. Goes to the conn mgr for processing.
#[derive(Debug)]
pub enum NetworkEvent {
    /// A new network connection was created and assigned a connection index in a slab pattern.
    AddConnection(usize, Connection),
    /// A message was received from a current network connection.
    ///
    /// The server channel's semaphore permit system is used to implement per-connection
    /// backpressure--the tasks for the connection will only allow a certain number of bytes worth
    /// of received messages to sit in the server channel before it stops reading from the
    /// underlying network transport until some are removed.
    Message(usize, UpMsg),
    /// A network connection has been destroyed and its connection index deallocated.
    RemoveConnection(usize),
}

impl NetworkServer {
    /// Construct. Doesn't yet bind.
    pub fn new(server_send: ServerSender) -> Self {
        NetworkServer(NetworkServerHandle(Arc::new(NetworkServerSharedState {
            server_send,
            lockable: Mutex::new(NetworkServerLockableState {
                shut_down: false,
                slab: Default::default(),
                bind_abort_handles: Default::default(),
            }),
        })))
    }

    /// Get a handle, which can be used directly to bind, or cloned for use elsewhere.
    pub fn handle(&self) -> &NetworkServerHandle {
        &self.0
    }
}

impl NetworkServerHandle {
    /// Bind to a port and open the network server to connections on that port.
    pub fn bind<B>(&self, bind_to: B, rt: &Handle, game: &Arc<GameData>)
    where
        B: ToSocketAddrs + Debug + Send + Sync + 'static,
    {
        ws::bind(&self.0, bind_to, rt, game);
    }

    /// Construct a new in-memory client. See `InMemClient`. This directly causes a single add
    /// connection network event, with the given connection object being the server-side half of
    /// this in-mem client.
    #[cfg(feature = "client")]
    pub fn in_mem_client(&self, client_send: ClientSender) -> InMemClient {
        in_mem::create(&self.0, client_send)
    }
}

impl Connection {
    /// Enqueue message to be transmitted to the client.
    ///
    /// This method itself never blocks or errors. Messages are added to an indefinitely growable
    /// queue to be transmitted to the client. If an error occurs that triggers the closure of the
    /// connection, that will become apparent to the server loop through a
    /// `NetworkEvent::RemoveConnection` event. If the connection is already closed or killed, a
    /// call to send will simply be ignored.
    pub fn send<M: Into<DownMsg>>(&self, msg: M) {
        let msg = msg.into();
        match &self.0 {
            &ConnectionInner::Ws(ref inner) => inner.send(msg),
            #[cfg(feature = "client")]
            &ConnectionInner::InMem(ref inner) => inner.send(msg),
        }
    }

    /// Our `server_t0` timestamp for this client connection.
    ///
    /// The client's `est_server_t0` should be an accurate estimate of this real time instant, but
    /// in terms of the client's own monotonic clock, which may be different from ours.
    /// Communicating real time timestamps with the client should be done by relativizing them
    /// against `server_t0`, as this does not rely on either party's clocks being accurately
    /// synchronized to Unix time.
    pub fn server_t0(&self) -> Instant {
        match &self.0 {
            &ConnectionInner::Ws(ref inner) => inner.server_t0(),
            #[cfg(feature = "client")]
            &ConnectionInner::InMem(ref inner) => inner.server_t0(),
        }
    }

    /// Relativize an instant against `server_t0`.
    ///
    /// Resultant `ServerRelTime` suitable for transmitting on this connection. Should not be
    /// called with instants before this `Connection` was created, or ridiculously far into the
    /// future.
    pub fn rel_time(&self, instant: Instant) -> ServerRelTime {
        ServerRelTime::new(instant, self.server_t0())
    }

    /// Relativize an instant against `server_t0`.
    ///
    /// Suitable for `ServerRelTime` received from this connection.
    pub fn derel_time(&self, rel_time: ServerRelTime) -> Instant {
        rel_time.to_instant(self.server_t0())
    }

    /// Kill and disconnect the network connection.
    ///
    /// This method itself never blocks or errors. This will trigger a
    /// `NetworkEvent::RemoveConnection` event to be sent to the server very soon, unless that gets
    /// triggered by some other thing which kills the network connection first. Subsequent calls to
    /// `send` will be silently ignored. An attempt will be made to gracefully close the underlying
    /// transport, but an attempt will not be actively made to continue transmission of enqueued
    /// or not fully transmitted data to the extent possible. 
    pub fn kill(&self) {
        match &self.0 {
            &ConnectionInner::Ws(ref inner) => inner.kill(),
            #[cfg(feature = "client")]
            &ConnectionInner::InMem(ref inner) => inner.kill(),
        }
    }
}

impl Drop for NetworkServer {
    fn drop(&mut self) {
        // shut down everything upon the main handle being dropped
        let mut lock = self.0.0.lockable.lock();
        for abort_handle in &lock.bind_abort_handles {
            abort_handle.abort();
        }
        lock.shut_down = true;
        for (_, entry) in &lock.slab {
            match entry {
                &SlabEntry::Ws(ref inner) => inner.shutdown(),
                #[cfg(feature = "client")]
                &SlabEntry::InMem(ref inner) => inner.shutdown(),
            }
        }
        drop(lock);
    }
}

// allocate a connection idx and send an add connection network event in a synchronized way.
// returns none if the whole network server is shutting down, in which case the caller should abort
// and clean itself up.
fn create_conn(
    shared: &NetworkServerSharedState,
    slab_entry: SlabEntry,
    connection: Connection,
) -> Option<usize> {
    let mut lock = shared.lockable.lock();
    if lock.shut_down {
        return None;
    }
    let conn_idx = lock.slab.insert(slab_entry);
    shared.server_send.send(
        ServerEvent::Network(NetworkEvent::AddConnection(conn_idx, connection)),
        EventPriority::Network,
        None,
        None,
    );
    drop(lock);
    Some(conn_idx)
}

// deallocate a connection idx and send a remove connection network event in a synchronized way.
fn destroy_conn(shared: &NetworkServerSharedState, conn_idx: usize) {
    let mut lock = shared.lockable.lock();
    if lock.shut_down {
        // save work, make shutdown faster
        return;
    }
    lock.slab.remove(conn_idx);
    shared.server_send.send(
        ServerEvent::Network(NetworkEvent::RemoveConnection(conn_idx)),
        EventPriority::Network,
        None,
        None,
    );
}
