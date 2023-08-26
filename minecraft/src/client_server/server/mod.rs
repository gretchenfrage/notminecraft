
mod connection;
mod chunk_loader;


use self::{
    connection::{
        Connection,
        NetworkEvent,
        spawn_network_stuff,
    },
    chunk_loader::{
        ChunkLoader,
        ReadyChunk,
    },
};
use super::message::*;
use crate::{
    game_data::GameData,
//    block_update_queue::BlockUpdateQueue,
};
use chunk_data::*;
use std::{
    sync::Arc,
    time::{
        Duration,
        Instant,
    },
};
use tokio::runtime::Handle;
use anyhow::{Result, bail};
use crossbeam_channel::RecvTimeoutError;
use slab::Slab;
use vek::*;


pub const TICK: Duration = Duration::from_millis(50);


/// Body of the server thread.
pub fn run_server(
    rt: &Handle,
    game: &Arc<GameData>,
) -> Result<()> {
    info!("initializing server data structures");
    let mut chunks: LoadedChunks = LoadedChunks::new();
    let mut tile_blocks: PerChunk<ChunkBlocks> = PerChunk::new();
    //let mut block_updates: BlockUpdateQueue = BlockUpdateQueue::new();
    let mut connections: Slab<Connection> = Slab::new();

    let chunk_loader = ChunkLoader::new(game);
    request_load_chunks(&chunk_loader);

    info!("beginning server tick loop");
    let mut next_tick = Instant::now();
    let network_events = spawn_network_stuff("127.0.0.1:35565", rt);
    loop {
        trace!("doing tick");
        do_tick(
            &chunk_loader,
            &mut chunks,
            &connections,
            &mut tile_blocks,
            game,
        );

        next_tick += TICK;
        let now = Instant::now();
        if next_tick < now {
            let behind_nanos = (now - next_tick).as_nanos();
            // poor man's div_ceil
            let behind_ticks = match behind_nanos % TICK.as_nanos() {
                0 => behind_nanos / TICK.as_nanos(),
                _ => behind_nanos / TICK.as_nanos() + 1,
            };
            let behind_ticks = u32::try_from(behind_ticks).expect("time broke");
            warn!("running too slow, skipping {behind_ticks} ticks");
            next_tick += TICK * behind_ticks;
        }

        while let Some(event) = match network_events.recv_deadline(next_tick) {
            Ok(event) => Some(event),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => {
                bail!("unexpected disconnection of network events channel");
            },
        } {
            match event {
                NetworkEvent::NewConnection(conn_key_1, conn) => {
                    let conn_key_2 = connections.insert(conn);
                    debug_assert_eq!(conn_key_1, conn_key_2);
                    on_new_connection(
                        conn_key_1,
                        &chunks,
                        game,
                        &connections,
                        &tile_blocks,
                    );
                }
                NetworkEvent::Disconnected(conn_key) => {
                    connections.remove(conn_key);
                }
                NetworkEvent::Received(conn_key, msg) => {
                    on_network_message(conn_key, msg);
                }
            }
        }
    }
}

fn do_tick(
    chunk_loader: &ChunkLoader,
    chunks: &mut LoadedChunks,
    connections: &Slab<Connection>,
    tile_blocks: &mut PerChunk<ChunkBlocks>,
    game: &Arc<GameData>,
) {
    while let Some(ready_chunk) = chunk_loader.poll_ready() {
        let ReadyChunk {
            cc,
            chunk_tile_blocks,
        } = ready_chunk;

        let ci = chunks.add(cc);

        for (_, conn) in connections {
            send_load_chunk_message(
                cc,
                ci,
                &chunk_tile_blocks,
                game,
                conn,
            );
        }

        tile_blocks.add(cc, ci, chunk_tile_blocks);
    }
}

fn on_network_message(
    _conn_key: usize,
    msg: UpMessage,
) {
    match msg {

    }
}

fn on_new_connection(
    conn_key: usize,
    chunks: &LoadedChunks,
    game: &Arc<GameData>,
    connections: &Slab<Connection>,
    tile_blocks: &PerChunk<ChunkBlocks>,
) {
    for (cc, ci) in chunks.iter() {
        send_load_chunk_message(
            cc,
            ci,
            tile_blocks.get(cc, ci),
            game,
            &connections[conn_key],
        );
    }
}

fn send_load_chunk_message(
    cc: Vec3<i64>,
    ci: usize,
    chunk_tile_blocks: &ChunkBlocks,
    game: &Arc<GameData>,
    connection: &Connection,
) {
    let mut chunk_tile_blocks_clone = ChunkBlocks::new(&game.blocks);
    for lti in 0..=MAX_LTI {
        chunk_tile_blocks.raw_meta::<()>(lti);
        chunk_tile_blocks_clone.raw_set(lti, chunk_tile_blocks.get(lti), ());
    }
    connection.send(DownMessage::LoadChunk(DownMessageLoadChunk {
        cc,
        ci,
        chunk_tile_blocks: chunk_tile_blocks_clone,
    }));
}

fn request_load_chunks(chunk_loader: &ChunkLoader) {
    let view_dist = 6;
    let mut to_request = Vec::new();
    for x in -view_dist..view_dist {
        for z in -view_dist..view_dist {
            for y in 0..2 {
                to_request.push(Vec3 { x, y, z });
            }
        }
    }
    fn square(n: i64) -> i64 {
        n * n
    }
    to_request.sort_by_key(|cc| square(cc.x) + square(cc.z));
    for cc in to_request {
        chunk_loader.request(cc);
    }
}
