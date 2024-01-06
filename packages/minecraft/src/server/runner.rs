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
        network::NetworkEvent,
        *,
    },
    thread_pool::ThreadPool,
};
use std::sync::Arc;

/*
/// Owned handle to a running server thread. Stops server when dropped.
pub struct ServerThread {
    server_send: ServerSender,
    network_server: NetworkServer,
}

impl */

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
                ServerEvent::Stop => {
                    info!("server stopping (stop requested)");
                    return;
                },
                ServerEvent::Network(event) => {
                    server.sync_ctx.conn_mgr.handle_network_event(event);
                    process_conn_mgr_effects(&mut server);
                    // TODO: do this better
                    
                }
                ServerEvent::PlayerSaveStateReady { pk, save_val } => {
                    server.sync_ctx.conn_mgr.on_player_save_state_ready(pk, save_val);
                }
                ServerEvent::ChunkReady { save_key, save_val, saved } => {
                    server.sync_ctx.chunk_mgr.on_chunk_ready(
                        save_key,
                        save_val,
                        saved,
                        server.sync_ctx.conn_mgr.players(),
                    );
                    process_chunk_mgr_effects(&mut server);
                }
                ServerEvent::SaveOpDone => {
                    server.sync_ctx.save_mgr.on_save_op_done(server.sync_ctx.tick_mgr.tick());
                }
            }
        }
    }
}

fn request_load_spawn_chunks(server: &mut Server) {
    unimplemented!()
}

fn do_tick(server: &mut Server) {

}

fn maybe_save(server: &mut Server) {
    let save_op = server.sync_ctx.save_mgr.maybe_save(server.sync_ctx.tick_mgr.tick());
    let mut save_op = match save_op {
        Some(save_op) => save_op,
        None => return,
    };

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

    save_op.submit();
}


fn process_conn_mgr_effects(server: &mut Server) {
    unimplemented!()
}

fn process_chunk_mgr_effects(server: &mut Server) {
    unimplemented!()
}
