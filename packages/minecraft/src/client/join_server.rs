//! Connecting to and joining the server, thereby initializing the client.

use crate::{
    client::{
        channel::*,
        network::*,
        process_msg::process_pre_join_msg,
        gui_state::ClientGuiState,
        mesh_item::create_item_meshes,
        *,
    },
    server::{
        runner::ServerThread,
        save_db::SaveDb,
    },
    message::*,
    gui::prelude::*,
    gui_state_loading::LoadingOneshot,
    gui_state_loading_failure::LoadingFailureMenu,
    sync_state_steve,
};
use get_assets::DataDir;
use std::{
    thread::spawn,
    sync::Arc,
    time::{Instant, Duration},
};
use tokio::{
    sync::Notify,
    runtime::Handle,
};
use futures::{
    FutureExt as _,
    select_biased
};
use crossbeam::queue::ArrayQueue;
use anyhow::*;


/// Description of how to connect to a server when initializing a client.
#[derive(Debug)]
pub enum ServerLocation {
    /// Create an internal server by opening the given save file and connect to it.
    Internal {
        save_name: String,
        data_dir: DataDir,
    },
    /// Connect to an external server over the network.
    External {
        url: String,
        rt: Handle,
    },
}

/// Spawn a thread to fully connect to and join a server in the background as a client.
///
/// The returned handle can be used to poll for the constructed gui state, and aborts
/// the joining process and disconnects from the server if dropped.
pub fn spawn_join_server_thread(
    server_location: ServerLocation,
    game: &Arc<GameData>,
    thread_pool: &ThreadPool,
    log_in_msg: UpMsgLogIn,
    gpu_vec_ctx: AsyncGpuVecContext,
) -> Box<dyn LoadingOneshot> {
    let (client_send_1, client_recv) = channel();
    let client_send_2 = client_send_1.clone();
    let oneshot_1 = Arc::new(ArrayQueue::new(1));
    let oneshot_2 = Arc::clone(&oneshot_1);
    let cancel_connect_1 = Arc::new(Notify::new());
    let cancel_connect_2 = Arc::clone(&cancel_connect_1);
    let game = Arc::clone(game);
    let thread_pool = thread_pool.clone();
    spawn(move || {
        let result = join_server(
            server_location,
            game,
            thread_pool,
            log_in_msg,
            client_send_1,
            client_recv,
            gpu_vec_ctx,
            cancel_connect_1,
        );
        if let Err(e) = result.as_ref() {
            error!(%e, "error joining server");
        }
        let _ = oneshot_1.push(result);
    });
    struct ClientLoadingOneshot {
        oneshot: Arc<ArrayQueue<Result<Client>>>,
        client_send: ClientSender,
        // used to possibly cancel the Connection::connect future
        cancel_connect: Arc<Notify>,
    }
    impl LoadingOneshot for ClientLoadingOneshot {
        fn poll(&mut self, ctx: &GuiGlobalContext) -> Option<Box<dyn GuiStateFrameObj>> {
            self.oneshot.pop().map(|result| result
                .map(|client| Box::new(ClientGuiState(client)) as _)
                .unwrap_or_else(|e| Box::new(LoadingFailureMenu::new(ctx, e)) as _))
        }
    }
    impl Drop for ClientLoadingOneshot {
        fn drop(&mut self) {
            self.client_send.send(ClientEvent::AbortInit, EventPriority::Control, None, None);
            self.cancel_connect.notify_one();
        }
    }
    Box::new(ClientLoadingOneshot {
        oneshot: oneshot_2,
        client_send: client_send_2,
        cancel_connect: cancel_connect_2,
    })
}

// fully connect to and join a server, blocking until success or error
fn join_server(
    server_location: ServerLocation,
    game: Arc<GameData>,
    thread_pool: ThreadPool,
    log_in_msg: UpMsgLogIn,
    client_send: ClientSender,
    client_recv: ClientReceiver,
    gpu_vec_ctx: AsyncGpuVecContext,
    cancel_connect: Arc<Notify>,
) -> Result<Client> {
    let (connection, server) =
        connect_to_server(server_location, &game, &thread_pool, &client_send, cancel_connect)?;
    log_in(&connection, &client_recv, log_in_msg)?;
    let mut client = construct_pre_join_client(
        game,
        client_send,
        client_recv,
        connection,
        server,
        thread_pool,
        gpu_vec_ctx,
    );
    let finalize_join_game_msg = load_world(&mut client)?;
    finalize_join_game(client, finalize_join_game_msg)
}

// resolve the server location, starting the internal server if necessary
fn connect_to_server(
    server_location: ServerLocation,
    game: &Arc<GameData>,
    thread_pool: &ThreadPool,
    client_send: &ClientSender,
    cancel_connect: Arc<Notify>,
) -> Result<(Connection, Option<ServerThread>)> {
    Ok(match server_location {
        // start internal server
        ServerLocation::Internal { save_name, data_dir } => {
            info!(?save_name, "starting internal server");
            let save_db = SaveDb::open(&save_name, &data_dir, game)
                .context("failed to open save file")?;
            let server = ServerThread::start(thread_pool.clone(), save_db, Arc::clone(game));
            let connection = server.network_handle().in_mem_client(client_send.clone());
            (Connection::in_mem(connection), Some(server))
        }
        // connect to external server
        ServerLocation::External { url, rt: rt_1 } => {
            info!(?url, "connecting to server");
            let rt_2 = rt_1.clone();
            let connection = rt_1.block_on(async move {
                let connect = Connection::connect(&url, client_send.clone(), &rt_2, game);
                select_biased! {
                    _ = cancel_connect.notified().fuse() => Err(anyhow!("cancelled")),
                    result = connect.fuse() => result,
                }
            })?;
            (connection, None)
        }
    })
}

// try send the LogIn message and receive the AcceptLogIn message
fn log_in(
    connection: &Connection,
    client_recv: &ClientReceiver,
    log_in_msg: UpMsgLogIn,
) -> Result<()> {
    info!("logging in");
    connection.send(UpMsg::LogIn(log_in_msg));
    Ok(loop {
        let event = client_recv.poll_blocking();
        trace!(?event, "client event (logging in)");
        match event {
            ClientEvent::AbortInit => bail!("client initialization aborted"),
            ClientEvent::Network(event) => match event {
                NetworkEvent::Received(msg) => match msg {
                    DownMsg::AcceptLogIn => break, 
                    _ => bail!("server protocol violation"),
                }
                NetworkEvent::Closed(msg) => bail!("server connection closed: {:?}", msg),
            }
            _ => unreachable!(),
        }
    })
}

// construct a pre-join client in the starting state
fn construct_pre_join_client(
    game: Arc<GameData>,
    client_send: ClientSender,
    client_recv: ClientReceiver,
    connection: Connection,
    server: Option<ServerThread>,
    thread_pool: ThreadPool,
    gpu_vec_ctx: AsyncGpuVecContext,
) -> PreJoinClient {
    PreJoinClient {
        game: Arc::clone(&game),
        client_send: client_send.clone(),
        client_recv,
        connection,
        server,
        thread_pool: thread_pool.clone(),
        gpu_vec_ctx: gpu_vec_ctx.clone(),
        item_mesh: create_item_meshes(&game, &gpu_vec_ctx),
        chunks: Default::default(),
        tile_blocks: Default::default(),
        chunk_mesh_mgr: ChunkMeshMgr::new(game, client_send, thread_pool, gpu_vec_ctx),
        players: Default::default(),
        player_username: Default::default(),
        player_pos: Default::default(),
        player_yaw: Default::default(),
        player_pitch: Default::default(),

        /*global_entity_hmap: Default::default(),
        global_entity_slab: Default::default(),
        chunk_steves: Default::default(),
        chunk_pigs: Default::default(),*/
        //steves: Default::default(),
        entities: Default::default(),
        chunk_steves: Default::default(),
        chunk_pigs: Default::default(),
    }
}

// try to receive and process pre-join messages interleaved with and until completing the sequence
// of: receive ShouldJoinGame -> send JoinGame -> receive FinalizeJoinGame
fn load_world(client: &mut PreJoinClient) -> Result<DownMsgFinalizeJoinGame> {
    let mut flushed = Instant::now();
    let mut joining = false;
    Ok(loop {
        // try to get event without blocking
        let now = Instant::now();
        let event = client.client_recv.poll()
            .unwrap_or_else(|| {
                // fall back to blocking, but always flush chunk mesh before doing so
                trace!("flushing chunk mesh (about to block)");
                client.chunk_mesh_mgr.flush_dirty(&client.chunks, &client.tile_blocks);
                flushed = now;
                client.client_recv.poll_blocking()
            });
        // make sure to also flush intermittently even (especially) if never blocking for events
        if now - flushed > Duration::from_millis(10) {
            trace!("flushing chunk mesh (period elapsed");
            client.chunk_mesh_mgr.flush_dirty(&client.chunks, &client.tile_blocks);
            flushed = now;
        }

        // process the event
        trace!(?event, "client event (pre-join)");
        match event {
            ClientEvent::AbortInit => bail!("client initialization aborted"),
            ClientEvent::Network(event) => match event {
                NetworkEvent::Received(msg) => match msg {
                    DownMsg::PreJoin(msg) =>
                        process_pre_join_msg(client, msg).context("server protocol violation")?,
                    DownMsg::ShouldJoinGame => {
                        ensure!(!joining, "server protocol violation");
                        info!("joining game");
                        joining = true;
                        client.connection.send(UpMsg::JoinGame);
                    }
                    DownMsg::FinalizeJoinGame(msg) => {
                        ensure!(joining, "server protocol vioaltion");
                        break msg;
                    }
                    _ => bail!("server protocol violation"),
                }
                NetworkEvent::Closed(msg) => bail!("server connection closed: {:?}", msg),
            }
            ClientEvent::ChunkMeshed { cc, ci, chunk_mesh } => {
                client.connection.send(UpMsg::PreJoin(PreJoinUpMsg::AcceptMoreChunks(1)));
                client.chunk_mesh_mgr
                    .on_chunk_meshed(cc, ci, chunk_mesh, &client.chunks, &client.tile_blocks);
            }
        }
    })
}

// do the final step of constructing the interactive client
fn finalize_join_game(
    client: PreJoinClient,
    finalize_join_game_msg: DownMsgFinalizeJoinGame,
) -> Result<Client> {
    let DownMsgFinalizeJoinGame {
        self_player_idx,
        inventory_slots,
        held_slot,
    } = finalize_join_game_msg;
    let self_pk = client.players.lookup(self_player_idx).context("server protocol violation")?;
    let steve_mesh = {
        let mut mesh_buf = MeshData::new();
        use mesh_data::*;
        for face in FACES {
            let (pos_start, [pos_ext_1, pos_ext_2]) = face.quad_start_extents();
            let steve_ext = Vec3::new(
                sync_state_steve::STEVE_WIDTH,
                sync_state_steve::STEVE_HEIGHT,
                sync_state_steve::STEVE_WIDTH,
            );
            let mut pos_start = pos_start.to_poles().map(|pole| pole.to_int() as f32) * (steve_ext / 2.0);
            pos_start.y += sync_state_steve::STEVE_HEIGHT / 2.0;
            mesh_buf.add_quad(&Quad {
                pos_start,
                pos_ext_1: (pos_ext_1.to_signs().map(|sign| sign.to_int() as f32) * steve_ext).into(),
                pos_ext_2: (pos_ext_2.to_signs().map(|sign| sign.to_int() as f32) * steve_ext).into(),
                tex_start: 0.0.into(),
                tex_extent: 1.0.into(),
                vert_colors: [Rgba::white(); 4],
                tex_index: 0,
            });
        }
        mesh_buf.upload(&client.gpu_vec_ctx)
    };
    Ok(Client {
        pre_join: client,
        self_pk,
        pos: Vec3::new(16.0, 48.0, 16.0),
        yaw: f32::to_radians(45.0),
        pitch: f32::to_radians(45.0),
        menu_mgr: Default::default(),
        inventory_slots: sync_state_inventory_slots::PlayerInventorySlots {
            inventory_slots,
            held_slot,
        },
        steve_mesh,
    })
}
