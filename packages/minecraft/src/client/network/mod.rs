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
};
use std::{
    sync::Arc,
    cell::Cell,
};
use tokio::runtime::Handle;


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
    /// Connect to a server at the given url.
    ///
    /// Returns immediately without blocking or erroring, spawning a task to initialize the
    /// connection in the background. If that initialization fails, will simply appear as the
    /// connection closing.
    pub fn connect(
        url: &str,
        client_send: ClientSender,
        rt: &Handle,
        game: &Arc<GameData>,
    ) -> Self {
        Connection {
            inner: ConnectionInner::Ws(ws::Connection::connect(url, client_send, rt, game)),
            last_up_msg_idx: Cell::new(0),
        }
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
}

impl From<InMemClient> for Connection {
    fn from(inner: InMemClient) -> Self {
        Self::in_mem(inner)
    }
}
