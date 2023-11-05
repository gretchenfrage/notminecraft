/// Server events.

pub mod control;


use self::control::ControlEvent;
use crossbeam_channel::{
    Sender,
    Receiver,
    TryRecvError,
    RecvTimeoutError,
    unbounded,
};
use crate::server::{
    connection::NetworkEvent,
    chunk_loader::LoadChunkEvent,
};
use std::time::Instant;


macro_rules! server_events {
    ($( $variant:ident($inner:ty) $send:ident $recv:ident $sender:ident $recv_now:ident $recv_by:ident, )*)=>{
        /// Events sent to the server thread.
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

        /// Server event channel. Has some domain-specific prioritization
        /// abilities. Notable is that event receipt may be reoredered to
        /// deliver events of lower `Event` variants before higher ones. Also,
        /// one can poll for specific variants of events, leaving others
        /// remaining in the channel.
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

        /// Generalized sender of server events.
        #[derive(Debug, Clone)]
        pub struct EventSenders {
            send_token: Sender<()>,
            $(
                $send: Sender<$inner>,
            )*
        }

        /// Sender of a specific type of server events.
        #[derive(Debug)]
        pub struct EventSender<T> {
            send_token: Sender<()>,
            send: Sender<T>,
        }

        /// Receiver of server events.
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
                /// Get a sender for this specific type of event.
                pub fn $sender(&self) -> EventSender<$inner> {
                    EventSender {
                        send_token: self.send_token.clone(),
                        send: self.$send.clone(),
                    }
                }
            )*
        }

        impl<T> EventSender<T> {
            /// Send the event.
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

            /// Receive any event if one is available immediately.
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

            /// Receive any event if one is available by the deadline, blocking
            /// up to the deadline if none are available earlier. Unlike
            /// underlying channels, will return None if the deadline has
            /// passed even if an event is available already without blocking.
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
                /// Receive this type of event if one is available immediately.
                pub fn $recv_now(&self) -> Option<$inner> {
                    self.$recv.try_recv()
                        .map_err(|e| debug_assert!(matches!(e, TryRecvError::Empty)))
                        .ok()
                }

                /// Receive this type of event if one is available by the
                /// deadline, blocking up to the deadline if none are available
                /// earlier. Unlike underlying channels, will return None if
                /// the deadline has passed even if an event is available
                /// already without blocking.
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
    Control(ControlEvent) send_control recv_control control_sender recv_control_now recv_control_by,
    Network(NetworkEvent) send_network recv_network network_sender recv_network_now recv_network_by,
    LoadChunk(LoadChunkEvent) send_chunk recv_chunk chunk_sender recv_chunk_now recv_chunk_by,
);
