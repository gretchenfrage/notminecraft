//! Client-side connection implementation for websocket transport. See the corresponding server
//! module for an explanation of the protocol.

use super::*;
use crate::{
    client::{
        channel::ClientSender,
        ClientEvent,
    },
    message::*,
    game_data::GameData,
    game_binschema::GameBinschema,
    util_net::{
        try_close,
        some_or_pending,
    },
};
use binschema::*;
use std::{
    sync::Arc,
    time::{Duration, Instant},
    convert::Infallible,
    io::Cursor,
};
use url::{
    Url,
    ParseError,
};
use tokio::{
    sync::{
        mpsc::{
            Sender,
            Receiver,
            UnboundedSender,
            UnboundedReceiver,
            unbounded_channel,
            channel,
        },
        Semaphore,
        Notify,
        oneshot,
    },
    runtime::Handle,
    time::sleep,
};
use tokio_tungstenite::{
    tungstenite::{
        protocol::WebSocketConfig,
        error::Error as TungsteniteError,
        Message,
    },
    connect_async_with_config,
};
use futures::{
    stream::{Stream, StreamExt},
    sink::{Sink, SinkExt},
    FutureExt as _,
    select_biased,
};
use anyhow::{
    Error,
    bail,
    ensure,
    anyhow,
};


// should be changed if meta-level things about how ws-binschema integration works changes.
const WS_BINSCHEMA_CLIENT_HELLO_MAGIC_BYTES: [u8; 4] = [0x2b, 0xc7, 0xa3, 0x41];

// should be changed if meta-level things about how ws-binschema integration works changes.
const WS_BINSCHEMA_SERVER_HELLO_MAGIC_BYTES: [u8; 4] = [0x62, 0xc2, 0xff, 0x90];

// part of clock synchronization protocol.
const CLOCK_SYNCHRONIZED_MSG: &[u8] = b"synchronized";

// amount of time the client sleeps to let the connection quiesce before synchronizing clocks.
const QUIESCE_BEFORE_SYNC_CLOCK: Duration = Duration::from_millis(100);

// number of simultaneous ping pong messages the server will buffer for sending back in a response
// before backpressure is triggered on receive from the websocket connection.
const PING_PONG_BUFFER_LIMIT: usize = 10;

// 16 MiB. 
//
// this is both the maximum message size we tell the websocket implementation to be willing to
// receive, and the maximum number of bytes worth of messages we allow to sit in the recv channel
// unprocessed before we start applying backpressure to the server.
const RECEIVE_BUFFER_LIMIT: usize = 16 << 20;

// timeout for attempting to send a close frame on a websocket connection.
const SEND_CLOSE_TIMEOUT: Duration = Duration::from_secs(10);


// connection inner type for websocket transport
pub(super) struct Connection {
    // sender for queue of messages to be transmitted to server
    send_send: UnboundedSender<UpMsg>,
    // client estimate of instant when server sampled server_t0
    est_server_t0: Instant,
    // ensures proper shutdown when Connection dropped
    _shutdown_trigger: ConnectionShutdownTrigger,
}

// triggers connection shutdown when dropped
struct ConnectionShutdownTrigger(Arc<ConnShared>);

struct ConnShared {
    // tell the receive task to cleanly close the connection, can be called from anywhere.
    shutdown_recv: Notify,
    // tell the send task to send a close message if it can, then terminate. only called from the
    // receive task.
    shutdown_send: Notify,
    // game content
    game: Arc<GameData>,
}


impl Connection {
    // attempt to establish connection.
    pub(super) async fn connect(
        url: &str,
        client_send: ClientSender,
        rt: &Handle,
        game: &Arc<GameData>,
    ) -> Result<Self, Error> {
        let (send_send, recv_send) = unbounded_channel();
        let shared_1 = Arc::new(ConnShared {
            shutdown_recv: Notify::new(),
            shutdown_send: Notify::new(),
            game: Arc::clone(game),
        });
        let shared_2 = Arc::clone(&shared_1);
        let (send_handshake_done, recv_handshake_done) = oneshot::channel();
        rt.spawn(recv_task(
            url.to_owned(),
            recv_send,
            client_send,
            rt.clone(),
            shared_1,
            send_handshake_done,
        ));
        // create shutdown trigger now to make this method cancel/error-safe
        let shutdown_trigger = ConnectionShutdownTrigger(shared_2);
        let est_server_t0 = recv_handshake_done.await
            .map_err(|_| {
                // this _should_ never happen, but best to be defensive
                anyhow!("unexpectedly dropped Connection handshake_done oneshot")
            })??;
        Ok(Connection {
            send_send,
            est_server_t0,
            _shutdown_trigger: shutdown_trigger,
        })
    }

    // see outer type
    pub(super) fn send(&self, msg: UpMsg) {
        let _ = self.send_send.send(msg);
    }

    // see outer type
    pub(super) fn est_server_t0(&self) -> Instant {
        self.est_server_t0
    }
}

impl Drop for ConnectionShutdownTrigger {
    fn drop(&mut self) {
        self.0.shutdown_recv.notify_one();
    }
}

// body of the receive task for a connection.
async fn recv_task(
    url: String,
    recv_send: UnboundedReceiver<UpMsg>,
    client_send: ClientSender,
    rt: Handle,
    shared: Arc<ConnShared>,
    send_handshake_done: oneshot::Sender<Result<Instant, Error>>,
) {
    // parse url
    let url = match parse_url(&url) {
        Ok(url) => url,
        Err(e) => {
            error!(%e, ?url, "error parsing url");
            let _ = send_handshake_done.send(Err(anyhow!("invalid url: {}", e)));
            return;
        }
    };
    info!("connecting to {}", url);

    // try to connect and do ws handshake
    let connect = connect_async_with_config(
        url,
        Some(WebSocketConfig {
            max_message_size: Some(RECEIVE_BUFFER_LIMIT),
            ..Default::default()
        }),
        true
    );
    let result = select_biased! {
        _ = shared.shutdown_recv.notified().fuse() => {
            // abandon attempt if connection dropped by user
            trace!("abandoning ws connect because connection closed");
            return;
        }
        result = connect.fuse() => result
    };
    let mut ws = match result {
        Ok((ws, _)) => ws,
        Err(e) => {
            // close connection on failure
            error!(%e, "error establishing ws connection");
            let _ = send_handshake_done.send(Err(anyhow!("unable to connect: {}", e)));
            return;
        }
    };

    // try to do ws-binschema handshake
    let up_schema = UpMsg::schema(&shared.game);
    let down_schema = DownMsg::schema(&shared.game);
    let handshake = handshake(&mut ws, &up_schema, &down_schema);
    let (coder_state_alloc, est_server_t0) = select_biased! {
        _ = shared.shutdown_recv.notified().fuse() => {
            // abandon attempt if connection dropped by user
            trace!("abandoning ws-binschema handshake because connection closed");
            return;
        }
        result = handshake.fuse() => match result {
            Ok(outcome) => outcome,
            Err(e) => {
                // close connection _elegantly_ on failure
                error!(%e, "error in ws-binschema handshake (closing connection)");
                let _ = send_handshake_done.send(Err(e));
                try_close(ws, None, SEND_CLOSE_TIMEOUT).await;
                return;
            }
        }
    };
    let _ = send_handshake_done.send(Ok(est_server_t0));
    let (ws_send, mut ws_recv) = ws.split();

    // spawn the send task
    let (send_pong, recv_pong) = channel(PING_PONG_BUFFER_LIMIT);
    rt.spawn(send_task(ws_send, Arc::clone(&shared), recv_pong, recv_send, up_schema));

    // enter recv loop until something breaks it
    let recv_loop = recv_loop(
        coder_state_alloc,
        &mut ws_recv,
        &shared,
        send_pong,
        &client_send,
        down_schema,
    );
    let closed_event = select_biased! {
        _ = shared.shutdown_recv.notified().fuse() => {
            // shutdown requested
            trace!("recv task shutting down because shut down requested");
            None
        }
        result = recv_loop.fuse() => {
            // send loop errored
            let e = match result {
                Err(e) => e,
                Ok(never) => match never {},
            };
            error!(%e, "recv loop error (closing connection)");
            Some(ClientEvent::Network(NetworkEvent::Closed(Some(e.to_string()))))
        }
    };

    // shut down elegantly
    shared.shutdown_send.notify_one();
    if let Some(closed_event) = closed_event {
        client_send.send(closed_event, EventPriority::Network, None, None);
    }
}

// attempt to do the ws-binschema handshake
async fn handshake<W>(
    ws: &mut W,
    up_schema: &Schema,
    down_schema: &Schema,
) -> Result<(CoderStateAlloc, Instant), Error>
where
    W: Stream<Item=Result<Message, TungsteniteError>>
    + Sink<Message, Error=TungsteniteError> + Unpin,
{
    // ==== clock synchronization ====

    // let it quiesce
    sleep(QUIESCE_BEFORE_SYNC_CLOCK).await;

    // time-sensitive part
    let client_t0 = Instant::now();
    ws.send(Message::Binary(WS_BINSCHEMA_CLIENT_HELLO_MAGIC_BYTES.into())).await?;
    let received = handshake_recv(ws).await?;
    let client_t1 = Instant::now();

    // non time-sensitive part
    ensure!(
        received == WS_BINSCHEMA_SERVER_HELLO_MAGIC_BYTES,
        "server ws-binschema msg has wrong ws-binschema server hello magic bytes",
    );
    ws.send(Message::Binary(CLOCK_SYNCHRONIZED_MSG.into())).await?;
    let est_rtt = client_t1.duration_since(client_t0);
    debug!("est rtt {:.6} ms", est_rtt.as_nanos() as f64 / 1_000_000.0);
    let est_server_t0 = client_t0 + est_rtt / 2;

    // ==== schema handshake ====

    // allocate state
    let mut buf = Vec::new();
    let mut coder_state_alloc = CoderStateAlloc::new();

    // form the up handshake msg
    buf.extend(&Schema::schema_schema_magic_bytes());
    let schema_schema = Schema::schema_schema();
    for schema in [&up_schema, &down_schema] {
        let mut coder_state = CoderState::new(&schema_schema, coder_state_alloc, None);
        let result = schema.encode_schema(&mut Encoder::new(&mut coder_state, &mut buf));
        if cfg!(debug_assertions) {
            result
                .and_then(|()| coder_state.is_finished_or_err())
                .expect("error encoding message schema for ws-binschema handshake");
        }
        coder_state_alloc = coder_state.into_alloc();
    }

    // transmit the up schema-handshake msg
    ws.send(Message::Binary(buf.clone())).await?;

    // receive the down schema-handshake message
    let received = handshake_recv(ws).await?;

    // now try to validate the server's schema-handshake msg
    let idx1 = Schema::schema_schema_magic_bytes().len();
    ensure!(received.len() >= idx1, "server ws-binschema handshake msg too short");
    ensure!(
        &received[..idx1] == Schema::schema_schema_magic_bytes(),
        "server ws-binschema handshake msg has wrong schema schema magic bytes",
    );
    let mut cursor = Cursor::new(&received[idx1..]);
    for (expected_schema, error_msg_name) in [
        (down_schema, "down schema"),
        (up_schema, "up schema"),
    ] {
        let mut coder_state = CoderState::new(&schema_schema, coder_state_alloc, None);
        let result = Schema::decode_schema(&mut Decoder::new(&mut coder_state, &mut cursor))
            .and_then(|msg| coder_state
                .is_finished_or_err()
                .map(move |()| msg));
        match result {
            Ok(actual_schema) => {
                if &actual_schema != expected_schema {
                    error!(
                        "server declared incompatible {}:\n{}",
                        error_msg_name,
                        actual_schema.pretty_fmt(),
                    );
                    bail!("server declared incompatible {}", error_msg_name);
                }
            }
            Err(e) => {
                if e.kind().is_programmer_fault() {
                    panic!("Schema::decode_schema programmer fault error: {}", e);
                } else {
                    bail!(
                        "binschema error decoding server's {}: {}",
                        error_msg_name,
                        e,
                    );
                }
            }
        }
        coder_state_alloc = coder_state.into_alloc();
    }
    ensure!(
        cursor.position() >= cursor.get_ref().len() as u64,
        "server ws-binschema handshake msg has extra bytes at end"
    );

    Ok((coder_state_alloc, est_server_t0))
}

// attempt to receive a binary ws message within the ws-binschema handshake.
// (distinct from after the handshake because the ws is not yet split).
async fn handshake_recv<W>(ws: &mut W) -> Result<Vec<u8>, Error>
where
    W: Stream<Item=Result<Message, TungsteniteError>>
    + Sink<Message, Error=TungsteniteError> + Unpin,
{
    Ok(loop {
        let item = ws.next().await;
        match item {
            // found binary message
            Some(Ok(Message::Binary(msg))) => break msg,
            Some(Ok(Message::Ping(msg))) => {
                // respond to ping with pong and continue
                ws.send(Message::Pong(msg)).await?;
                continue;
            }
            Some(Ok(Message::Close(close_frame))) => {
                // if connection closed on ws level, that's an error
                if let Some(close_frame) = close_frame {
                    trace!(?close_frame, "received close frame from server");
                }
                bail!("server closed connection");
            }
            // other message types are protocol errors
            Some(Ok(_)) => bail!("server send invalid ws msg type"),
            // errors are errors
            Some(Err(e)) => Err(e)?,
            // closing in this way is an error
            None => bail!("ws connection closed"),
        }
    })
}

// message receiving loop for the portion of recv task where the connection is alive
async fn recv_loop<W>(
    mut coder_state_alloc: CoderStateAlloc,
    ws: &mut W,
    shared: &Arc<ConnShared>,
    send_pong: Sender<Vec<u8>>,
    client_send: &ClientSender,
    down_schema: Schema,
) -> Result<Infallible, Error>
where
    W: Stream<Item=Result<Message, TungsteniteError>> + Unpin,
{
    let backpressure_semaphore = Arc::new(Semaphore::new(RECEIVE_BUFFER_LIMIT));
    loop {
        // receive next binary message or early escape loop iteration
        let item = ws.next().await;
        let msg = match item {
            // found binary message
            Some(Ok(Message::Binary(msg))) => msg,
            Some(Ok(Message::Ping(msg))) => {
                // respond to ping with pong and continue
                let _ = send_pong.send(msg).await;
                continue;
            }
            Some(Ok(Message::Close(close_frame))) => {
                // if connection closed on ws level, that's an error
                if let Some(close_frame) = close_frame {
                    trace!(?close_frame, "received close frame from server");
                }
                bail!("server closed connection");
            }
            // other message types are protocol errors
            Some(Ok(_)) => bail!("server send invalid ws msg type"),
            // errors are errors
            Some(Err(e)) => Err(e)?,
            // closing in this way is an error
            None => bail!("ws connection closed"),
        };
        let msg_size = msg.len();

        // decode
        let mut cursor = Cursor::new(msg.as_slice());
        let mut coder_state = CoderState::new(&down_schema, coder_state_alloc, None);
        let result =
            DownMsg::decode(
                &mut Decoder::new(&mut coder_state, &mut cursor), &shared.game,
            )
            .and_then(|msg| coder_state
                .is_finished_or_err()
                .map(move |()| msg));
        if let &Err(ref e) = &result {
            if e.kind().is_programmer_fault() {
                error!(%e, "decoding error detected as being programmer's fault");
            }
        }
        let msg = result?;
        coder_state.is_finished_or_err()?;
        coder_state_alloc = coder_state.into_alloc();
        ensure!(
            cursor.position() >= cursor.get_ref().len() as u64,
            "received msg with extra bytes at end",
        );

        // receive backpressure
        // unwrap safety: we never close the semaphore
        let permit = Arc::clone(&backpressure_semaphore)
            .acquire_many_owned(msg_size as u32).await.unwrap();

        // deliver received message to user
        client_send.send(
            ClientEvent::Network(NetworkEvent::Received(msg)),
            EventPriority::Network,
            None,
            Some(permit),
        );
    }
}

// body of the send task for a connection
async fn send_task<W>(
    mut ws: W,
    shared: Arc<ConnShared>,
    recv_pong: Receiver<Vec<u8>>,
    recv_send: UnboundedReceiver<UpMsg>,
    up_schema: Schema,
)
where
    W: Sink<Message, Error=TungsteniteError> + Unpin,
{
    // enter send loop until something breaks it
    let send_loop = send_loop(&mut ws, &shared, recv_pong, recv_send, up_schema);
    select_biased! {
        _ = shared.shutdown_send.notified().fuse() => {
            // shutdown requested
            trace!("send task shutting down because shut down requested");
        }
        result = send_loop.fuse() => {
            // send loop errored
            let e = match result {
                Err(e) => e,
                Ok(never) => match never {},
            };
            // tell the receive task to shut down in this case
            error!(%e, "send loop error (closing connection)");
            shared.shutdown_recv.notify_one();
        }
    }

    // try to close elegantly
    try_close(ws, None, SEND_CLOSE_TIMEOUT).await;
}

// message sending loop for the portion of send task where the connection is alive
async fn send_loop<W>(
    ws: &mut W,
    shared: &Arc<ConnShared>,
    mut recv_pong: Receiver<Vec<u8>>,
    mut recv_send: UnboundedReceiver<UpMsg>,
    up_schema: Schema,
) -> Result<Infallible, Error>
where
    W: Sink<Message, Error=TungsteniteError> + Unpin,
{
    let mut coder_state_alloc = CoderStateAlloc::new();
    let mut buf = Vec::new();
    loop {
        // get message to send or early escape loop iteration
        let msg = select_biased! {
            msg = some_or_pending(recv_pong.recv()).fuse() => {
                // respond to ping with pong then skip to next loop iteration
                ws.send(Message::Pong(msg)).await?;
                continue
            }
            msg = some_or_pending(recv_send.recv()).fuse() => msg,
        };

        // encode
        let mut coder_state = CoderState::new(&up_schema, coder_state_alloc, None);
        msg
            .encode(&mut Encoder::new(&mut coder_state, &mut buf), &shared.game)
            .and_then(|()| coder_state.is_finished_or_err())?;

        // transmit
        ws.send(Message::Binary(buf.clone())).await?;

        // reset
        coder_state_alloc = coder_state.into_alloc();
        buf.clear();
    }
}

// parse url and fill in default parts if absent
fn parse_url(url: &str) -> Result<Url, Error> {
    let mut url = match Url::parse(url) {
        Ok(url) => url,
        Err(ParseError::RelativeUrlWithoutBase) => Url::parse(&format!("ws://{}", url))?,
        Err(e) => Err(e)?,
    };
    if url.scheme().is_empty() {
        url.set_scheme("ws").unwrap();
    }
    if url.port().is_none() {
        url.set_port(Some(35565)).unwrap();
    }
    Ok(url)
}
