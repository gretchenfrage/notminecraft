//! Integration code for pre-join logic only.

use crate::{
    client::{
        channel::*,
        network::*,
        gui_state::ClientGuiState,
        *,
    },
    message::*,
    gui_state_loading::LoadingOneshot,
    server::{
        runner::ServerThread,
        save_db::SaveDb,
    },
    gui::prelude::*,
};
use chunk_data::*;
use get_assets::DataDir;
use tokio::runtime::Handle;
use std::{
    thread::spawn,
    sync::Arc,
    time::{Instant, Duration},
};
use crossbeam::queue::ArrayQueue;
use anyhow::Result;


#[derive(Debug)]
pub enum ServerLocation {
    Internal {
        save_name: String,
        data_dir: DataDir,
    },
    External {
        url: String,
        rt: Handle,
    },
}

/// Spawn a thread to join the game in the background.
pub fn join_in_background(
    game: Arc<GameData>,
    thread_pool: ThreadPool,
    server_loc: ServerLocation,
    log_in: UpMsgLogIn,
    gpu_vec_ctx: AsyncGpuVecContext,
) -> Box<dyn LoadingOneshot> {
    let (client_send_1, client_recv) = channel();
    let client_send_2 = client_send_1.clone();
    let oneshot_1 = Arc::new(ArrayQueue::new(1));
    let oneshot_2 = Arc::clone(&oneshot_1);
    spawn(move || {
        let (server, connection) = match server_loc {
            ServerLocation::Internal { save_name, data_dir } => {
                info!("starting internal server");
                let save_db = SaveDb::open(&save_name, &data_dir, &game)
                    .unwrap_or_else(|e| {
                        error!(%e, "failed to open save file");
                        todo!("handle this")
                    });
                let server = ServerThread::start(thread_pool.clone(), save_db, Arc::clone(&game));
                let connection = server.network_handle().in_mem_client(client_send_1.clone());
                (Some(server), Connection::in_mem(connection))
            },
            ServerLocation::External { url, rt } => {
                info!(?url, "connecting to server");
                (None, Connection::connect(&url, client_send_1.clone(), &rt, &game))
            },
        };
        info!("logging in");
        connection.send(UpMsg::LogIn(log_in));
        loop {
            let event = client_recv.poll_blocking();
            trace!(?event, "client event (logging in)");
            match event {
                ClientEvent::AbortInit => {
                    info!("client initialization aborted");
                    return;
                }
                ClientEvent::Network(event) => match event {
                    NetworkEvent::Received(msg) => match msg {
                        DownMsg::AcceptLogIn => {
                            break;
                        }
                        _ => {
                            error!("server protocol violation");
                            todo!("handle this");
                        }
                    }
                    NetworkEvent::Closed(msg) => {
                        error!(?msg, "server connection closed");
                        todo!("handle this");
                    }
                }
                _ => unreachable!(),
            }
        }
        info!("loading world");
        let mut client = PreJoinClient {
            game: Arc::clone(&game),
            client_send: client_send_1.clone(),
            client_recv,
            connection,
            server,
            thread_pool: thread_pool.clone(),
            gpu_vec_ctx: gpu_vec_ctx.clone(),
            chunks: Default::default(),
            tile_blocks: Default::default(),
            chunk_mesh_mgr: ChunkMeshMgr::new(
                game,
                client_send_1,
                thread_pool,
                gpu_vec_ctx,
            ),
            players: Default::default(),
            player_username: Default::default(),
            player_pos: Default::default(),
            player_yaw: Default::default(),
            player_pitch: Default::default(),
        };
        let mut flushed = Instant::now();
        let mut joining = false;
        let finalize_msg = loop {
            let event = match client.client_recv.poll() {
                Some(event) => event,
                None => {
                    trace!("flushing chunk mesh (about to block)");
                    client.chunk_mesh_mgr.flush_dirty(&client.chunks, &client.tile_blocks);
                    flushed = Instant::now();
                    client.client_recv.poll_blocking()
                }
            };
            if Instant::now() - flushed > Duration::from_millis(10) {
                trace!("flushing chunk mesh (period elapsed)");
                client.chunk_mesh_mgr.flush_dirty(&client.chunks, &client.tile_blocks);
                flushed = Instant::now();
            }

            trace!(?event, "client event (pre-join)");

            match event {
                ClientEvent::AbortInit => return,
                ClientEvent::Network(event) => match event {
                    NetworkEvent::Received(msg) => match msg {
                        DownMsg::PreJoin(msg) => {
                            let result = process_pre_join_msg(&mut client, msg);
                            if let Err(e) = result {
                                error!(%e, "server protocol violation");
                                todo!("handle this")
                            }
                        },
                        DownMsg::ShouldJoinGame => {
                            info!("joining game");
                            client.connection.send(UpMsg::JoinGame);
                            joining = true;
                        }
                        DownMsg::FinalizeJoinGame(msg) if joining => {
                            break msg;
                        }
                        msg => {
                            error!(?msg, "invalid msg type from server at this time (pre-join)");
                            todo!("handle this");
                        }
                    }
                    NetworkEvent::Closed(msg) => {
                        error!(?msg, "server connection closed");
                        todo!("handle this");
                    }
                }
                ClientEvent::ChunkMeshed { cc, ci, chunk_mesh } => {
                    client.connection.send(UpMsg::PreJoin(PreJoinUpMsg::AcceptMoreChunks(1)));
                    client.chunk_mesh_mgr
                        .on_chunk_meshed(cc, ci, chunk_mesh, &client.chunks, &client.tile_blocks);
                }
            }
        };
        info!("finalizing joining game");
        let DownMsgFinalizeJoinGame { self_player_idx } = finalize_msg;
        let self_pk = match client.players.lookup(self_player_idx) {
            Ok(pk) => pk,
            Err(e) => {
                error!(%e, "server protocol violation");
                todo!("handle this");
            },
        };
        let client = Client {
            pre_join: client,
            self_pk,
            pos: Vec3::new(16.0, 48.0, 16.0),
            yaw: f32::to_radians(45.0),
            pitch: f32::to_radians(45.0),
            menu_mgr: Default::default(),
        };
        let _ = oneshot_1.push(Box::new(ClientGuiState(client)));
    });
    struct ClientLoadingOneshot {
        oneshot: Arc<ArrayQueue<Box<ClientGuiState>>>,
        client_send: ClientSender,
    }
    impl LoadingOneshot for ClientLoadingOneshot {
        fn poll(&mut self) -> Option<Box<dyn GuiStateFrameObj>> {
            self.oneshot.pop().map(|b| b as _)
        }
    }
    impl Drop for ClientLoadingOneshot {
        fn drop(&mut self) {
            self.client_send.send(ClientEvent::AbortInit, EventPriority::Control, None, None);
        }
    }
    Box::new(ClientLoadingOneshot { oneshot: oneshot_2, client_send: client_send_2 })
}


/// Process a pre join msg received from the network. Error indicates server protocol violation.
pub fn process_pre_join_msg(client: &mut PreJoinClient, msg: PreJoinDownMsg) -> Result<()> {
    match msg {
        // add player to world
        PreJoinDownMsg::AddPlayer(DownMsgAddPlayer {
            player_idx,
            username,
            pos,
            pitch,
            yaw,
        }) => {
            let pk = client.players.on_add_player(player_idx)?;
            client.player_username.insert(pk, username);
            client.player_pos.insert(pk, pos);
            client.player_pitch.insert(pk, pitch);
            client.player_yaw.insert(pk, yaw);
        }
        // remove player from world
        PreJoinDownMsg::RemovePlayer(DownMsgRemovePlayer { player_idx }) => {
            let pk = client.players.on_remove_player(player_idx)?;
            client.player_username.remove(pk);
            client.player_pos.remove(pk);
            client.player_pitch.remove(pk);
            client.player_yaw.remove(pk);
        }
        // add chunk to world
        PreJoinDownMsg::AddChunk(DownMsgAddChunk { chunk_idx, cc, chunk_tile_blocks }) => {
            let (ci, _getter) = client.chunks.on_add_chunk(chunk_idx, cc)?.get(&client.chunks);
            client.tile_blocks.add(cc, ci, chunk_tile_blocks);
            client.chunk_mesh_mgr.add_chunk(cc, ci, &client.chunks, &client.tile_blocks);
        }
        // remove chunk from world
        PreJoinDownMsg::RemoveChunk(DownMsgRemoveChunk { chunk_idx }) => {
            let (cc, ci) = client.chunks.on_remove_chunk(chunk_idx)?;
            client.tile_blocks.remove(cc, ci);
            client.chunk_mesh_mgr.remove_chunk(cc, ci);
        }
        // apply edit
        PreJoinDownMsg::ApplyEdit(edit) => match edit {
            // set tile block
            Edit::SetTileBlock { chunk_idx, lti, bid_meta } => {
                let (cc, ci, getter) = client.chunks.lookup(chunk_idx)?;
                let tile = TileKey { cc, ci, lti };
                tile.get(&mut client.tile_blocks).erased_set(bid_meta);
                client.chunk_mesh_mgr.mark_adj_dirty(&getter, tile.gtc());
            }
            // set char state
            Edit::SetPlayerCharState { player_idx, pos, yaw, pitch } => {
                let pk = client.players.lookup(player_idx)?;
                client.player_pos[pk] = pos;
                client.player_yaw[pk] = yaw;
                client.player_pitch[pk] = pitch;
            }
        }
    }
    Ok(())
}
