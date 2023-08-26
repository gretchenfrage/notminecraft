
mod connection;
mod chunk_loader;


use self::{
    connection::{
        Connection,
        NetworkEvent,
        spawn_network_stuff,
    },
    chunk_loader::ChunkLoader,
};
use super::message::*;
use crate::{
    game_data::GameData,
    block_update_queue::BlockUpdateQueue,
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
    let mut block_updates: BlockUpdateQueue = BlockUpdateQueue::new();
    let mut connections: Slab<Connection> = Slab::new();

    let chunk_loader = ChunkLoader::new(game);
    request_load_chunks(&chunk_loader);

    info!("beginning server tick loop");
    let mut next_tick = Instant::now();
    let network_events = spawn_network_stuff("127.0.0.1:35565", rt);
    loop {
        trace!("doing tick");
        do_tick();

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
            warn!("running to slow, skipping {behind_ticks} ticks");
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

fn do_tick() {

}

fn on_network_message(
    conn_key: usize,
    msg: UpMessage,
) {
    match msg {

    }
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
