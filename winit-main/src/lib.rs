//! This is a [`winit`](https://lib.rs/crates/winit) utility which abstracts away
//! winit's event-loop inversion of control.
//! 
//! ## Rationale
//! 
//! Winit necessarily hijacks the main thread due to platform constraints,
//! creating a "don't call us, we'll call you" situation. Inversions of control
//! have some undesirable properties, including:
//! 
//! - It's difficult to add inversion of control to a program after the fact, as
//!   it tends to fundamentally affect the program's architecture.
//! - For the above reason, it's difficult to write programs which are generic
//!   between inversions-of-control, or to modify a program from using one
//!   inversion-of-control framework to using a different framework.
//! - It's tricky to use several inversions of control simultaneously. For example,
//!   it would be difficult to combine tokio with winit without creating additional
//!   abstractions.
//! 
//! ## Solution
//! 
//! This library spawns your code on a second thread (a "simulated main thread"),
//! hijacks the real main thread with winit's event loop, and provides your code
//! handles to communicate with the main event loop. This allows you to write your
//! program as you would any other program, treating winit's event loop as an
//! iterator of events and a handle with which to create windows and ask about the
//! system. When the simulated main thread exits, it triggers the event loop to
//! exit, shutting down the process, just like if it were the real main thread.
//! 
//! ## Handling of Control Flow
//! 
//! ### Blockers
//! 
//! The simulated main thread receives winit `Event`s through an `EventReceiver`.
//! In these events, the user event type is a `Blocker`. This is a concurrency
//! structure emitted by the main thread which blocks the event loop from
//! processing further winit events until the `Blocker` is dropped. This is a way
//! to synchronize the event loop with the simulated main thread to some extent,
//! such as to synchronize the presenting of images.
//! 
//! Whenever the event loop encounters a `RedrawRequested` event, it immediately
//! emits a `Blocker`, and thus will not proceed until the simulated main thread
//! receives and drops that `Blocker`.
//! 
//! ### `ControlFlow`
//! 
//! This library keeps the winit event loop in the `ControlFlow::Wait` state.
//! Therefore, if you want to redraw a window in a loop, you should call
//! `Window::request_redraw` after every draw.
//! 
//! ## Example
//! 
//! ### Without `winit-main`:
//! 
//! ```rust,no_run
//! use winit::{
//!     event::{Event, WindowEvent},
//!     event_loop::{ControlFlow, EventLoop},
//!     window::WindowBuilder,
//! };
//! 
//! fn main() {
//!     let event_loop = EventLoop::new();
//!     let window = WindowBuilder::new().build(&event_loop).unwrap();
//! 
//!     event_loop.run(move |event, _, control_flow| {
//!         *control_flow = ControlFlow::Wait;
//! 
//!         if matches!(
//!             event,
//!             Event::WindowEvent {
//!                 event: WindowEvent::CloseRequested,
//!                 window_id,
//!             } if window_id == window.id()
//!         ) {
//!             *control_flow = ControlFlow::Exit;
//!         }
//!     });
//! }
//! ```
//! 
//! ### With `winit-main`:
//!
//! ```rust,no_run
//! use winit_main::reexports::{
//!     event::{Event, WindowEvent},
//!     window::WindowAttributes,
//! };
//! 
//! fn main() {
//!     winit_main::run(|event_loop, events| {
//!         let window = event_loop
//!             .create_window(WindowAttributes::default())
//!             .unwrap();
//! 
//!         for event in events.iter() {
//!             if matches!(
//!                 event,
//!                 Event::WindowEvent {
//!                     event: WindowEvent::CloseRequested,
//!                     window_id,
//!                 } if window_id == window.id()
//!             ) {
//!                 break;
//!             }
//!         }
//!     });
//! }
//! ```

use std::{
    thread,
    time::Duration,
    iter,
    panic::{
        catch_unwind,
        AssertUnwindSafe,
    },
    sync::mpsc as std_mpsc,
    future::Future,
};
use winit::{
    event_loop::{
        EventLoop,
        EventLoopProxy,
        ControlFlow,
    },
    event::Event,
    monitor::MonitorHandle,
    window::{
        WindowAttributes,
        Window,
    },
    error::OsError,
};
use crate::request::{
    Request,
    RequestMessage,
    RequestCallback,
    GetAvailableMonitors,
    GetPrimaryMonitor,
    CreateWindow,
};
use tokio::{
    sync::{
        oneshot,
        mpsc as tokio_mpsc,
    },
    runtime::Runtime,
};


mod request;


/// Re-exports of `winit` modules.
///
/// Re-exports all `winit` modules except `winit::event_loop`.
pub mod reexports {
    // re-export everthing except `event_loop`
    pub use winit::{
        dpi,
        error,
        event,
        monitor,
        platform,
        window,
    };
}

/// Message sent from the simulated main thread to the event loop.
enum Message {
    /// Request for some function to be evaluated in the context of the event
    /// loop and the response sent back to the sender. Sent by
    /// `EventLoopHandle`.
    Request(RequestMessage),
    /// Request for the event loop, and therefore the entire process, to exit.
    /// Sent when the simulated main thread's user function exits.
    Exit,
    /// Unblock the event loop from its currently blocked state. Sent to the
    /// event loop once, no more and no less, after and only after the event
    /// loop sends out a `Blocked` user event. 
    Unblock,
}


/// Handle for sending requests to the main event loop and receiving responses.
#[derive(Clone)]
pub struct EventLoopHandle {
    // use this to wake the event loop up and trigger it to process messages
    wake_sender: EventLoopProxy<()>,
    // use this to actually send the message
    msg_send: std_mpsc::Sender<Message>,
}

fn sleep_forever() -> ! {
    loop {
        thread::sleep(Duration::new(u64::MAX, 1_000_000_000 - 1));
    }
}

impl EventLoopHandle {
    /// Send a request, wait for a response.
    async fn request_wait<R>(&self, request: R) -> R::Response
    where
        R: Request,
        RequestMessage: From<RequestCallback<R>>,
    {
        // pair the request with a channel for the response to return on
        let (send_response, recv_response) = oneshot::channel();
        let request = RequestMessage::from(RequestCallback {
            request,
            callback: send_response,
        });

        // send the request
        let _ = self.msg_send.send(Message::Request(request));
        // trigger the event loop to wake up and process the request
        let _ = self.wake_sender.send_event(());

        // wait for the response
        match recv_response.await {
            Ok(response) => response,
            Err(_) => sleep_forever(),
        }
    }

    /// The list of all monitors available on the system. 
    ///
    /// Equivalent to
    /// `winit::event_loop::EventLoopWindowTarget::available_monitors`.
    pub async fn available_monitors(&self) -> Vec<MonitorHandle> {
        self.request_wait(GetAvailableMonitors).await
    }

    /// The primary monitor of the system.
    /// 
    /// Equivalent to
    /// `winit::event_loop::EventLoopWindowTarget::primary_monitor`.
    pub async fn primary_monitor(&self) -> Option<MonitorHandle> {
        self.request_wait(GetPrimaryMonitor).await
    }

    /// Attempt to create a new window.
    ///
    /// Equivalent to `winit::window::WindowBuilder::build`.
    pub async fn create_window(&self, attributes: WindowAttributes) -> Result<Window, OsError> {
        self.request_wait(CreateWindow(attributes)).await
    }
}

/// Concurrency structure, emitted as a user event immediately after certain 
/// other events are emitted, which blocks the event loop until this `Blocker`
/// is dropped.
pub struct Blocker(std_mpsc::Sender<Message>);

impl Drop for Blocker {
    fn drop(&mut self) {
        let _ = self.0.send(Message::Unblock);
    }
}

impl Blocker {
    /// Unblock the event loop. This is only to facilitate readability, since 
    /// `Blocker` unblocks the event loop when dropped.
    pub fn unblock(self) {
        drop(self)
    }
}


/// Handle for receiving events from the main event loop.
///
/// Unlike a raw `std::sync::mpsc::Receiver`, this never returns error on
/// disconnection, because disconnection can only occur for a brief moment
/// between the main event loop beginning to shut down, and the process as a 
/// whole exiting. Therefore, when this receives a disconnection error from
/// the underlying receiver, it enters an infinite sleep cycle as it waits for
/// the OS to kill the process. TODO update all comments
pub struct EventReceiver(tokio_mpsc::UnboundedReceiver<Event<'static, Blocker>>);

impl EventReceiver {
    /// Receive an event, blocking until one is available. 
    pub async fn recv(&mut self) -> Event<'static, Blocker> {
        match self.0.recv().await {
            Some(event) => event,
            None => sleep_forever(),
        }
    }

    /// Try to receive an event immediately, never blocking.
    pub fn try_recv(&mut self) -> Option<Event<'static, Blocker>> {
        match self.0.try_recv() {
            Ok(event) => Some(event),
            Err(tokio_mpsc::error::TryRecvError::Empty) => None,
            Err(tokio_mpsc::error::TryRecvError::Disconnected) => sleep_forever(),
        }
    }

    /// Iterator form of `self.try_recv()`. Non-blocking iterator that drains
    /// the events currently in the queue. 
    pub fn try_iter<'a>(&'a mut self) -> impl Iterator<Item=Event<'static, Blocker>> + 'a {
        iter::from_fn(move || self.try_recv())
    }
}


/// Hijack the main thread with a winit event loop, and spawn a new thread with
/// callbacks to communicate with the main thread.
/// 
/// When the new thread, the "simulated main thread" exits, the event loop will
/// also exit loop. This is this is the primary abstraction of this crate, as
/// it abstracts away `winit`'s inversion of control, and allows `winit` to be
/// used more like any other library.
pub fn run<F, Fut>(f: F) -> !
where
    F: FnOnce(EventLoopHandle, EventReceiver) -> Fut + Send + 'static,
    Fut: Future<Output=()>,
{
    // create event loop
    let event_loop = EventLoop::with_user_event();

    // create queues
    let (event_send, event_recv) = tokio_mpsc::unbounded_channel();
    let (msg_send, msg_recv) = std_mpsc::channel();
    let msg_send_1 = msg_send;
    let msg_send_2 = msg_send_1.clone();
    let msg_send_3 = msg_send_1.clone();
    let wake_sender_1 = event_loop.create_proxy();
    let wake_sender_2 = event_loop.create_proxy();

    // spawn simulated main thread    
    thread::spawn(move || {
        let handle = EventLoopHandle {
            wake_sender: wake_sender_1,
            msg_send: msg_send_1,
        };
        let receiver = EventReceiver(event_recv);

        // run the user code
        let _ = catch_unwind(AssertUnwindSafe(move || {
            Runtime::new()
                .expect("failure to create tokio runtime")
                .block_on(f(handle, receiver));
        }));

        // send the exit message to the event loop
        let _ = msg_send_2.send(Message::Exit);
        // wake up the event loop
        let _ = wake_sender_2.send_event(());
    });

    // enter event loop
    event_loop.run(move |event, window_target, control_flow| {
        *control_flow = ControlFlow::Wait;

        let event = match event.to_static() {
            Some(event) => event,
            None => return, // TODO: what if user wants the static event?
        };

        match event.map_nonuser_event() {
            Ok(nonuser_event) => {
                // send out event
                let triggers_block = matches!(
                    &nonuser_event,
                    &Event::RedrawRequested(_)
                );
                
                let _ = event_send.send(nonuser_event);

                if triggers_block {
                    // maybe send out a blocker, then block on it
                    let blocker = Blocker(msg_send_3.clone());
                    let _ = event_send.send(Event::UserEvent(blocker));

                    // we must still process messages while blocked blocked, or
                    // it would likely cause deadlock
                    'block: for msg in msg_recv.iter() {
                        match msg {
                            Message::Request(request) => {
                                request.run_respond(window_target);
                            }
                            Message::Unblock => {
                                break 'block;
                            },
                            Message::Exit => {
                                *control_flow = ControlFlow::Exit;
                            }
                        };
                    }
                }
            }
            Err(Event::UserEvent(())) => {
                // process messages
                // the user event is sent to wake us up and trigger us to 
                // process messages after a message is sent
                for msg in msg_recv.try_iter() {
                    match msg {
                        Message::Request(request) => {
                            request.run_respond(window_target);
                        }
                        Message::Unblock => unreachable!("not blocked"),
                        Message::Exit => {
                            *control_flow = ControlFlow::Exit;
                        }
                    };
                }
            }
            Err(_) => unreachable!(),
        };
    });
}
