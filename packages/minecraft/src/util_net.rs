//! Network utilities.

use std::{
    sync::Once,
    time::Duration,
};
use tokio::{
    net::TcpStream,
    time::timeout,
};
use tokio_tungstenite::tungstenite::{
    protocol::frame::{
        coding::CloseCode,
        CloseFrame,
    },
    error::Error as TungsteniteError,
    Message,
};
use futures::{
    sink::{Sink, SinkExt},
    future::pending,
    Future,
};


/// Attempt to disable nagling, log error on failure.
pub fn try_denagle(tcp: &TcpStream) {
    let denagle_result = tcp.set_nodelay(true);
    if let Err(e) = denagle_result {
        // ooh I get to use a static variable in Rust, how exciting!
        static WARN_DENAGLE_FAILED: Once = Once::new();
        WARN_DENAGLE_FAILED.call_once(|| warn!(%e, "failed to disable nagling"));
    }
}

/// Attempt to elegantly close a websocket connection by sending a close message, with a timeout.
/// If `reason` is given it will be sent in a close frame to the client.
pub async fn try_close<W>(mut ws: W, reason: Option<&'static str>, timeout_duration: Duration)
where
    W: Sink<Message, Error=TungsteniteError> + Unpin,
{
    trace!(?reason, "sending ws close frame");
    let close_frame = reason
        .map(|reason| CloseFrame {
            code: CloseCode::Invalid,
            reason: reason.into(),
        });
    let msg = Message::Close(close_frame);
    let result = timeout(timeout_duration, ws.send(msg)).await;
    match result {
        Ok(Ok(())) => (),
        Ok(Err(e)) => trace!(%e, "error sending close frame"),
        Err(_) => trace!("timeout sending close frame"),
    }
}

/// Wrapper around a future option that resolves to the some value or pends forever.
pub async fn some_or_pending<T, F: Future<Output=Option<T>>>(option: F) -> T {
    match option.await {
        Some(t) => t,
        None => pending().await,
    }
}
