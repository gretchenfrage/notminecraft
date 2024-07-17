//! Running the server.
//!
//! This is the top-level integration layer betweens server modules.

use crate::{
    game_data::*,
    server::{
        channel::*,
        network::*,
        save_content::*,
        save_db::SaveDb,
        save_mgr::{SaveMgr, ShouldSave},
        conn_mgr::ConnMgrEffect,
        chunk_mgr::ChunkMgrEffect,
        tick_mgr::TICK,
        process_player_msg::process_player_msg,
        *,
    },
    message::*,
    thread_pool::ThreadPool,
    util_must_drain::MustDrain,
    util_array::*,
    sync_state_entities,
};
use std::{
    sync::Arc,
    thread,
};
use vek::*;


/// Owned handle to a running server thread. Stops server when dropped.
pub struct ServerThread {
    // for sending events to the server loop
    server_send: ServerSender,
    // handle to the network server
    network_handle: NetworkServerHandle,
}

impl ServerThread {
    /// Start a server in a new thread. Does _not_ bind.
    pub fn start(thread_pool: ThreadPool, save_db: SaveDb, game: Arc<GameData>) -> Self {
        let (server_send, server_recv) = channel();
        let network_server = NetworkServer::new(server_send.clone());
        let network_handle = network_server.handle().clone();
        thread::spawn({
            let server_send = server_send.clone();
            let server_recv = server_recv.clone();
            move || run(server_send, server_recv, thread_pool, network_server, save_db, game)
        });
        ServerThread { server_send, network_handle }
    }

    /// Get the network server handle, which can be used to bind.
    pub fn network_handle(&self) -> &NetworkServerHandle {
        &self.network_handle
    }
}

impl Drop for ServerThread {
    fn drop(&mut self) {
        self.server_send.send(ServerEvent::Stop, EventPriority::Control, None, None);
    }
}


/// Run the server in this thread until it exits. Does _not_ bind.
pub fn run(
    server_send: ServerSender,
    server_recv: ServerReceiver,
    thread_pool: ThreadPool,
    network_server: NetworkServer,
    save_db: SaveDb,
    game: Arc<GameData>,
) {
    // construct
    let mut server = Server {
        server_only: ServerOnlyState {
            server_send: server_send.clone(),
            server_recv,
            thread_pool: thread_pool.clone(),
            network_server,
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
            player_open_sync_menu: Default::default(),
        },
        sync_ctx: ServerSyncCtx {
            game,
            tick_mgr: Default::default(),
            chunk_mgr: Default::default(),
            save_mgr: SaveMgr::new(server_send, save_db, thread_pool),
            conn_mgr: Default::default(),
            entities: Default::default(),
        },
        sync_state: ServerSyncState {
            tile_blocks: Default::default(),
            player_inventory_slots: Default::default(),
            chunk_steves: Default::default(),
            sw_bufs_steves: Default::default(),
            chunk_pigs: Default::default(),
            sw_bufs_pigs: Default::default(),
        },
    };

    // other initialization
    trace!("initializing server");
    request_load_spawn_chunks(&mut server);

    // enter event loop
    trace!("entering server event loop");
    loop {
        // do tick
        do_tick(&mut server);
        maybe_save(&mut server);
        server.sync_ctx.tick_mgr.on_tick_done();
        
        // process events between ticks
        while let Some(event) = server.server_only.server_recv
            .recv(Some(server.sync_ctx.tick_mgr.tick_instant()), None)
        {
            trace!(?event, "server event");
            match event {
                // server shutdown requested
                ServerEvent::Stop => {
                    info!("server stopping (stop requested)");
                    return stop(server);
                },
                // server forceful shutdown requested
                ServerEvent::ForceStop => {
                    info!("server force stopping immediately, won't save (force stop requested)");
                    return;
                },
                // network event
                ServerEvent::Network(event) => {
                    let MustDrain = server.sync_ctx.conn_mgr.handle_network_event(event);
                    process_conn_mgr_effects(&mut server);
                }
                // player save state ready
                ServerEvent::PlayerSaveStateReady { pk, save_val } => {
                    let MustDrain =
                        server.sync_ctx.conn_mgr.on_player_save_state_ready(pk, save_val);
                    process_conn_mgr_effects(&mut server);
                }
                // chunk ready
                ServerEvent::ChunkReady { save_key, save_val, saved } => {
                    let MustDrain = server.sync_ctx.chunk_mgr.on_chunk_ready(
                        save_key,
                        save_val,
                        saved,
                        server.sync_ctx.conn_mgr.players(),
                    );
                    process_chunk_mgr_effects(&mut server);
                }
                // save operation done
                ServerEvent::SaveOpDone => {
                    server.sync_ctx.save_mgr.on_save_op_done(server.sync_ctx.tick_mgr.tick_num());
                }
            }
        }
    }
}

// temporary, until we made chunk interest logic better
fn spawn_chunks() -> impl IntoIterator<Item=Vec3<i64>> {
    let ld = std::env::var("LOAD_DIST").ok()
        .and_then(|s| s
            .parse::<i64>()
            .map_err(|e| error!(%e, "error parsing load dist env var"))
            .ok()
            .filter(|&n| n > 0))
        .unwrap_or(8);
    let mut ccs = Vec::new();
    for z in -ld..ld {
        for y in 0..2 {
            for x in -ld..ld {
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
        let MustDrain = server.sync_ctx.chunk_mgr.incr_load_request_count(
            cc, server.sync_ctx.conn_mgr.players()
        );
        process_chunk_mgr_effects(server);
    }
}

// do a tick of world simulation
fn do_tick(server: &mut Server) {
    trace!("tick");
    let mut world = server.as_sync_world();

    let mut chunk_steves = world.chunk_steves.iter_move_batch();
    let mut chunk_pigs = world.chunk_pigs.iter_move_batch();

    //let (cc, ci) = world.sync_ctx.chunk_mgr.chunks().iter().next().unwrap();
    //chunk_steves.get(cc, ci);

    for (cc, ci) in world.sync_ctx.chunk_mgr.chunks().iter() {
        let mut steves = chunk_steves.get(cc, ci);
        while let Some(mut steve) = steves.next() {
            let mut rel_pos = steve.as_write().as_ref().rel_pos;
            let mut vel = steve.as_write().as_ref().state.vel;
            
            //rel_pos += Vec3::from(0.05);
            /*
            vel -= GRAVITY_ACCEL * dt;
            do_physics(
                TICK.as_secs_f32(),
                &mut rel_pos,
                &mut vel,

            )*/
            sync_state_entities::do_steve_physics(
                TICK.as_secs_f32(),
                cc,
                &mut rel_pos,
                &mut vel,
                &world.getter,
                world.tile_blocks.as_ref(),
                &world.sync_ctx.game,
                Some(steve.as_write().extra_mut()),
            );

            let rel_cc_after = (rel_pos / CHUNK_EXTENT.map(|n| n as f32)).map(f32::floor);
            if rel_cc_after != Vec3::from(0.0) {
                let cc_after = cc + rel_cc_after.map(|n| n as i64);
                if world.getter.get(cc_after).is_none() {
                    // TODO: handle this better
                    continue;
                }
            }

            steve.as_write().set_vel(vel);
            steve.set_rel_pos(rel_pos);
        }

        let mut pigs = chunk_pigs.get(cc, ci);
        while let Some(mut pig) = pigs.next() {
            let mut color = pig.as_write().as_ref().state.color;
            color += Rgb::from(0.05);
            color %= Rgb::from(1.0);
            //pig.set_color(color);
        }
    }

}

// do a save operation if appropriate to do so
fn maybe_save(server: &mut Server) {
    // ask whether should save
    if server.sync_ctx.save_mgr.should_save(server.sync_ctx.tick_mgr.tick_num()) {
        save(server);
    }
}

// unconditionally do a save operation
fn save(server: &mut Server) {
    // compile changed world state to be saved
    debug!("saving");
    let mut save_op = server.sync_ctx.save_mgr.begin_save();
    while let Some(should_save) = save_op.should_save.pop() {
        trace!(?should_save, "will save");
        save_op.will_save.push(match should_save {
            ShouldSave::Chunk { cc, ci } => SaveEntry::Chunk(
                ChunkSaveKey { cc },
                ChunkSaveVal {
                    chunk_tile_blocks: server.sync_ctx.game
                        .clone_chunk_blocks(server.sync_state.tile_blocks.get(cc, ci)),
                    // TODO factor out somehow
                    steves: server.sync_state.chunk_steves.get(cc, ci).iter().map(|entry| entry.entity.clone()).collect(),
                    pigs: server.sync_state.chunk_pigs.get(cc, ci).iter().map(|entry| entry.entity.clone()).collect(),
                },
            ),
            ShouldSave::Player { pk } => SaveEntry::Player(
                PlayerSaveKey {
                    username: server.sync_ctx.conn_mgr.player_username(pk).into(),
                },
                PlayerSaveVal {
                    pos: server.server_only.player_pos[pk],
                    yaw: server.server_only.player_yaw[pk],
                    pitch: server.server_only.player_pitch[pk],
                    inventory_slots:
                        server.sync_state.player_inventory_slots[pk].inventory_slots.clone(),
                    held_slot: server.sync_state.player_inventory_slots[pk].held_slot.clone(),
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
        trace!(?effect, "conn mgr effect");
        match effect {
            // send AcceptLogIn, initialize new player key, begin loading save state
            ConnMgrEffect::InitPlayer { pk, save_key, aborted } => {
                // send AcceptLogIn, enabling the client to process pre-join messages
                server.sync_ctx.conn_mgr.send(pk, DownMsg::AcceptLogIn(DownMsgAcceptLogIn {
                    next_tick_num: server.sync_ctx.tick_mgr.tick_num(),
                    next_tick_instant: server.sync_ctx.conn_mgr.rel_time(
                        pk,
                        server.sync_ctx.tick_mgr.tick_instant()
                    ),
                }));

                // add player
                server.sync_ctx.chunk_mgr.add_player(pk);
                for cc in spawn_chunks() {
                    let MustDrain = server.sync_ctx.chunk_mgr.add_chunk_client_interest(
                        pk, cc, server.sync_ctx.conn_mgr.players()
                    );
                    process_chunk_mgr_effects(server);
                }

                // request load
                if let Some(save_val) = server.sync_ctx.save_mgr.take_unflushed_player(&save_key) {
                    let MustDrain =
                        server.sync_ctx.conn_mgr.on_player_save_state_ready(pk, Some(save_val));
                } else {
                    server.server_only.player_save_state_loader.trigger_load(pk, save_key, aborted);
                }
            }
            // pre join message from player
            ConnMgrEffect::PreJoinMsg(pk, msg) => {
                process_pre_join_msg(server, pk, msg);
            }
            // maybe send player ShouldJoinGame
            ConnMgrEffect::ConsiderSendShouldJoinGame(pk) => {
                if server.sync_ctx.chunk_mgr.may_send_should_join_game(pk) {
                    server.sync_ctx.conn_mgr.send_should_join_game(pk);
                }
            }
            // upgrade player key to joined player key
            ConnMgrEffect::BeginJoinPlayer { pk, save_state } => {
                // **initialize most per-player stuff here**
                server.sync_ctx.save_mgr.join_player(pk, save_state.is_some());

                let (pos, yaw, pitch, inventory_slots, held_slot) = save_state
                    .map(|val| (val.pos, val.yaw, val.pitch, val.inventory_slots, val.held_slot))
                    .unwrap_or((
                        Vec3::new(8.0, 8.0, 80.0),
                        0.0,
                        0.0,
                        {
                            let mut inventory_slots = array_default();
                            inventory_slots[0] = Some(server.sync_ctx.game.content.stone.iid_stone
                                .instantiate((), 13.try_into().unwrap(), 0));
                            inventory_slots
                        },
                        Some(
                            server.sync_ctx.game.content.stone.iid_stone
                                .instantiate((), 7.try_into().unwrap(), 0)
                        ),
                    ));

                server.server_only.player_pos.insert(pk, pos);
                server.server_only.player_yaw.insert(pk, yaw);
                server.server_only.player_pitch.insert(pk, pitch);
                server.server_only.player_open_sync_menu.insert(pk, None);
                server.sync_state.player_inventory_slots.insert(pk, sync_state_inventory_slots::PlayerInventorySlots {
                    inventory_slots,
                    held_slot,
                });
            }
            // send player FinalizeJoinGame message
            ConnMgrEffect::FinalizeJoinPlayer { pk, self_clientside_player_idx } => {
                server.sync_ctx.conn_mgr.send(pk, DownMsg::FinalizeJoinGame(DownMsgFinalizeJoinGame {
                    self_player_idx: DownPlayerIdx(self_clientside_player_idx),
                    inventory_slots: server.sync_state.player_inventory_slots[pk].inventory_slots.clone(),
                    held_slot: server.sync_state.player_inventory_slots[pk].held_slot.clone(),
                }));
            }
            // add fully joined player to client
            ConnMgrEffect::AddPlayerToClient { add_to, to_add, clientside_player_idx } => {
                server.sync_ctx.conn_mgr.send(add_to, DownMsg::PreJoin(PreJoinDownMsg::AddPlayer(
                    DownMsgAddPlayer {
                        player_idx: DownPlayerIdx(clientside_player_idx),
                        username: server.sync_ctx.conn_mgr.player_username(to_add).into(),
                        pos: server.server_only.player_pos[to_add],
                        yaw: server.server_only.player_yaw[to_add],
                        pitch: server.server_only.player_pitch[to_add],
                    }
                )));
            }
            // message from player
            ConnMgrEffect::PlayerMsg(pk, msg) => {
                // process
                process_player_msg(&mut server.as_sync_world(), pk, msg);

                // ack
                if let Some(last_processed) = server.sync_ctx.conn_mgr.ack_last_processed(pk) {
                    server.sync_ctx.conn_mgr.send(pk, DownMsg::PostJoin(
                        PostJoinDownMsg::Ack { last_processed }
                    ));
                }
            }
            // remove player
            ConnMgrEffect::RemovePlayer { pk, jpk, username } => {
                // **deinitialize per-player stuff here**

                let MustDrain = server.sync_ctx.chunk_mgr.remove_player(
                    pk, spawn_chunks(), server.sync_ctx.conn_mgr.players(),
                );
                process_chunk_mgr_effects(server);
                if let Some(jpk) = jpk {
                    let pos = server.server_only.player_pos.remove(jpk);
                    let yaw = server.server_only.player_yaw.remove(jpk);
                    let pitch = server.server_only.player_pitch.remove(jpk);
                    server.server_only.player_open_sync_menu.remove(jpk);
                    let inventory_slots = server.sync_state.player_inventory_slots.remove(jpk);

                    server.sync_ctx.save_mgr.remove_player(
                        jpk,
                        PlayerSaveKey { username },
                        PlayerSaveVal {
                            pos,
                            yaw,
                            pitch,
                            inventory_slots: inventory_slots.inventory_slots,
                            held_slot: inventory_slots.held_slot,
                        },
                    );
                }
            }
        }
    }
}

// process a received pre join msg
fn process_pre_join_msg(server: &mut Server, pk: PlayerKey, msg: PreJoinUpMsg) {
    match msg {
        // relieve chunk load backpressure
        PreJoinUpMsg::AcceptMoreChunks(n) => {
            let MustDrain = server.sync_ctx.chunk_mgr.increase_client_add_chunk_budget(pk, n);
            process_chunk_mgr_effects(server);
        }
    }
}

// drain and process the chunk mgr effect queue
fn process_chunk_mgr_effects(server: &mut Server) {
    while let Some(effect) = server.sync_ctx.chunk_mgr.effects.pop_front() {
        trace!(?effect, "chunk mgr effect");
        match effect {
            // begin loading / generating save state
            ChunkMgrEffect::RequestLoad { save_key, aborted } => {
                if let Some(save_val) = server.sync_ctx.save_mgr.take_unflushed_chunk(&save_key) {
                    let MustDrain = server.sync_ctx.chunk_mgr.on_chunk_ready(
                        save_key, save_val, false, server.sync_ctx.conn_mgr.players(),
                    );
                } else {
                    server.server_only.chunk_loader.trigger_load(save_key, aborted);
                }
            }
            // install loaded chunk into world
            ChunkMgrEffect::AddChunk { cc, ci, save_val, saved } => {
                let ChunkSaveVal {
                    chunk_tile_blocks,
                    steves,
                    pigs,
                } = save_val;
                server.sync_ctx.save_mgr.add_chunk(cc, ci, saved);
                server.sync_state.tile_blocks.add(cc, ci, chunk_tile_blocks);
                // TODO: we actually should deal with UUID collisions here
                server.sync_ctx.entities.borrow_mut()
                    .add_chunk(
                        &mut server.sync_state.chunk_steves,
                        cc, ci,
                        steves.into_iter().map(|entity| (entity, Default::default())),
                    ).unwrap();
                server.sync_state.sw_bufs_steves.add_chunk(cc, ci);
                server.sync_ctx.entities.borrow_mut()
                    .add_chunk(
                        &mut server.sync_state.chunk_pigs,
                        cc, ci,
                        pigs.into_iter().map(|entity| (entity, Default::default())),
                    ).unwrap();
                server.sync_state.sw_bufs_pigs.add_chunk(cc, ci); 
            }
            // remove chunk from the world
            ChunkMgrEffect::RemoveChunk { cc, ci } => {
                let chunk_tile_blocks = server.sync_state.tile_blocks.remove(cc, ci);
                let steves = server.sync_ctx.entities.borrow_mut()
                    .remove_chunk(&mut server.sync_state.chunk_steves, cc, ci)
                    .into_iter().map(|entry| entry.entity).collect();
                server.sync_state.sw_bufs_steves.remove_chunk(cc, ci);
                let pigs = server.sync_ctx.entities.borrow_mut()
                    .remove_chunk(&mut server.sync_state.chunk_pigs, cc, ci)
                    .into_iter().map(|entry| entry.entity).collect();
                server.sync_state.sw_bufs_pigs.remove_chunk(cc, ci);
                server.sync_ctx.save_mgr.remove_chunk(
                    cc,
                    ci,
                    ChunkSaveKey { cc },
                    ChunkSaveVal {
                        chunk_tile_blocks,
                        steves,
                        pigs,
                    },
                );
            }
            // download chunk to client
            ChunkMgrEffect::AddChunkToClient { cc, ci, pk, clientside_ci } => {
                server.sync_ctx.conn_mgr.send(pk, DownMsg::PreJoin(PreJoinDownMsg::AddChunk(
                    DownMsgAddChunk {
                        chunk_idx: DownChunkIdx(clientside_ci),
                        cc,
                        chunk_tile_blocks: server.sync_ctx.game
                            .clone_chunk_blocks(server.sync_state.tile_blocks.get(cc, ci)),
                        steves: server.sync_state.chunk_steves.get(cc, ci)
                            .iter().map(|entry| entry.entity.clone()).collect(),
                        pigs: server.sync_state.chunk_pigs.get(cc, ci)
                            .iter().map(|entry| entry.entity.clone()).collect(),
                    }
                )));
            }
            // tell client to remove chunk
            ChunkMgrEffect::RemoveChunkFromClient { cc: _, ci: _, pk, clientside_ci } => {
                server.sync_ctx.conn_mgr.send(pk, DownMsg::PreJoin(PreJoinDownMsg::RemoveChunk(
                    DownMsgRemoveChunk {
                        chunk_idx: DownChunkIdx(clientside_ci),
                    }
                )));
            }
            // maybe send player ShouldJoinGame
            ChunkMgrEffect::ConsiderSendShouldJoinGame(pk) => {
                if server.sync_ctx.conn_mgr.may_send_should_join_game(pk) {
                    server.sync_ctx.conn_mgr.send_should_join_game(pk);
                }
            }
        }
    }
}

// gracefully shut down the server
fn stop(mut server: Server) {
    // shutdown subsystems
    server.sync_ctx.conn_mgr.on_shutdown();
    server.sync_ctx.chunk_mgr.on_shutdown();

    // try to save until fully saved
    while !server.sync_ctx.save_mgr.fully_saved() {
        if !server.sync_ctx.save_mgr.save_op_in_progress() {
            save(&mut server);
        }

        let event = server.server_only.server_recv.recv_unlimited_blocking(None);
        match event {
            // abort graceful shutdown upon receiving force stop
            ServerEvent::ForceStop => return,
            // kill incoming network connections when made
            ServerEvent::Network(NetworkEvent::AddConnection(_, conn)) => conn.kill(),
            // loop will exit after receiving this 0, 1, or 2 times
            ServerEvent::SaveOpDone => {
                server.sync_ctx.save_mgr.on_save_op_done(server.sync_ctx.tick_mgr.tick_num());
            }
            // everything else just ignore
            _ => (),
        }
    }
    info!("server fully saved, now exiting");
}
