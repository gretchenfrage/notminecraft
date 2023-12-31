//! Network connection implementation for websocket transport.
//!
//! This is a websocket connection running over TCP. Encryption is not yet enabled. Only binary
//! messages are used, not text ones. When the connection begins, after the ws handshake completes,
//! the "ws-binschema" handshake is performed as such:
//!
//! 1. Both sides transmit a message containing a concatenation of:
//!
//!    - A constant defined here, the "ws-binschema magic bytes", which should be changed if this
//!      binschema integration protocol is changed.
//!    - A constant defined in binschema, the "schema schema magic bytes", which should be changed
//!      if binschema's behavior or the schema of schemas are changed.
//!    - The binschema-encoded schema of the messages this side will send.
//!    - The binschema-encoded schema of the messages this side will receive.
//!
//! 2. Both sides wait to receive that message from the other side and validate that it matches
//!    their expectations or is otherwise acceptable.
//!
//! Internals
//! ---------
//!
//! The bind task accepts TCP streams, and then spawns a new receive task for each stream. The
//! receive task is the only task that generates network events for the connection. The receive
//! task does the ws handshake, creates the connection, and spawns the send task. Once the send
//! task is spawned it is the only task that sends data on the stream.
//!
//! The receive task has associated with it a shared `Notify` that is used to put it into the
//! shutdown state. This can be called from the connection handle if killed, from the network
//! server if dropped, or from the send task if it errors. Upon entering this state, it destroys
//! the connection and uses a second shared `Notify` associated with the transmit task to put it
//! into its own shutdown state. In this state, it attempts to gracefully close the websocket
//! connection on a timeout, then dies.
//!
//! Additionally, a bound-length channel is shared between the receive and send tasks for the
//! receive task to convey to the send task pong messages it should send in response to ping
//! messages.

use super::{
    send_buffer_policy_enforcer::SendBufferPolicyEnforcer,
    *,
};
use crate::{
    server::{
        ServerEvent,
        channel::*,
    },
    game_data::*,
};
use binschema::*;
use std::{
    sync::{Arc, Once},
    time::{Duration, Instant},
    convert::Infallible,
    cmp::{min, max},
    future::Future,
};
use tokio::{
    sync::{
        mpsc::{
            Receiver,
            UnboundedSender,
            UnboundedReceiver,
            channel,
            unbounded_channel,
        },
        Notify,
        Semaphore,
    },
    task::AbortHandle,
    net::{TcpListener, TcpStream},
    time::{
        sleep,
        timeout,
        timeout_at,
    },
};
use tokio_tungstenite::{
    tungstenite::{
        protocol::{
            frame::{
                coding::CloseCode,
                CloseFrame,
            },
            WebSocketConfig,
        },
        error::Error as TungsteniteError,
        Message,
    },
    accept_async_with_config,
};
use futures::{
    stream::{Stream, StreamExt},
    sink::Sink,
    future::pending,
    select,
    select_biased,
};
use anyhow::{Error, bail};


// should be changed if meta-level things about how ws-binschema integration works changes  
const WS_BINSCHEMA_MAGIC_BYTES: [u8; 4] = [0x1f, 0x1b, 0x08, 0x63];

// 16 MiB. 
//
// this is both the maximum message size we tell the websocket implementation to be willing to
// receive, and the maximum number of bytes worth of messages we allow to sit in the server channel
// unprocessed before we start applying backpressure to the client. as such, it's possible for each
// client connection to overall buffer twice this many bytes.
const RECEIVE_BUFFER_LIMIT: usize = 16 << 20;

// exponential backoff parameters for various failures in accepting new TCP connections
const BIND_BACKOFF_MIN: Duration = Duration::from_millis(100);
const BIND_BACKOFF_MAX: Duration = Duration::from_secs(60);

// timeout for the websocket and ws-binschema handshake to complete after a TCP connection is
// established. this can be one component of TCP connection exhaustion attack mitigation.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(60);

// timeout for attempting to send a close frame on a websocket connection.
const SEND_CLOSE_TIMEOUT: Duration = Duration::from_secs(10);

// number of simultaneous ping pong messages the server will buffer for sending back in a response
// before backpressure is triggered on receive from the websocket connection.
const PING_PONG_BUFFER_LIMIT: usize = 10;


// slab entry inner type for websocket connections.
pub(super) struct SlabEntry {
    // notify to put the recv task into the shutdown state
    send_shutdown_recv: Arc<Notify>,
}

impl SlabEntry {
    // called upon network shutdown
    pub(super) fn shutdown(&self) {
        self.send_shutdown_recv.notify_one();
    }
}

// connection inner type for websocket connections
pub(super) struct Connection {
    // sender for queue of messages to send
    send_send: UnboundedSender<DownMsg>,
    // notify to put the recv task into the shutdown state
    send_shutdown_recv: Arc<Notify>,
}

// general module-level shared context
struct WsShared {
    // the network server shared state struct
    ns_shared: Arc<NetworkServerSharedState>,
    // schema of up messages
    up_schema: Schema,
    // schema of down messages
    down_schema: Schema,
    // upwards-travelling ws binschema handshake message to expect
    up_handshake: Vec<u8>,
    // downwards-travelling ws-binschema handshake message to transmit
    down_handshake: Vec<u8>,
    // handle to the tokio runtime for spawning tasks
    rt: Handle,
    // game content
    game: Arc<GameData>,
}

// shared state for a single ws connection
struct WsConnShared {
    // tell the receive task to cleanly close the connection. can be called from anywhere.
    shutdown_recv: Notify,
    // tell the send task to send a close message if it can, then terminate. only called from the
    // receive task.
    shutdown_send: Notify,
}


impl Connection {
    // see outer type
    pub(super) fn send(&self, msg: DownMsg) {
        let _ = self.send_send.send(msg);
    }

    // see outer type
    pub(super) fn kill(&self) {
        self.send_shutdown_recv.notify_one();
    }
}

// bind to port and start accepting connections on it
pub(super) fn bind<B>(server: &mut NetworkServer, bind_to: B, rt: &Handle, game: &Arc<GameData>)
where
    B: ToSocketAddrs + Send + Sync + 'static,
{
    // spawn the accept task
    let join_accept = rt.spawn(accept_task(
        Arc::clone(&server.shared),
        bind_to,
        rt.clone(),
        Arc::clone(game),
    ));

    // store its abort handle for when the network server closes
    server.bind_abort_handles.push(join_accept.abort_handle());
}

// body of the task to bind to the TCP port and start accepting new connections
async fn accept_task<B: ToSocketAddrs>(
    ns_shared: Arc<NsShared>,
    bind_to: B,
    rt: Handle,
    game: Arc<GameData>,
) {
    // initialize shared state
    let up_schema = UpMsg::schema(&game);
    let down_schema = DownMsg::schema(&game);
    let up_handshake = form_handshake_msg(&up_schema, &down_schema);
    let down_handshake = form_handshake_msg(&down_schema, &up_schema);

    let ws_shared = Arc::new(WsShared {
        ns_shared,
        up_schema,
        down_schema,
        up_handshake,
        down_handshake,
        rt,
        game,
    });

    // keep trying the inner part
    let mut backoff = BIND_BACKOFF_MIN;
    let mut attempt_start = Instant::now();
    loop {
        // try until error
        let result = try_accept_task_inner(&ws_shared, &bind_to);
        let attempt_end = Instant::now();

        // log error
        let Err(e) = result;
        error!(%e, "websocket accept task error (retrying in {:.3} s)", backoff.as_secs_f32());

        // backoff sleep
        sleep(backoff).await;

        // increase the backoff, unless the attempt ran for a long time, in which case reset it
        let attempt_elapsed = attempt_end - attempt_start;
        if attempt_elapsed > BIND_BACKOFF_MAX {
            backoff = BIND_BACKOFF_MIN;
        } else {
            backoff *= 2;
            backoff = min(backoff, BIND_BACKOFF_MAX);
        }
    }
}

// form a ws-binschema handshake message that should be sent by the side transmitting send_schema
fn form_handshake_msg(send_schema: &Schema, recv_schema: &Schema) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend(&WS_BINSCHEMA_MAGIC_BYTES);
    buf.extend(&Schema::schema_schema_magic_bytes());
    let schema_schema = Schema::schema_schema();
    let mut coder_state_alloc = CoderStateAlloc::new();
    for schema in [send_schema, recv_schema] {
        let mut coder_state = CoderState::new(&schema_schema, coder_state_alloc, None);
        let result = msg_schema.encode_schema(&mut Encoder::new(&mut coder_state, &mut buf));
        if cfg!(debug_assertions) {
            result
                .and_then(|()| coder_state.is_finished_or_err())
                .expect("error encoding message schema for ws-binschema handshake");
        }
        coder_state_alloc = coder_state.into_alloc();
    }
    buf
}

// inner part of the accept task which gets retried if fails
async fn try_accept_task_inner<B: ToSocketAddrs>(
    ws_shared: &Arc<WsShared>,
    bind_to: &B,
) -> Result<Infallible, Error> {
    // TCP bind
    let listener = TcpListener::bind(bind_to).await?;

    // accept connections
    loop {
        // spawn the receive task for each
        let (tcp, _) = listener.accept().await?;
        rt.spawn(recv_task(Arc::clone(ws_shared), tcp));
    }
}

// body of the receive task for a connection
//
// 1. does the ws + ws-binschema handshake
// 2. spawns the corresponding send task
// 3. switches to itself doing the receive loop
async fn recv_task(ws_shared: Arc<WsShared>, tcp: TcpStream) {
    // attempt to disable nagling
    try_denagle(&tcp);

    // attempt to do the ws and ws-binschema handshakes (with timeout)
    let ws = try_handshake_handle_err(tcp, &ws_shared).await;
    let ws = match ws {
        Some(ws) => ws,
        // if handshake failed, the task can just stop here
        None => return,
    };

    // split this beast in twain and in twixt
    let (ws_send, mut ws_recv) = ws.split();

    // construct channels for communicating with tasks

    // (send/recv)_shutdown_recv tells the recv task to shut down
    let send_shutdown_recv_1 = Arc::new(Notify::new());
    let send_shutdown_recv_2 = Arc::clone(&send_shutdown_recv_1);
    let send_shutdown_recv_3 = Arc::clone(&send_shutdown_recv_1);
    let recv_shutdown_recv = Arc::new(Notify::new());
    // (send/recv)_shutdown_send tells the send task to shut down
    let send_shutdown_send = Arc::new(Notify::new());
    let recv_shutdown_send = Arc::new(Notify::new());
    // (send/recv)_pong tells the send task to send a pong message
    let (send_pong, recv_pong) = channel(PING_PONG_BUFFER_LIMIT);
    // (send/recv)_send tells the send task to send an application message
    let (send_send, recv_send) = unbounded_channel();

    // create the connection
    let conn_idx = create_conn(
        &ws_shared.ns_shared,
        super::SlabEntry::Ws(SlabEntry { send_shutdown_recv: send_shutdown_recv_1 }),
        super::Connection(ConnectionInner::Ws(Connection {
            send_send,
            send_shutdown_recv: send_shutdown_recv_2,
        })),
    );
    let conn_idx = match conn_idx {
        Some(conn_idx) => conn_idx,
        // this case happens if the whole network server is shutting down. just drop everything.
        None => return,
    };

    // spawn the send task
    let task = send_task(
        Arc::clone(&ws_shared),
        ws_send,
        send_shutdown_recv_3,
        recv_shutdown_send,
        recv_pong,
        recv_send,
    );
    ws_shared.rt.spawn(task);

    // allocate other recv task state
    let coder_state_alloc = CoderStateAlloc::new();
    let backpressure_semaphore = Arc::new(Semaphore::new(RECEIVE_BUFFER_LIMIT));

    // enter receive task event loop, break loop when connection should be shut down
    loop {
        // try to receive a message, break loop if told to shut down
        let result = select_biased! {
            _ = recv_shutdown_recv.notified() => break,
            result = ws_recv.next() => result,
        };

        // try to extract ok result, log and break loop on error
        let msg: Message = match result {
            Ok(msg) => msg,
            Err(e) => {
                trace!(%e, "ws error receiving message (killing connection)");
                break;
            }
        };

        // try to extract application message
        let msg: Vec<u8> = match msg {
            Message::Binary(msg) => msg,
            Message::Ping(msg) => {
                // in the event of a ping, try to tell the send task to send a pong
                select! {
                    _ = send_pong.send(msg) => (),
                    // but if gets blocked on ping-pong buffer backpressure, and then told to shut
                    // down while still blocking on that, break loop
                    _ = recv_shutdown_recv.notified() => break,
                };

                // upon successfully forwarding ping pong, continue loop
                continue;
            }
            // if connection closed on ws level, break loop
            Message::Close(_) => break,
            // if any other type of message received, break loop
            _ => {
                trace!("invalid ws msg type received (killing connection)");
                break;
            }
        };

        // store this for backpressure
        let msg_size = msg.len();

        // try to decode up msg
        let mut coder_state = CoderState::new(&ws_shared.up_schema, coder_state_alloc, None);
        let result =
            UpMsg::decode(
                &mut Decoder::new(&mut coder_state, &mut msg.as_slice()), &ws_shared.game,
            )
            .and_then(|msg| coder_state
                .is_finished_or_err()
                .map(move |()| msg));
        coder_state_alloc = coder_state.into_alloc();

        // on decoding error, log and break loop
        let msg: UpMsg = match result {
            Ok(msg) => msg,
            Err(e) => {
                trace!(%e, "error decoding ws-binschema msg (killing connection)");
                break;
            }
        };

        // try to wait for receive backpressure to allow the delivery of this message to the server
        let permit = select! {
            // we shouldn't ever close the semaphore
            result = backpressure_semaphore.acquire_many_owned(msg_size) => result.unwrap(),
            // but if gets blocked on backpressure, and then is told to shut down, break loop
            _ => recv_shutdown_recv.notified() => break,
        };

        // finally, we can send the received message to the server loop
        ws_shared.ns_shared.server_send.send(
            ServerEvent::Network(NetworkEvent::Message(conn_idx, msg)),
            EventPriority::Network,
            None,
            Some(permit),
        );
    }

    // loop is broken, so shut down connection
    drop(ws_recv);
    send_shutdown_send.notify_one();
    destroy_conn(&ws_shared.ns_shared, conn_idx);
}

// body of the send task for a connection
async fn send_task<W: Sink<Message, Error=TungsteniteError> + Unpin>(
    ws_shared: Arc<WsShared>,
    mut ws_send: W,
    send_shutdown_recv: Arc<Notify>,
    recv_shutdown_send: Arc<Notify>,
    recv_pong: Receiver<Vec<u8>>,
    recv_send: UnboundedReceiver<DownMsg>,
) {
    // allocate send task reusable allocations
    let mut coder_state_alloc = CoderStateAlloc::new();

    // enter send task event loop
    loop {
        // determine what message we should send, or early return
        let msg: Message = select_biased! {
            // send task was told to shut down by recv task
            _ = shutdown_send.notified() => {
                drop(recv_send);
                try_close(ws_send, None).await;
                return;
            },
            // respond to a ping with a pong
            msg = some_or_pending(recv_pong.recv()) => Message::Pong(msg),
            // encode down msg to be sent
            msg = some_or_pending(recv_send.recv()) => {
                let msg: DownMsg = msg;
                let mut buf = Vec::new();
                let mut coder_state =
                    CoderState::new(&ws_shared.down_schema, coder_state_alloc, None);
                let result = msg
                    .encode(&mut Encoder::new(&mut coder_state, &mut buf), &ws_shared.game)
                    .and_then(|()| coder_state.is_finished_or_err())
                coder_state_alloc = coder_state.into_alloc();
                
                if let Err(e) = result {
                    // if encounter error encoding it, kill connection.
                    //
                    // log this extra bad because this actually does indicate a server programmer
                    // error.
                    drop(recv_send);
                    error!(%e, "error encoding down message (killing connection)");    
                    send_shutdown_recv.notify_one();
                    drop(ws_send);
                    return;
                }
                
                Message::Binary(message)
            }
        };

        // try to send the message
        select! {
            result = ws_send.send(msg) => {
                if let Err(e) = result {
                    // if encounter error sending it, kill the connection
                    drop(recv_send);
                    trace!(%e, "ws error sending message (killing connection)");
                    send_shutdown_recv.notify_one();
                    drop(ws_send);
                    return;
                }
            }
            // if send task told to shut down by recv task while waiting for message to send, then
            // shut down.
            _ = shutdown_send.notified() => {
                drop(recv_send);
                try_close(ws_send, None).await;
                return;
            }
        }
    }
}

// wrapper around a future option that resolves to the some value or pends forever
async fn some_or_pending<T, F: Future<Output=Option<T>>>(option: T) -> T {
    match option.await {
        Some(t) => t,
        None => pending().await,
    }
}

// attempt to disable nagling, log error on failure
fn try_denagle(tcp: &TcpStream) {
    let denagle_result = tcp.set_nodelay(true);
    if let Err(e) = denagle_result {
        // ooh I get to use a static variable in Rust, how exciting!
        static WARN_DENAGLE_FAILED: Once = Once::new();
        WARN_DENAGLE_FAILED.call_once(|| warn!(%e, "failed to disable nagling"));
    }
}

// attempt to do a ws handshake then a ws-binschema handshake on the TCP stream. on error, attempt
// to handle the error appropriately. implement timeouts as necessary in both parts.
async fn try_handshake_handle_err(
    tcp: TcpStream,
    ws_shared: &Arc<WsShared>,
) -> Option<WebSocketStream<TcpStream>> {
    let deadline = Instant::now() + HANDSHAKE_TIMEOUT;
    let result = try_handshake(tcp, deadline, ws_shared).await;
    match result {
        // success
        Ok(ws) => Some(ws),
        Err(HandshakeError::Ws(e)) => {
            // just log these ones
            trace!(%e, "ws-level handshake error");
            None
        }
        Err(HandshakeError::WsBinschema { ws, reason }) => {
            // try to send back a close frame, with timeout
            try_close(ws, reason).await;
            None
        }
        Err(HandshakeError::Timeout(opt_ws)) => {
            // try to send back a close frame here too, if applicable
            if let Some(ws) = opt_ws {
                try_close(ws, "ws-binschema handshake timeout");
            }
            None
        }
    }
}

// attempt to do a ws handshake then a ws-binschema handshake on the TCP stream with the given
// timeout instant.
async fn try_handshake(
    tcp: TcpStream,
    deadline: Instant,
    ws_shared: &Arc<WsShared>,
) -> Result<WebSocketStream<TcpStream>, HandshakeError> {
    // try to do the websocket handshake, with timeout
    let ws_task = accept_async_with_config(
        tcp,
        Some(WebSocketConfig {
            max_message_size: Some(RECEIVE_BUFFER_LIMIT),
            ..Default::default()
        }),
    );
    let mut ws = match timeout_at(deadline, ws_task).await {
        Ok(Ok(ws)) => ws,
        Ok(Err(e)) => return Err(HandshakeError::Ws(e)),
        Err(_) => return Err(HandshakeError::Timeout(None)),
    };

    // try to transmit the down handshake msg, with timeout
    let send_task = ws.send(Message::Binary(ws_shared.down_handshake.clone()));
    match timeout_at(deadline, send_task).await {
        Ok(Ok(())) => (),
        Ok(Err(e)) => return Err(HandshakeError::Ws(e)),
        Err(_) => return Err(HandshakeError::Timeout(Some(ws))),
    };

    // receive the first binary message from the other side
    let received = loop {
        // receive message with timeout 
        let ws_msg = match timeout_at(deadline, ws.next().await) {
            Ok(Ok(ws_msg)) => ws_msg,
            Ok(Err(e)) => return Err(HandshakeError::Ws(e)),
            Err(_) => return Err(HandshakeError::Timeout(Some(ws))),
        };

        // branch on message type
        match ws_msg {
            // found binary message
            Message::Binary(msg) => break msg,
            // try to respond to pings with pongs, with timeout
            Message::Ping(msg) => {
                let send_task = ws.send(Message::Pong(msg));
                match timeout_at(deadline, send_task).await {
                    Ok(Ok(())) => (),
                    Ok(Err(e)) => return Err(HandshakeError::Ws(e)),
                    Err(_) => return Err(HandshakeError::Timeout(Some(ws))),
                };
            }
            // if connection closed on ws level, that's an error
            Message::Close(_) => return HandshakeError::Ws(TungsteniteError::ConnectionClosed),
            // count the receipt of other message types as a ws-binschema error
            _ => return Err(HandshakeError::WsBinschema { ws, reason: "invalid ws msg type" }),
        }
    };

    // validate
    if &received != &ws_shared.up_handshake {
        return Err(HandshakeError::WsBinschema { ws, reason: "wrong up handshake msg" });
    }

    // done! :D
    Ok(ws)
}

// ways a handshake can fail
enum HandshakeError {
    // error in the underlying websocket transport. just drop the connection.
    Ws(TungsteniteError),
    // error in the ws-binschema handshake--try to properly close the websocket connection.
    WsBinschema {
        ws: WebSocketStream<TcpStream>,
        // string to be transmitted to client in close frame
        reason: &'static str,
    },
    // handshake timeout reached. try to properly close websocket connection if exists.
    Timeout(Option<WebSocketStream<TcpStream>>),
}

// attempt to elegantly close a websocket connection by sending a close message, with a timeout. if
// `reason` is given it will be sent in a close frame to the client.
async fn try_close<W>(mut ws: W, reason: Option<&'static str>)
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
    let result = timeout(SEND_CLOSE_TIMEOUT, ws.send(msg)).await;
    match result {
        Ok(Ok(())) => (),
        Ok(Err(e)) => trace!(%e, "error sending close frame"),
        Err(_) => trace!("timeout sending close frame"),
    }
}
