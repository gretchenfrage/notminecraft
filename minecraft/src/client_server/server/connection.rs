//! Server-side connection handling system.

use crate::{
    game_data::GameData,
    client_server::{
        message::*,
        server::event::EventSender,
        client,
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
    sync::{
        Arc,
        atomic::{
            AtomicU64,
            Ordering,
        },
    },
    marker::Unpin,
};
use anyhow::{Result, anyhow};
use crossbeam_channel::{
    Sender,
    Receiver,
    unbounded,
    TryRecvError,
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
#[derive(Debug)]
pub enum NetworkEvent {
    NewConnection(usize, Connection),
    Disconnected(usize),
    Received(usize, UpMessage),
}

/// Handle for sending messages down a single network connection. Received
/// messages are centrally serialized through `NetworkServer` to facilitate
/// a single-threaded usage pattern.
#[derive(Debug)]
pub struct Connection {
    send: SendDown,
}

#[derive(Debug)]
enum SendDown {
    Network(TokioUnboundedSender<DownMessage>),
    InMem(Sender<DownMessage>),
}

impl Connection {
    /// Queue message to be transmitted to client and return immediately.
    pub fn send(&self, msg: impl Into<DownMessage>) {
        match self.send {
            SendDown::Network(ref inner) => {
                let _ = inner.send(msg.into());
            }
            SendDown::InMem(ref inner) => {
                let _ = inner.send(msg.into());
            }
        }
    }
}

/// Handle to the asynchronous system that serves network connections. Source
/// of network events.
pub struct NetworkServer {
    send_event: EventSender<NetworkEvent>,
    // the slab is used to assign connection keys. it is also used to serialize
    // changes to the set of connection keys. when the set of connection keys
    // changes, the thread that changes that must lock the slab, make the change,
    // and then make sure to send the network event which tells the user about
    // that changes before unlocking the slab. unlocking the slab too early could
    // result in the user receiving events in the wrong order, which could cause
    // all sorts of problemos.
    slab: Arc<Mutex<Slab<()>>>,
}

/// It is possible to construct a `NetworkServer` without actually opening it
/// to the network immediately. It is possible to then, at some later point, open
/// it to the network. That's what this is for.
pub struct NetworkServerOpener {
    send_event: EventSender<NetworkEvent>,
    slab: Arc<Mutex<Slab<()>>>,
}

impl NetworkServer {
    /// Construct a `NetworkServer` and a corresponding `NetworkServerOpener`. The
    /// network server will not actually spawn tasks that start serving network
    /// connections until the opener is called upon to open it. The convenience
    /// `NetworkServer::open` method does that automatically.
    pub fn new(send_event: EventSender<NetworkEvent>) -> (Self, NetworkServerOpener) {
        let slab = Arc::new(Mutex::new(Slab::new()));
        (
            NetworkServer {
                send_event: send_event.clone(),
                slab: Arc::clone(&slab),
            },
            NetworkServerOpener {
                send_event,
                slab,
            }
        )
    }

    /// Convenience method to construct a `NetworkServer` and spawn tasks that
    /// opens it up to the network.
    pub fn new_networked(
        send_event: EventSender<NetworkEvent>,
        bind_to: impl ToSocketAddrs + Send + Sync + 'static,
        rt: &Handle,
        game: &Arc<GameData>,
    ) -> Self {
        let (server, opener) = Self::new(send_event);
        opener.open(bind_to, rt, game);
        server
    }

    /// Convenience method to construct a `NetworkServer` that isn't opened up
    /// to a network along with a client `Connection` linked up in-memory.
    pub fn new_internal(
        send_event: EventSender<NetworkEvent>,
    ) -> (Self, client::connection::Connection) {
        let (server, _) = Self::new(send_event);
        let client = server.create_in_mem_client().into();
        (server, client)
    }
    
    /// Create a new in-memory network client that just passes its messages
    /// over a channel without even transcoding them.
    pub fn create_in_mem_client(&self) -> InMemClient {
        let (send_down, recv_down) = unbounded();

        let mut slab_guard = self.slab.lock();
        let key = slab_guard.insert(());
        let connection = Connection {
            send: SendDown::InMem(send_down),
        };
        let _ = self.send_event.send(NetworkEvent::NewConnection(key, connection));
        drop(slab_guard); // take care to send event before unlocking

        InMemClient {
            key,
            send_event: self.send_event.clone(),
            slab: Arc::clone(&self.slab),
            recv: recv_down,
        }
    }
}

/// A client connection to a `NetworkServer` in the same process that doesn't
/// actually use the network.
#[derive(Debug)]
pub struct InMemClient {
    key: usize,
    send_event: EventSender<NetworkEvent>,
    slab: Arc<Mutex<Slab<()>>>,
    recv: Receiver<DownMessage>,
}

impl InMemClient {
    pub fn send(&self, msg: UpMessage) {
        let _ = self.send_event.send(NetworkEvent::Received(self.key, msg));
    }

    pub fn poll(&self) -> Result<Option<DownMessage>> {
        match self.recv.try_recv() {
            Ok(msg) => Ok(Some(msg)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(anyhow!("internal server disconnected")),
        }
    }
}

impl Drop for InMemClient {
    fn drop(&mut self) {
        let mut slab_guard = self.slab.lock();
        slab_guard.remove(self.key);
        let _ = self.send_event.send(NetworkEvent::Disconnected(self.key));
        drop(slab_guard); // take care to send event before unlocking
    }
}

impl NetworkServerOpener {
    /// Spawn tasks onto the runtime that bind to the port and begin serving
    /// network connections on it.
    pub fn open(
        self,
        bind_to: impl ToSocketAddrs + Send + Sync + 'static,
        rt: &Handle,
        game: &Arc<GameData>,
    ) {
        let NetworkServerOpener { send_event, slab } = self;

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

            // accept connections
            loop {
                match listener.accept().await {
                    Ok((tcp, _)) => {
                        let slab = Arc::clone(&slab);
                        let rt = Handle::clone(&rt_2);
                        let send_event = send_event.clone();
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
    }
}

async fn handle_tcp_connection(
    tcp: TcpStream,
    slab: Arc<Mutex<Slab<()>>>,
    rt: &Handle,
    send_event: EventSender<NetworkEvent>,
    game: Arc<GameData>,
) -> Result<()> {
    // do the handshake to upgrade it to websocket
    let ws = accept_async(tcp).await?;

    // initialize the connection, emit an event
    let (send_to_transmit, recv_to_transmit) = tokio_unbounded_channel();
    let key;
    {
        let mut slab_guard = slab.lock();
        key = slab_guard.insert(());
        let connection = Connection {
            send: SendDown::Network(send_to_transmit),
        };
        let _ = send_event.send(NetworkEvent::NewConnection(key, connection));
        drop(slab_guard); // take care to send event before unlocking
    }

    // split it into sink half and stream half
    let (ws_send, mut ws_recv) = ws.split();

    // create the "accept more chunks budget"
    //
    // it's kinda awkward to be handling this here but whatever. the client is
    // not allowed to send more AcceptMoreChunks than it has received AddChunk,
    // and we make sure to enforce this because otherwise a client could
    // perform a slow read denial of service attack where it keeps requesting
    // more chunks without actually reading them from the network connection
    // causing recv_to_transmit to fill up until the server runs out of memory
    // at disproportionately little cost to the client.
    let accept_more_chunks_budget_1 = Arc::new(AtomicU64::new(0));
    let accept_more_chunks_budget_2 = Arc::clone(&accept_more_chunks_budget_1);

    // spawn a new task to handle the send half
    async fn try_do_send_half(
        mut recv_to_transmit: TokioUnboundedReceiver<DownMessage>,
        mut ws_send: impl Sink<WsMessage, Error=WsError> + Unpin,
        accept_more_chunks_budget: Arc<AtomicU64>,
        game: Arc<GameData>,
    ) -> Result<()> {
        let schema = DownMessage::schema(&game);
        info!("DownMessage schema:\n{}", schema.pretty_fmt());
        let mut coder_state_alloc = CoderStateAlloc::new();
        //let mut dbg_buf = Vec::new();

        while let Some(msg) = recv_to_transmit.recv().await {
            if matches!(msg, DownMessage::AddChunk(_)) {
                accept_more_chunks_budget.fetch_add(1, Ordering::SeqCst);
            }

            // encode message
            let mut coder_state = CoderState::new(&schema, coder_state_alloc, None);
            let mut buf = Vec::new();
            msg.encode(&mut Encoder::new(&mut coder_state, &mut buf), &game)?;
            coder_state.is_finished_or_err()?;

            // reset coder state
            coder_state_alloc = coder_state.into_alloc();
  
            if buf.len() < 16 {
                //debug!("sending down {} bytes:\n{}", buf.len(), str::from_utf8(&dbg_buf).unwrap());
                trace!(?msg, "sending down {} bytes", buf.len());
            } else {
                trace!("sending down {} bytes", buf.len());
            }

            //dbg_buf.clear();

            // send message
            ws_send.send(WsMessage::Binary(buf)).await?;

        }

        Ok(())
    }

    // send task returns Ok if ends due to user dropping handle for sending
    // messages, returns Err if ends due to connection actually closing.
    let game_2 = Arc::clone(&game);
    let send_task = rt.spawn(async move {
        if let Err(e) = try_do_send_half(
            recv_to_transmit,
            ws_send,
            accept_more_chunks_budget_1,
            game_2,
        ).await {
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

                    if let &UpMessage::AcceptMoreChunks(up::AcceptMoreChunks { number }) = &msg {
                        let prev_budget = accept_more_chunks_budget_2
                            .fetch_sub(number as u64, Ordering::SeqCst);
                        if number as u64 > prev_budget {
                            error!("AcceptMoreChunks message exceeded allowed values");
                            break;
                        }
                    }

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
    let mut slab_guard = slab.lock();
    slab_guard.remove(key);
    let _ = send_event.send(NetworkEvent::Disconnected(key));
    drop(slab_guard); // take care to send event before unlocking

    Ok(())
}
