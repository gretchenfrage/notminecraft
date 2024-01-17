//! Handle to the network IO connection to the server.

mod ws;

use crate::{
    message::*,
    game_data::GameData,
    dyn_flex_channel::DynFlexReceiver,
};
use std::sync::Arc;
use tokio::runtime::Handle;


/// Handle to the network IO connection to the server.
///
/// Closes the connection when dropped. If dropped, a `ConnectionEvent::Closed` may not be
/// delivered, because it's assumed that the client is intentionally dropping the entire
/// connection-related state.
pub struct Connection(ConnectionInner);

enum ConnectionInner {
    Ws(ws::Connection),
}

/// Message received from `Connection.receiver()`.
pub enum ConnectionEvent {
    /// Message received from connection.
    Received(DownMsg),
    /// Connection closed. No further connection events will occur after this. May contain message
    /// suitable for displaying to user.
    Closed(Option<String>),
}

impl Connection {
    /// Connect to a server at the given url.
    ///
    /// Returns immediately without blocking or erroring, spawning as task to initialize the
    /// connection in the background. If that initialization fails, will simply appear as the
    /// connection closing.
    pub fn connect(url: &str, rt: &Handle, game: &Arc<GameData>) -> Self {
        Connection(ConnectionInner::Ws(ws::Connection::connect(url, rt, game)))
    }

    /// Enqueue a message to be transmitted to the server.
    pub fn send<M: Into<UpMsg>>(&self, msg: M) {
        match &self.0 {
            &ConnectionInner::Ws(ref inner) => inner.send(msg.into()),
        }
    }

    /// Get the dyn flex receiver. Will produce `ConnectionEvent`.
    pub fn receiver(&self) -> &DynFlexReceiver {
        match &self.0 {
            &ConnectionInner::Ws(ref inner) => inner.receiver(),
        }
    }
}
