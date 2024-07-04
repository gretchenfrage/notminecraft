//! Handle to the network IO connection to the server.

mod ws;

use crate::{
    message::*,
    game_data::GameData,
    server::network::InMemClient,
    client::channel::{
        ClientSender,
        EventPriority,
    },
    util_time::ServerRelTime,
};
use std::{
    sync::Arc,
    cell::Cell,
    fmt::{self, Formatter, Debug},
    time::Instant,
};
use tokio::runtime::Handle;
use anyhow::Error;


/// Handle to the network IO connection to the server.
///
/// Closes the connection when dropped. If dropped, a `ConnectionEvent::Closed` may not be
/// delivered, because it's assumed that the client is intentionally dropping the entire
/// connection-related state.
pub struct Connection {
    inner: ConnectionInner,
    last_up_msg_idx: Cell<u64>,
}

enum ConnectionInner {
    Ws(ws::Connection),
    InMem(InMemClient),
}

/// Network event for client.
#[derive(Debug)]
pub enum NetworkEvent {
    /// Message received from connection.
    Received(DownMsg),
    /// Connection closed. No further connection events will occur after this. May contain message
    /// suitable for displaying to user.
    Closed(Option<String>),
}

impl Connection {
    /// Establish a connection to a server at the given url.
    pub async fn connect(
        url: &str,
        client_send: ClientSender,
        rt: &Handle,
        game: &Arc<GameData>,
    ) -> Result<Self, Error> {
        let inner = ws::Connection::connect(url, client_send, rt, game).await?;
        Ok(Connection {
            inner: ConnectionInner::Ws(inner),
            last_up_msg_idx: Cell::new(0),
        })
    }

    /// Wrap around a server in-mem client.
    pub fn in_mem(inner: InMemClient) -> Self {
        Connection {
            inner: ConnectionInner::InMem(inner),
            last_up_msg_idx: Cell::new(0),
        }
    }

    /// Enqueue a message to be transmitted to the server.
    ///
    /// Return the up msg index of the message that was just sent. They start at 1.
    pub fn send<M: Into<UpMsg>>(&self, msg: M) -> u64 {
        self.last_up_msg_idx.set(self.last_up_msg_idx.get() + 1);
        match &self.inner {
            &ConnectionInner::Ws(ref inner) => inner.send(msg.into()),
            &ConnectionInner::InMem(ref inner) => inner.send(msg.into()),
        }
        self.last_up_msg_idx.get()
    }

    /// Estimated instant at which the server sampled its `server_t0` timestamp.
    ///
    /// Communicating real time timestamps with the server should be done by relativizing them
    /// against `est_server_t0`, as this does not rely on either party's clocks being accurately
    /// synchronized to Unix time.
    pub fn est_server_t0(&self) -> Instant {
        match &self.inner {
            &ConnectionInner::Ws(ref inner) => inner.est_server_t0(),
            &ConnectionInner::InMem(ref inner) => inner.est_server_t0(),
        }
    }

    /// Relativize an instant against `est_server_t0`.
    ///
    /// Resultant `ServerRelTime` suitable for transmitting on this connection.
    ///
    /// Warns and saturates if called with instants ridiculously far before or after
    /// `est_server_t0`.
    pub fn rel_time(&self, instant: Instant) -> ServerRelTime {
        ServerRelTime::new(instant, self.est_server_t0())
    }

    /// Derelativize an instant against `est_server_t0`.
    ///
    /// Suitable for `ServerRelTime` received from this connection.
    pub fn derel_time(&self, rel_time: ServerRelTime) -> Instant {
        rel_time.to_instant(self.est_server_t0())
    }
}

impl From<InMemClient> for Connection {
    fn from(inner: InMemClient) -> Self {
        Self::in_mem(inner)
    }
}

impl Debug for Connection {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Connection { .. }")
    }
}
