//! Client-side connection handling.

use crate::{
    client_server::message::{
        UpMessage,
        DownMessage,
    },
    game_data::GameData,
    game_binschema::GameBinschema,
};
use binschema::{
    CoderStateAlloc,
    CoderState,
    Encoder,
    Decoder,
};
use std::{
    sync::Arc,
    marker::Unpin,
    time::Duration,
};
use anyhow::{Result, anyhow, bail};
use crossbeam_channel::{
    Sender,
    Receiver,
    TryRecvError,
    unbounded,
};
use tokio::{
    runtime::Handle,
    task::AbortHandle,
    sync::mpsc::{
        UnboundedSender as TokioUnboundedSender,
        UnboundedReceiver as TokioUnboundedReceiver,
        unbounded_channel as tokio_unbounded_channel,
    },
};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{
        client::IntoClientRequest,
        error::{
            Error as WsError,
            Result as WsResult,
        },
        handshake::client::Request as WsClientRequest,
        Message as WsMessage,
    },
};
use futures::{
    prelude::*,
    select,
};


#[derive(Debug)]
pub struct Connection {
    send_up: TokioUnboundedSender<UpMessage>,
    recv_down: Receiver<Result<DownMessage>>,
}

impl Connection {
    /// Asynchronously begin connecting and return immediately.
    pub fn connect(
        connect_to: impl IntoClientRequest,
        rt: &Handle,
        game: &Arc<GameData>,
    ) -> Self {
        // pre-do this part so connect_to can be non-static
        let connect_to = connect_to.into_client_request();

        let (send_up, recv_up) = tokio_unbounded_channel();
        let (send_down, recv_down) = unbounded();

        let rt_2 = Handle::clone(rt);
        let game = Arc::clone(game);
        rt.spawn(async move {
            // spawn a task to try and run the connection
            let mut abort_send_task = None;
            let result = try_run_connection(
                connect_to,
                rt_2,
                game,
                &mut abort_send_task,
                Sender::clone(&send_down),
                recv_up,
            ).await;
            
            // if it terminated with error, rather than by the Connection handle
            // being dropped, send the error. only do this once.
            if let Err(e) = result {
                let _ = send_down.send(Err(e));
            }

            // finally, if an send task was spawned, abort it, just in case
            // terminated of this task was caused in some way that didn't itself
            // promptly terminate the send task
            if let Some(abort_send_task) = abort_send_task {
                abort_send_task.abort();
            }
        });

        Connection {
            send_up,
            recv_down,
        }
    }

    /// Asynchronously queue a message for sending, return immediately.
    pub fn send(&self, msg: UpMessage) {
        let _ = self.send_up.send(msg);
    }

    /// Check for an asynchronously received message or error without blocking,
    /// return with result immediately. Error will be received only once, at end
    /// if connection dies or errors somehow.
    pub fn poll(&self) -> Result<Option<DownMessage>> {
        match self.recv_down.try_recv() {
            Ok(result) => result.map(Some),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => {
                panic!("unexpected ws client recv_down disconnection");
            }
        }
    }
}

/// Returns with error on connection error, or Ok if stopped due to the
/// Connection handle being dropped.
async fn try_run_connection(
    connect_to: WsResult<WsClientRequest>,
    rt: Handle,
    game: Arc<GameData>,
    abort_send_task: &mut Option<AbortHandle>,
    send_down: Sender<Result<DownMessage>>,
    recv_up: TokioUnboundedReceiver<UpMessage>,
) -> Result<()> {
    // TODO this is temporary
    tokio::time::sleep(Duration::from_secs(1)).await;

    // connect
    let (ws, _) = connect_async(connect_to?).await?;
    let (ws_send, mut ws_recv) = ws.split();

    // spawn task to handle the send half
    let send_task = rt.spawn(try_do_send_half(recv_up, ws_send, Arc::clone(&game)));
    *abort_send_task = Some(send_task.abort_handle());
    let mut send_task = send_task.fuse();

    // just make this task be the receive half
    let schema = DownMessage::schema(&game);
    let mut coder_state_alloc = CoderStateAlloc::new();

    loop {
        select! {
            ws_event = ws_recv.next().fuse() => {
                let ws_msg = ws_event
                    .transpose()?
                    .ok_or_else(|| anyhow!("connection closing (stream produced None"))?;
                let buf = match ws_msg {
                    WsMessage::Binary(buf) => buf,
                    WsMessage::Text(_) => bail!("receipt of ws text message"),
                    WsMessage::Ping(_) => continue,
                    WsMessage::Pong(_) => continue,
                    WsMessage::Close(_) => bail!("connection closing (received close frame)"),
                    WsMessage::Frame(_) => unreachable!(),
                };

                // binary websocket message received
                // decode message
                let mut coder_state = CoderState::new(&schema, coder_state_alloc, None);
                let msg =DownMessage::decode(
                    &mut Decoder::new(&mut coder_state, &mut &buf[..]),
                    &game,
                )?;

                // deliver to user
                let _ = send_down.send(Ok(msg));

                // reset
                coder_state_alloc = coder_state.into_alloc();
            },
            // if send task terminated, we terminate in the same way
            send_task_result = send_task => {
                return send_task_result.expect("unexpected send task abort");
            }
        }
    }
}

/// Returns with error on connection error, or Ok if stopped due to the
/// Connection handle being dropped.
async fn try_do_send_half(
    mut recv_up: TokioUnboundedReceiver<UpMessage>,
    mut ws_send: impl Sink<WsMessage, Error=WsError> + Unpin,
    game: Arc<GameData>,
) -> Result<()> {
    let schema = UpMessage::schema(&game);
    let mut coder_state_alloc = CoderStateAlloc::new();

    while let Some(msg) = recv_up.recv().await {
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
