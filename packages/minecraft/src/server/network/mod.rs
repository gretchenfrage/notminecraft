//! Server side handling of network connections with clients.


/// Server's top-level handle to its system of tasks that maintain asynchronous network connections
/// with clients.
///
/// Constructed around a `ServerSender`, and sends events to the server.
pub struct NetworkServer {
    slab: Arc<Mutex<Slab<()>>>,
}




#[derive(Clone)]
struct ConnectionOpener {
    slab: Arc<Mutex<Slab
}

impl NetworkServer {
    pub fn new() -> Self {
        
    }
}
