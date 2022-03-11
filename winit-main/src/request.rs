
use std::sync::mpsc;
use winit::{
    event_loop::EventLoopWindowTarget,
    monitor::MonitorHandle,
    window::{
        WindowBuilder,
        WindowAttributes,
        Window,
    },
    error::OsError,
};


/// Request for some function to be evaluated in the context of the event loop
/// and the response to be sent back to the sender.
pub trait Request: Sized + Send {
    type Response: Send;

    /// Evaluate the request, create side effects, return the response. 
    fn run(self, window_target: &EventLoopWindowTarget<()>) -> Self::Response;
}


/// A `Request` and an `mpsc::Sender` for sending its `Response` back to the
/// simulated main thread.
pub struct RequestCallback<R: Request> {
    pub request: R,
    pub callback: mpsc::Sender<R::Response>,
}

impl<R: Request> RequestCallback<R> {
    /// Evaluate the request, create side effects, and send the response back
    /// to the simulated main thread.
    pub fn run_respond(self, window_target: &EventLoopWindowTarget<()>) {
        let _ = self.callback.send(self.request.run(window_target));
    }
}


/// Request for `EventLoopWindowTaret::available_monitors`.
pub struct GetAvailableMonitors;

impl Request for GetAvailableMonitors {
    type Response = Vec<MonitorHandle>;

    fn run(self, window_target: &EventLoopWindowTarget<()>) -> Self::Response {
        window_target.available_monitors().collect()
    }
}


/// Request for `EventLoopEventLoopWindowTarget<()>::primary_monitor`.
pub struct GetPrimaryMonitor;

impl Request for GetPrimaryMonitor {
    type Response = Option<MonitorHandle>;

    fn run(self, window_target: &EventLoopWindowTarget<()>) -> Self::Response {
        window_target.primary_monitor()
    }
}


/// Request for `WindowBuilder::build`.
pub struct CreateWindow(pub WindowAttributes);

impl Request for CreateWindow {
    type Response = Result<Window, OsError>;

    fn run(self, window_target: &EventLoopWindowTarget<()>) -> Self::Response {
        let CreateWindow(attributes) = self;
        let mut builder = WindowBuilder::new();
        builder.window = attributes;
        builder.build(window_target)
    }
}


/// We use this to create `RequestMessage`, an enum over various
/// `RequestCallback<_>`.
macro_rules! request_message {
    ($($request:ident),*$(,)?)=>{
        pub enum RequestMessage {
            $( $request(RequestCallback<$request>), )*
        }

        impl RequestMessage {
            pub fn run_respond(self, window_target: &EventLoopWindowTarget<()>) {
                match self {
                    $(
                        RequestMessage::$request(inner) => {
                            inner.run_respond(window_target);
                        }
                    )*
                }
            }
        }

        $(
            impl From<RequestCallback<$request>> for RequestMessage {
                fn from(request_callback: RequestCallback<$request>) -> Self {
                    RequestMessage::$request(request_callback)
                }
            }
        )*
    };
}

request_message! {
    GetAvailableMonitors,
    GetPrimaryMonitor,
    CreateWindow,
}
