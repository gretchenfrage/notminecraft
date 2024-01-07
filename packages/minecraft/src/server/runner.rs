//! Running the server.
//!
//! This is the top-level integration layer betweens server modules.

use crate::{
    game_content::*,
    server::{
        channel::*,
        save_content::*,
        save_db::SaveDb,
        save_mgr::{SaveMgr, ShouldSave},
        conn_mgr::ConnMgrEffect,
        chunk_mgr::ChunkMgrEffect,
        network::NetworkEvent,
        process_player_msg::process_player_msg,
        *,
    },
    message::*,
    thread_pool::ThreadPool,
};
use std::{
    sync::Arc,
    thread,
};
use vek::*;


/// Owned handle to a running server thread. Stops server when dropped.
pub struct ServerThread {
    server_send: ServerSender,
    network_server: NetworkServer,
}

impl ServerThread {
    /// Start a server in a new thread. Does _not_ bind.
    pub fn start(thread_pool: ThreadPool, save_db: SaveDb, game: Arc<GameData>) -> Self {
        let (server_send, server_recv) = channel();

    }
}


/// Run the server in this thread until it exits. Does _not_ run the network server.
pub fn run(
    server_send: ServerSender,
    server_recv: ServerReceiver,
    thread_pool: ThreadPool,
    save_db: SaveDb,
    game: Arc<GameData>,
) {
    // construct
    let mut server = Server {
        server_only: ServerOnlyState {
            server_send: server_send.clone(),
            server_recv,
            thread_pool: thread_pool.clone(),
            chunk_loader: ChunkLoader::new(
                Arc::clone(&game),
                server_send.clone(),
                thread_pool.clone(),
                save_db.clone(),
            ),
            player_save_state_loader: PlayerSaveStateLoader::new(
                server_send.clone(),
                thread_pool.clone(),
                save_db.clone(),
            ),
            player_pos: Default::default(),
            player_yaw: Default::default(),
            player_pitch: Default::default(),
        },
        sync_ctx: ServerSyncCtx {
            game,
            tick_mgr: Default::default(),
            chunk_mgr: Default::default(),
            save_mgr: SaveMgr::new(server_send, save_db, thread_pool),
            conn_mgr: Default::default(),
        },
        sync_state: ServerSyncState {
            tile_blocks: Default::default(),
        },
    };

    // other initialization
    request_load_spawn_chunks(&mut server);

    // enter event loop
    loop {
        // do tick
        do_tick(&mut server);
        maybe_save(&mut server);
        server.sync_ctx.tick_mgr.on_tick_done();
        
        // process events between ticks
        while let Some(event) = server.server_only.server_recv
            .recv(Some(server.sync_ctx.tick_mgr.next_tick()), None)
        {
            match event {
                // server shutdown requested
                ServerEvent::Stop => {
                    info!("server stopping (stop requested)");
                    return;
                },
                // network event
                ServerEvent::Network(event) => {
                    server.sync_ctx.conn_mgr.handle_network_event(event);
                    process_conn_mgr_effects(&mut server);
                }
                // player save state ready
                ServerEvent::PlayerSaveStateReady { pk, save_val } => {
                    server.sync_ctx.conn_mgr.on_player_save_state_ready(pk, save_val);
                }
                // chunk ready
                ServerEvent::ChunkReady { save_key, save_val, saved } => {
                    server.sync_ctx.chunk_mgr.on_chunk_ready(
                        save_key,
                        save_val,
                        saved,
                        server.sync_ctx.conn_mgr.players(),
                    );
                    process_chunk_mgr_effects(&mut server);
                }
                // save operation done
                ServerEvent::SaveOpDone => {
                    server.sync_ctx.save_mgr.on_save_op_done(server.sync_ctx.tick_mgr.tick());
                }
            }
        }
    }
}

// temporary, until we made chunk interest logic better
fn spawn_chunks() -> impl IntoIterator<Item=Vec3<i64>> {
    let mut ccs = Vec::new();
    for z in -8..8 {
        for y in 0..2 {
            for x in -8..8 {
                ccs.push(Vec3 { x, y, z });
            }
        }
    }
    ccs.sort_by_key(|cc| cc.map(|n| n * n).sum());
    ccs
}

// add chunk interests for chunks near spawn
fn request_load_spawn_chunks(server: &mut Server) {
    for cc in spawn_chunks() {
        server.sync_ctx.chunk_mgr.incr_load_request_count(cc, server.sync_ctx.conn_mgr.players());
        process_chunk_mgr_effects(server);
    }
}

// do a tick of world simulation
fn do_tick(server: &mut Server) {
    trace!("tick");
}

// do a save operation if appropriate to do so
fn maybe_save(server: &mut Server) {
    // ask whether should save
    let save_op = server.sync_ctx.save_mgr.maybe_save(server.sync_ctx.tick_mgr.tick());
    let mut save_op = match save_op {
        Some(save_op) => save_op,
        None => return,
    };

    // compile changed world state to be saved
    while let Some(should_save) = save_op.should_save.pop() {
        save_op.will_save.push(match should_save {
            ShouldSave::Chunk { cc, ci } => SaveEntry::Chunk(
                ChunkSaveKey { cc },
                ChunkSaveVal {
                    chunk_tile_blocks: server.sync_ctx.game
                        .clone_chunk_blocks(server.sync_state.tile_blocks.get(cc, ci))
                },
            ),
            ShouldSave::Player { pk } => SaveEntry::Player(
                PlayerSaveKey {
                    username: server.sync_ctx.conn_mgr.player_username(pk).clone(),
                },
                PlayerSaveVal {
                    pos: server.server_only.player_pos[pk],
                    yaw: server.server_only.player_yaw[pk],
                    pitch: server.server_only.player_pitch[pk],
                },
            ),
        });
    }

    // submit save operation
    save_op.submit();
}

// drain and process the conn mgr effect queue
fn process_conn_mgr_effects(server: &mut Server) {
    while let Some(effect) = server.sync_ctx.conn_mgr.effects.pop_front() {
        match effect {
            // initialize new player key, begin loading save state
            ConnMgrEffect::AddPlayerRequestLoad { pk, save_key, aborted } => {
                // add player
                server.sync_ctx.chunk_mgr.add_player(pk);
                for cc in spawn_chunks() {
                    server.sync_ctx.chunk_mgr.add_chunk_chunk_client_interest(
                        pk, cc, server.sync_ctx.conn_mgr.players()
                    );
                    process_chunk_mgr_effects(server);
                }

                // request load
                if let Some(save_val) = server.sync_ctx.save_mgr.take_unflushed_player(save_key) {
                    server.sync_ctx.conn_mgr.on_player_save_state_ready(pk, save_val);
                } else {
                    server.server_only.player_save_state_loader.trigger_load(pk, save_key, aborted);
                }
            }
            // upgrade player key to joined player key
            ConnMgrEffect::BeginJoinPlayer { pk, save_state } => {
                server.sync_ctx.save_mgr.join_player(pk, save_state.is_some());

                let pos = save_state.map(|val| val.pos).unwrap_or(Vec3::new(8.0, 8.0, 80.0));
                let yaw = save_state.map(|val| val.yaw).unwrap_or(0.0);
                let pitch = save_state.map(|val| val.yaw).unwrap_or(0.0);

                server.server_only.player_pos.insert(pk, pos);
                server.server_only.player_yaw.insert(pk, yaw);
                server.server_only.player_pitch.insert(pk, pitch);
            }
            // send player FinalizeJoinGame message
            ConnMgrEffect::FinalizeJoinPlayer { pk, self_clientside_player_idx } => {
                server.sync_ctx.conn_mgr.send(pk, DownMsg::FinalizeJoinGame(DownMsgFinalizeJoinGame {
                    self_player_idx: DownPlayerIdx(self_clientside_player_idx),
                    pos: server.server_only.player_pos[pk],
                    yaw: server.server_only.player_yaw[pk],
                    pitch: server.server_only.player_pitch[pk],
                }));
            }
            // add fully joined player to client
            ConnMgrEffect::AddPlayerToClient { add_to, to_add, clientside_player_idx } => {
                server.sync_ctx.conn_mgr.send(add_to, DownMsg::AddPlayer(DownMsgAddPlayer {
                    player_idx: DownPlayerIdx(clientside_player_idx),
                    pos: server.server_only.player_pos[pk],
                    yaw: server.server_only.player_yaw[pk],
                    pitch: server.server_only.player_pitch[pk],
                }));
            }
            // message from player
            ConnMgrEffect::PlayerMsg(pk, msg) => {
                // process
                process_player_msg(server.as_sync_world(), pk, msg);

                // ack
                if let Some(last_processed) = server.sync_ctx.conn_mgr.ack_last_processed(pk) {
                    server.sync_ctx.conn_mgr.send(pk, DownMsg::Ack { last_processed });
                }
            }
            // remove player
            ConnMgrEffect::RemovePlayer { pk, jpk, username } => {
                server.sync_ctx.chunk_mgr.remove_player(
                    pk, spawn_chunks(), server.sync_ctx.conn_mgr.connections(),
                );
                if let Some(jpk) = jpk {
                    server.sync_ctx.save_mgr.remove_player(
                        pk,
                        PlayerSaveKey { username },
                        PlayerSaveVal {
                            pos: server.server_only.player_pos[jpk],
                            yaw: server.server_only.player_yaw[jpk],
                            pitch: server.server_only.player_pitch[jpk],
                        },
                    );

                    server.server_only.player_pos.remove(jpk);
                    server.server_only.player_yaw.remove(jpk);
                    server.server_only.player_pitch.remove(jpk);
                }
            }
        }
    }
}

// drain and process the chunk mgr effect queue
fn process_chunk_mgr_effects(server: &mut Server) {
    while let Some(effect) = server.sync_ctx.chunk_mgr.effects.pop_front() {
        match effect {
            // begin loading / generating save state
            ChunkMgrEffect::RequestLoad { save_key, aborted } => {
                if let Some(save_val) = server.sync_ctx.save_mgr.take_unflushed_chunk(save_key) {
                    server.sync_ctx.chunk_mgr.on_chunk_ready(
                        save_key, save_val, false, server.sync_ctx.conn_mgr.players(),
                    );
                } else {
                    server.server_only.chunk_loader.trigger_load(save_key, aborted);
                }
            }
            // install loaded chunk into world
            ChunkMgrEffect::AddChunk { cc, ci, save_val, saved } => {
                server.sync_ctx.save_mgr.add_chunk(cc, ci, saved);
                server.sync_state.tile_blocks.insert(cc, ci, save_val.chunk_tile_blocks);
            }
            // remove chunk from the world
            ChunkMgrEffect::RemoveChunk { cc, ci } => {
                let chunk_tile_blocks = server.sync_state.tile_blocks.remove(cc, ci);
                server.sync_ctx.save_mgr.remove_chunk(
                    cc, ci, ChunkSaveKey { cc }, ChunkSaveVal { chunk_tile_blocks }
                );
            }
            // download chunk to client
            ChunkMgrEffect::AddChunkToClient { cc, ci, pk, clientside_ci } => {
                server.sync_ctx.conn_mgr.send(pk, DownMsg::AddChunk(DownMsgAddChunk {
                    chunk_idx: DownChunkIdx(clientside_ci),
                    cc,
                    chunk_tile_blocks: server.sync_ctx.game
                        .clone_chunk_blocks(server.sync_state.tile_blocks.get(cc, ci)),
                }));
            }
            // tell client to remove chunk
            ChunkMgrEffect::RemoveChunkFromClient { cc, ci, pk, clientside_ci } => {
                server.sync_ctx.conn_mgr.send(pk, DownMsg::RemoveChunk(DownMsgRemoveChunk {
                    chunk_idx: DownChunkIdx(clientside_ci),
                }));
            }
        }
    }
}
