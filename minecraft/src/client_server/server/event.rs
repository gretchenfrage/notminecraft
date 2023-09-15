
use crossbeam_channel::{
    Sender,
    Receiver,
    TryRecvError,
    RecvTimeoutError,
    unbounded,
};
use crate::client_server::server::{
    connection::NetworkEvent,
    chunk_loader::ReadyChunk,
};
use std::time::Instant;


macro_rules! server_events {
    ($( $variant:ident($inner:ty) $send:ident $recv:ident $try_recv:ident $recv_deadline:ident, )*)=>{
        /// Event that happened outside of the server primary thread that then
        /// gets sent to the server primary thread asynchronously for it to
        /// process.
        #[derive(Debug)]
        pub enum ServerEvent {$(
            $variant($inner),
        )*}

        $(
            impl From<$inner> for ServerEvent {
                fn from(inner: $inner) -> Self {
                    ServerEvent::$variant(inner)
                }
            }
        )*

        /// Concurrent queues of `ServerEvent` that prioritizes it by variant
        /// and is capable of polling for only one variant.
        #[derive(Debug, Clone)]
        pub struct ServerEventQueue {
            // always send a token after sending an event
            // sometimes take a token before taking an event
            // thus message will not become available on any without token
            // appearing first, but presence of token does not guarantee
            // message available
            send_token: Sender<()>,
            recv_token: Receiver<()>,
            
            $(
                $send: Sender<$inner>,
                $recv: Receiver<$inner>,
            )*
        }

        impl ServerEventQueue {
            pub fn new() -> Self {
                let (send_token, recv_token) = unbounded();
                $(
                    let ($send, $recv) = unbounded();
                )*
                ServerEventQueue {
                    send_token,
                    recv_token,
                    $(
                        $send,
                        $recv,
                    )*
                }
            }

            pub fn send(&self, event: impl Into<ServerEvent>) {
                match event.into() {$(
                    ServerEvent::$variant(inner) => self.$send.send(inner).unwrap(),
                )*}
                self.send_token.send(()).unwrap();
            }

            fn inner_try_recv(&self) -> Option<ServerEvent> {
                $(
                    if let Ok(inner) = self.$recv
                        .try_recv()
                        .map_err(|e| debug_assert!(matches!(e, TryRecvError::Empty)))
                    {
                        return Some(ServerEvent::$variant(inner));
                    }
                )*
                None
            }

            pub fn try_recv(&self) -> Option<ServerEvent> {
                loop {
                    if self.recv_token.try_recv()
                        .map_err(|e| debug_assert!(matches!(e, TryRecvError::Empty)))
                        .is_ok()
                    {
                        if let Some(event) = self.inner_try_recv() {
                            return Some(event);
                        }
                    } else {
                        return None;
                    }
                }
            }

            pub fn recv_deadline(&self, deadline: Instant) -> Option<ServerEvent> {
                loop {
                    if self.recv_token.recv_deadline(deadline)
                        .map_err(|e| debug_assert!(matches!(e, RecvTimeoutError::Timeout)))
                        .is_ok()
                    {
                        if let Some(event) = self.inner_try_recv() {
                            return Some(event);
                        }
                    } else {
                        return None;
                    }
                }
            }

            $(
                pub fn $try_recv(&self) -> Option<$inner> {
                    self.$recv.try_recv()
                        .map_err(|e| debug_assert!(matches!(e, TryRecvError::Empty)))
                        .ok()
                }

                pub fn $recv_deadline(&self, deadline: Instant) -> Option<$inner> {
                    self.$recv.recv_deadline(deadline)
                        .map_err(|e| debug_assert!(matches!(e, RecvTimeoutError::Timeout)))
                        .ok()
                }
            )*
        }
    };
}

server_events!(
    Network(NetworkEvent) send_network recv_network try_recv_network recv_network_deadline,
    ChunkReady(ReadyChunk) send_chunk recv_chunk try_recv_chunk recv_chunk_deadline,
);
