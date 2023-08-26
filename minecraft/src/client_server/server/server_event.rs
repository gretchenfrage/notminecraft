
use super::connection::ConnectionEvent;


/// Thing happening from externally asynchronously to the server thread.
#[derive(Debug)]
pub enum ServerEvent {
    Connection(ConnectionEvent),
}

