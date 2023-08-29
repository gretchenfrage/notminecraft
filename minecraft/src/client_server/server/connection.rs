//! Server-side connection handling system.

use crate::{
    game_data::GameData,
    client_server::message::{
        UpMessage,
        DownMessage,
    },
    game_binschema::GameBinschema,
};
use binschema::{
    CoderStateAlloc,
    CoderState,
    Encoder,
    Decoder,
};
use std::{
    time::Duration,
    sync::Arc,
    marker::Unpin,
};
use anyhow::Result;
use crossbeam_channel::{
    Sender,
    Receiver,
    unbounded,
};
use parking_lot::Mutex;
use tokio::{
    runtime::Handle,
    net::{
        ToSocketAddrs,
        TcpListener,
        TcpStream,
    },
    time::sleep,
    sync::mpsc::{
        UnboundedSender as TokioUnboundedSender,
        UnboundedReceiver as TokioUnboundedReceiver,
        unbounded_channel as tokio_unbounded_channel,
    },
};
use tokio_tungstenite::{
    accept_async,
    tungstenite::{
        error::Error as WsError,
        Message as WsMessage,
    },
};
use futures::{
    prelude::*,
    select,
};
use slab::Slab;


/// Event delivered to the server thread that something network-y happened.
///
/// Connections are identified with connection keys which are guaranteed to
/// be assigned in a slab pattern.
pub enum NetworkEvent {
    NewConnection(usize, Connection),
    Disconnected(usize),
    Received(usize, UpMessage),
}

pub struct Connection {
    send: TokioUnboundedSender<DownMessage>,
}

impl Connection {
    /// Queues message to be transmitted to client and returns immediately.
    pub fn send(&self, msg: impl Into<DownMessage>) {
        let _ = self.send.send(msg.into());
    }
}

pub fn spawn_network_stuff(
    bind_to: impl ToSocketAddrs + Send + Sync + 'static,
    rt: &Handle,
    game: &Arc<GameData>,
) -> Receiver<NetworkEvent> {
    let (send_event, recv_event) = unbounded();

    let rt_2 = Handle::clone(&rt);
    let game = Arc::clone(&game);
    rt.spawn(async move {
        // TCP bind with exponential backoff
        let mut backoff = Duration::from_millis(100);
        let listener = loop {
            match TcpListener::bind(&bind_to).await {
                Ok(listener) => break listener,
                Err(e) => {
                    error!(
                        %e,
                        "failure binding TCP listener, retrying in {}s",
                        backoff.as_secs_f32(),
                    );
                    sleep(backoff).await;
                    backoff *= 2;
                }
            }
        };

        let slab = Arc::new(Mutex::new(Slab::new()));

        // accept connections
        loop {
            match listener.accept().await {
                Ok((tcp, _)) => {
                    let slab = Arc::clone(&slab);
                    let rt = Handle::clone(&rt_2);
                    let send_event = Sender::clone(&send_event);
                    let game = Arc::clone(&game);
                    rt_2.spawn(async move {
                        // new task just to handle this TCP connection
                        let result = handle_tcp_connection(
                            tcp,
                            slab,
                            &rt,
                            send_event,
                            game,
                        ).await;
                        if let Err(e) = result {
                            warn!(%e, "connection error");
                        }
                    });
                },
                Err(e) => warn!(%e, "failure accepting TCP connection"),
            }
        }
    });

    recv_event
}

async fn handle_tcp_connection(
    tcp: TcpStream,
    slab: Arc<Mutex<Slab<()>>>,
    rt: &Handle,
    send_event: Sender<NetworkEvent>,
    game: Arc<GameData>,
) -> Result<()> {
    // do the handshake to upgrade it to websocket
    let ws = accept_async(tcp).await?;

    // initialize the connection, emit an event
    let (send_to_transmit, recv_to_transmit) = tokio_unbounded_channel();
    let key;
    {
        // it's necessary for correctness that the slab changes the set of indexes
        // in the same order that the user is presented with those changes. thus
        // this lock must be held at least until the event is sent.
        let mut slab_guard = slab.lock();
        key = slab_guard.insert(());
        let connection = Connection {
            send: send_to_transmit,
        };
        let _ = send_event.send(NetworkEvent::NewConnection(key, connection));
    }

    // split it into sink half and stream half
    let (ws_send, mut ws_recv) = ws.split();

    // spawn a new task to handle the send half
    async fn try_do_send_half(
        mut recv_to_transmit: TokioUnboundedReceiver<DownMessage>,
        mut ws_send: impl Sink<WsMessage, Error=WsError> + Unpin,
        game: Arc<GameData>,
    ) -> Result<()> {
        let schema = DownMessage::schema(&game);
        let mut coder_state_alloc = CoderStateAlloc::new();

        while let Some(msg) = recv_to_transmit.recv().await {
            // encode message
            let mut coder_state = CoderState::new(&schema, coder_state_alloc, None);
            let mut buf = Vec::new();
            msg.encode(&mut Encoder::new(&mut coder_state, &mut buf), &game)?;
            coder_state.is_finished_or_err()?;

            // send message
            ws_send.send(WsMessage::Binary(buf)).await?;

            // reset
            coder_state_alloc = coder_state.into_alloc();
        }

        Ok(())
    }

    // send task returns Ok if ends due to user dropping handle for sending
    // messages, returns Err if ends due to connection actually closing.
    let game_2 = Arc::clone(&game);
    let send_task = rt.spawn(async move {
        if let Err(e) = try_do_send_half(recv_to_transmit, ws_send, game_2).await {
            error!(%e, "connection send half error");
            Err(())
        } else {
            Ok(())
        }
    });
    let abort_send_task = send_task.abort_handle();
    let mut send_task = send_task.fuse();

    // just make this task be the receive half
    //
    // also, this task will be the only task that generates network events for
    // this connection. this prevents race conditions where an event could be
    // presented to the user after the event that removes this connection from
    // existence was already sent to the user.
    let schema = UpMessage::schema(&game);
    let mut coder_state_alloc = CoderStateAlloc::new();

    loop {
        select! {
            ws_event = ws_recv.next().fuse() => match ws_event {
                None => break,
                Some(Err(e)) => {
                    error!(%e, "ws connection error");
                    break;
                }
                Some(Ok(ws_msg)) => {
                    let buf = match ws_msg {
                        WsMessage::Binary(buf) => buf,
                        WsMessage::Text(_) => {
                            error!("receipt of text ws message");
                            break;
                        }
                        WsMessage::Ping(_) => continue,
                        WsMessage::Pong(_) => continue,
                        WsMessage::Close(_) => break,
                        WsMessage::Frame(_) => unreachable!(),
                    };

                    // binary websocket message received

                    // decode message
                    let mut coder_state = CoderState::new(&schema, coder_state_alloc, None);
                    let msg = match UpMessage::decode(
                        &mut Decoder::new(&mut coder_state, &mut &buf[..]),
                        &game,
                    ) {
                        Ok(msg) => msg,
                        Err(e) => {
                            error!(%e, "error decoding message");
                            break;
                        }
                    };

                    // deliver
                    let _ = send_event.send(NetworkEvent::Received(key, msg));

                    // reset
                    coder_state_alloc = coder_state.into_alloc();
                }
            },
            // shut down if the send half shuts down with error
            send_task_result = send_task => match send_task_result {
                Ok(Ok(())) => (),
                _ => break,
            },
        }
    }

    // do shutdown stuff now
    abort_send_task.abort();
    {
        // it's necessary for correctness that the slab changes the set of indexes
        // in the same order that the user is presented with those changes. thus
        // this lock must be held at least until the event is sent.
        let mut slab_guard = slab.lock();
        slab_guard.remove(key);
        let _ = send_event.send(NetworkEvent::Disconnected(key));
    }

    Ok(())
}
