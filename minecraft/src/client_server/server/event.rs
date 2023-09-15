
use crossbeam_channel::{
    Sender,
    Receiver,
    TryRecvError,
    RecvTimeoutError,
    unbounded,
};
use crate::client_server::server::{
    connection::NetworkEvent,
    chunk_loader::LoadChunkEvent,
};
use std::time::Instant;


macro_rules! server_events {
    ($( $variant:ident($inner:ty) $send:ident $recv:ident $sender:ident $recv_now:ident $recv_by:ident, )*)=>{
        /// Events sent to the server thread for it to process asynchronously.
        #[derive(Debug)]
        pub enum Event {$(
            $variant($inner),
        )*}

        $(
            impl From<$inner> for Event {
                fn from(inner: $inner) -> Self {
                    Event::$variant(inner)
                }
            }
        )*

        pub fn event_channel() -> (EventSenders, EventReceiver) {
            let (send_token, recv_token) = unbounded();
            $(
                let ($send, $recv) = unbounded();
            )*
            (
                EventSenders {
                    send_token,
                    $(
                        $send,
                    )*
                },
                EventReceiver {
                    recv_token,
                    $(
                        $recv,
                    )*
                },
            )
        }

        #[derive(Debug, Clone)]
        pub struct EventSenders {
            send_token: Sender<()>,
            $(
                $send: Sender<$inner>,
            )*
        }

        #[derive(Debug)]
        pub struct EventSender<T> {
            send_token: Sender<()>,
            send: Sender<T>,
        }


        #[derive(Debug, Clone)]
        pub struct EventReceiver {
            // always send a token after sending an event
            // sometimes take a token before taking an event
            recv_token: Receiver<()>,
            $(
                $recv: Receiver<$inner>,
            )*
        }

        impl EventSenders {
            $(
                pub fn $sender(&self) -> EventSender<$inner> {
                    EventSender {
                        send_token: self.send_token.clone(),
                        send: self.$send.clone(),
                    }
                }
            )*
        }

        impl<T> EventSender<T> {
            pub fn send(&self, event: T) {
                let _ = self.send.send(event);
                let _ = self.send_token.send(());
            }
        }

        impl<T> Clone for EventSender<T> {
            fn clone(&self) -> Self {
                EventSender {
                    send_token: self.send_token.clone(),
                    send: self.send.clone(),
                }
            }
        }

        impl EventReceiver {
            fn sweep(&self) -> Option<Event> {
                $(
                    if let Ok(inner) = self.$recv
                        .try_recv()
                        .map_err(|e| debug_assert!(matches!(e, TryRecvError::Empty)))
                    {
                        return Some(Event::$variant(inner));
                    }
                )*
                None
            }

            pub fn recv_any_now(&self) -> Option<Event> {
                loop {
                    if self.recv_token.try_recv()
                        .map_err(|e| debug_assert!(matches!(e, TryRecvError::Empty)))
                        .is_ok()
                    {
                        if let Some(event) = self.sweep() {
                            return Some(event);
                        }
                    } else {
                        return None;
                    }
                }
            }

            pub fn recv_any_by(&self, deadline: Instant) -> Option<Event> {
                loop {
                    if Instant::now() > deadline {
                        return None;
                    }
                    if self.recv_token.recv_deadline(deadline)
                        .map_err(|e| debug_assert!(matches!(e, RecvTimeoutError::Timeout)))
                        .is_ok()
                    {
                        if let Some(event) = self.sweep() {
                            return Some(event);
                        }
                    } else {
                        return None;
                    }
                }
            }

            $(
                pub fn $recv_now(&self) -> Option<$inner> {
                    self.$recv.try_recv()
                        .map_err(|e| debug_assert!(matches!(e, TryRecvError::Empty)))
                        .ok()
                }

                pub fn $recv_by(&self, deadline: Instant) -> Option<$inner> {
                    if Instant::now() > deadline {
                        return None;
                    }
                    self.$recv.recv_deadline(deadline)
                        .map_err(|e| debug_assert!(matches!(e, RecvTimeoutError::Timeout)))
                        .ok()
                }
            )*
        }
    };
}

// the order in which these appear will be the priority in which they're dispensed
server_events!(
    Network(NetworkEvent) send_network recv_network network_sender recv_network_now recv_network_by,
    LoadChunk(LoadChunkEvent) send_chunk recv_chunk chunk_sender recv_chunk_now recv_chunk_by,
);
