
mod connection;
mod chunk_loader;
pub mod save_file; // TODO make private again or like move elsewhere or something


use self::{
    connection::{
        Connection,
        NetworkEvent,
        spawn_network_stuff,
    },
    save_file::{
        SaveFile,
        WriteEntry,
    },
    chunk_loader::ChunkLoader,
};
use super::{
    message::*,
};
use crate::{
    game_data::GameData,
    util::sparse_vec::SparseVec,
};
use chunk_data::*;
use get_assets::DataDir;
use std::{
    sync::Arc,
    time::{
        Duration,
        Instant,
    },
};
use tokio::runtime::Handle;
use anyhow::Result;
use crossbeam_channel::{
    Receiver,
    RecvTimeoutError,
    TryRecvError,
};
use vek::*;


pub const TICK: Duration = Duration::from_millis(50);


/// Body of the server thread.
pub fn run_server(
    rt: &Handle,
    data_dir: &DataDir,
    game: &Arc<GameData>,
) -> Result<()> {
    info!("initializing server data structures");
    let mut chunks: LoadedChunks = LoadedChunks::new();
    let mut tile_blocks: PerChunk<ChunkBlocks> = PerChunk::new();
    let mut connections: SparseVec<Connection> = SparseVec::new();

    // mapping from connection to clientside ci spaces
    let mut client_loaded_chunks: SparseVec<LoadedChunks> = SparseVec::new();
    // mapping from chunk to connection to clientside ci
    let mut chunk_client_cis: PerChunk<SparseVec<usize>> = PerChunk::new();
    // mapping from connection to highest up message number processed
    let mut client_last_processed: SparseVec<u64> = SparseVec::new();
    // remains all false except when used
    let mut client_last_processed_increased: SparseVec<bool> = SparseVec::new();

    let mut save = SaveFile::open("server", data_dir, game)?;
    let mut chunk_unsaved: PerChunk<bool> = PerChunk::new();
    let mut last_tick_saved = 0;

    let chunk_loader = ChunkLoader::new(&save, game);
    request_load_chunks(&chunk_loader);

    info!("beginning server tick loop");
    let mut tick = 0;
    let mut next_tick = Instant::now();
    let network_events = spawn_network_stuff("127.0.0.1:35565", rt, game);
    loop {
        trace!("doing tick");
        do_tick(
            &chunk_loader,
            &mut chunks,
            &connections,
            &mut tile_blocks,
            game,
            &mut client_loaded_chunks,
            &mut chunk_client_cis,
            &mut chunk_unsaved,
        );

        update_time_stuff_after_doing_tick(&mut tick, &mut next_tick);

        maybe_save(
            tick,
            &mut last_tick_saved,
            &chunks,
            &tile_blocks,
            &mut chunk_unsaved,
            &mut save,
            game,
        );

        process_network_events_until_next_tick(
            next_tick,
            &network_events,
            &chunks,
            &game,
            &mut connections,
            &mut tile_blocks,
            &mut client_loaded_chunks,
            &mut chunk_client_cis,
            &mut client_last_processed,
            &mut client_last_processed_increased,
            &mut chunk_unsaved,
        );
    }
}

fn do_tick(
    chunk_loader: &ChunkLoader,
    chunks: &mut LoadedChunks,
    connections: &SparseVec<Connection>,
    tile_blocks: &mut PerChunk<ChunkBlocks>,
    game: &Arc<GameData>,
    client_loaded_chunks: &mut SparseVec<LoadedChunks>,
    chunk_client_cis: &mut PerChunk<SparseVec<usize>>,
    chunk_unsaved: &mut PerChunk<bool>,
) {
    while let Some(chunk) = chunk_loader.poll_ready() {
        // oh boy, chunk ready to load
        // assign it ci in server chunk space
        let ci = chunks.add(chunk.cc);

        let mut client_cis = SparseVec::new();

        for (conn_key, conn) in connections.iter() {
            // for each connection, assign it ci in that client chunk space
            let client_ci = client_loaded_chunks[conn_key].add(chunk.cc);

            // backlink it in this chunk's new chunk_client_cis entry
            client_cis.set(conn_key, client_ci);

            // and send to that client
            send_load_chunk_message(
                chunk.cc,
                client_ci,
                &chunk.chunk_tile_blocks,
                game,
                conn,
            );
        }

        // insert into server data structures
        tile_blocks.add(chunk.cc, ci, chunk.chunk_tile_blocks);
        chunk_client_cis.add(chunk.cc, ci, client_cis);

        chunk_unsaved.add(chunk.cc, ci, true);
    }
}

fn update_time_stuff_after_doing_tick(tick: &mut u64, next_tick: &mut Instant) {
    *tick += 1;

    *next_tick += TICK;
    let now = Instant::now();
    if *next_tick < now {
        let behind_nanos = (now - *next_tick).as_nanos();
        // poor man's div_ceil
        let behind_ticks = match behind_nanos % TICK.as_nanos() {
            0 => behind_nanos / TICK.as_nanos(),
            _ => behind_nanos / TICK.as_nanos() + 1,
        };
        let behind_ticks = u32::try_from(behind_ticks).expect("time broke");
        warn!("running too slow, skipping {behind_ticks} ticks");
        *next_tick += TICK * behind_ticks;
    }
}

fn maybe_save(
    tick: u64,
    last_tick_saved: &mut u64,
    chunks: &LoadedChunks,
    tile_blocks: &PerChunk<ChunkBlocks>,
    chunk_unsaved: &mut PerChunk<bool>,
    save: &mut SaveFile,
    game: &Arc<GameData>,
) {
    const TICKS_BETWEEN_SAVES: u64 = 10 * 20;

    if tick - *last_tick_saved < TICKS_BETWEEN_SAVES {
        return;
    }

    debug!("saving");
    
    *last_tick_saved = tick;

    save.write(chunks.iter()
        .filter_map(|(cc, ci)| {
            if *chunk_unsaved.get(cc, ci) {
                *chunk_unsaved.get_mut(cc, ci) = false;
                Some(WriteEntry::Chunk(
                    cc,
                    clone_chunk_tile_blocks(tile_blocks.get(cc, ci), game),
                ))
            } else {
                None
            }
        }))
        .unwrap(); // TODO: don't panic
}

fn process_network_events_until_next_tick(
    next_tick: Instant,
    network_events: &Receiver<NetworkEvent>,
    chunks: &LoadedChunks,
    game: &Arc<GameData>,
    connections: &mut SparseVec<Connection>,
    tile_blocks: &mut PerChunk<ChunkBlocks>,
    client_loaded_chunks: &mut SparseVec<LoadedChunks>,
    chunk_client_cis: &mut PerChunk<SparseVec<usize>>,
    client_last_processed: &mut SparseVec<u64>,
    client_last_processed_increased: &mut SparseVec<bool>,
    chunk_unsaved: &mut PerChunk<bool>,
) {
    while let Ok(event) = network_events
        .recv_deadline(next_tick)
        .map_err(|e| debug_assert!(matches!(e, RecvTimeoutError::Timeout)))
    {
        on_network_event(
            event,
            chunks,
            game,
            connections,
            tile_blocks,
            client_loaded_chunks,
            chunk_client_cis,
            client_last_processed,
            client_last_processed_increased,
            chunk_unsaved,
        );

        while let Ok(event) = network_events
            .try_recv()
            .map_err(|e| debug_assert!(matches!(e, TryRecvError::Empty)))
        {
            on_network_event(
                event,
                chunks,
                game,
                connections,
                tile_blocks,
                client_loaded_chunks,
                chunk_client_cis,
                client_last_processed,
                client_last_processed_increased,
                chunk_unsaved,
            );                
        }

        after_process_available_network_events(
            connections,
            client_last_processed,
            client_last_processed_increased,
        )
    }
}

fn on_network_event(
    event: NetworkEvent,
    chunks: &LoadedChunks,
    game: &Arc<GameData>,
    connections: &mut SparseVec<Connection>,
    tile_blocks: &mut PerChunk<ChunkBlocks>,
    client_loaded_chunks: &mut SparseVec<LoadedChunks>,
    chunk_client_cis: &mut PerChunk<SparseVec<usize>>,
    client_last_processed: &mut SparseVec<u64>,
    client_last_processed_increased: &mut SparseVec<bool>,
    chunk_unsaved: &mut PerChunk<bool>,
) {
    match event {
        NetworkEvent::NewConnection(conn_key, conn) => on_new_connection(
            conn_key,
            conn,
            chunks,
            game,
            connections,
            tile_blocks,
            client_loaded_chunks,
            chunk_client_cis,
            client_last_processed,
            client_last_processed_increased,
        ),
        NetworkEvent::Disconnected(conn_key) => on_disconnected(
            conn_key,
            connections,
            client_loaded_chunks,
            client_last_processed,
            client_last_processed_increased,
            chunks,
            chunk_client_cis,
        ),
        NetworkEvent::Received(conn_key, msg) => on_received(
            conn_key,
            msg,
            tile_blocks,
            chunk_client_cis,
            connections,
            chunks,
            client_last_processed,
            client_last_processed_increased,
            chunk_unsaved,
        ),
    }
}

fn on_new_connection(
    conn_key: usize,
    conn: Connection,
    chunks: &LoadedChunks,
    game: &Arc<GameData>,
    connections: &mut SparseVec<Connection>,
    tile_blocks: &PerChunk<ChunkBlocks>,
    client_loaded_chunks: &mut SparseVec<LoadedChunks>,
    chunk_client_cis: &mut PerChunk<SparseVec<usize>>,
    client_last_processed: &mut SparseVec<u64>,
    client_last_processed_increased: &mut SparseVec<bool>,
) {
    connections.set(conn_key, conn);

    let mut loaded_chunks = LoadedChunks::new();

    // for each chunk already loaded
    for (cc, ci) in chunks.iter() {
        // add it to the client's loaded chunks set
        let client_ci = loaded_chunks.add(cc);

        // backlink it in the chunk's chunk_client_cis entry
        chunk_client_cis.get_mut(cc, ci).set(conn_key, client_ci);
        
        // send the chunk to the client
        send_load_chunk_message(
            cc,
            client_ci,
            tile_blocks.get(cc, ci),
            game,
            &connections[conn_key],
        );
    }

    // insert the client's new loaded_chunks set into the server's data structures
    client_loaded_chunks.set(conn_key, loaded_chunks);

    // insert other things into the server's data structures
    // (up msg indices starts at 1, so setting last_processed to 0 indicates that
    // no messages from that client have been processed)
    client_last_processed.set(conn_key, 0);
    client_last_processed_increased.set(conn_key, false);
}

fn on_disconnected(
    conn_key: usize,
    connections: &mut SparseVec<Connection>,
    client_loaded_chunks: &mut SparseVec<LoadedChunks>,
    client_last_processed: &mut SparseVec<u64>,
    client_last_processed_increased: &mut SparseVec<bool>,
    chunks: &LoadedChunks,
    chunk_client_cis: &mut PerChunk<SparseVec<usize>>,
) {
    // remove from list of connections
    connections.remove(conn_key);

    // remove client's clientside ci from all chunks
    for (cc, _) in client_loaded_chunks[conn_key].iter() {
        let ci = chunks.getter().get(cc).unwrap();
        chunk_client_cis.get_mut(cc, ci).remove(conn_key);
    }

    // remove client's ci space
    client_loaded_chunks.remove(conn_key);

    // remove from other data structures
    client_last_processed.remove(conn_key);
    client_last_processed_increased.remove(conn_key);
}

fn on_received(
    conn_key: usize,
    msg: UpMessage,
    tile_blocks: &mut PerChunk<ChunkBlocks>,
    chunk_client_cis: &PerChunk<SparseVec<usize>>,
    connections: &SparseVec<Connection>,
    chunks: &LoadedChunks,
    client_last_processed: &mut SparseVec<u64>,
    client_last_processed_increased: &mut SparseVec<bool>,
    chunk_unsaved: &mut PerChunk<bool>,
) {
    client_last_processed[conn_key] += 1;
    client_last_processed_increased[conn_key] = true;

    match msg {
        UpMessage::SetTileBlock(up::SetTileBlock {
            gtc,
            bid,
        }) => {
            // lookup tile
            let tile = match chunks.getter().gtc_get(gtc) {
                Some(tile) => tile,
                None => {
                    info!("client tried SetTileBlock on non-present gtc");
                    return;
                }
            };

            // set tile block
            tile.get(tile_blocks).raw_set(bid, ());

            // send update to all clients with that chunk loaded
            for (conn_key, &client_ci) in chunk_client_cis.get(tile.cc, tile.ci).iter() {
                let ack = if client_last_processed_increased[conn_key] {
                    client_last_processed_increased[conn_key] = false;
                    Some(client_last_processed[conn_key])
                } else {
                    None
                };
                connections[conn_key].send(down::ApplyEdit {
                    ack,
                    ci: client_ci,
                    edit: edit::SetTileBlock {
                        lti: tile.lti,
                        bid,
                    }.into(),
                });
                client_last_processed_increased[conn_key] = false;
                *chunk_unsaved.get_mut(tile.cc, tile.ci) = true;
            }
        }
    }
}

fn after_process_available_network_events(
    connections: &SparseVec<Connection>,
    client_last_processed: &mut SparseVec<u64>,
    client_last_processed_increased: &mut SparseVec<bool>,
) {
    for (conn_key, conn) in connections.iter() {
        if client_last_processed_increased[conn_key] {
            conn.send(down::Ack {
                last_processed: client_last_processed[conn_key],
            });
            client_last_processed_increased[conn_key] = false;
        }
    }
}

fn send_load_chunk_message(
    cc: Vec3<i64>,
    ci: usize,
    chunk_tile_blocks: &ChunkBlocks,
    game: &Arc<GameData>,
    connection: &Connection,
) {
    connection.send(down::LoadChunk {
        cc,
        ci,
        chunk_tile_blocks: clone_chunk_tile_blocks(chunk_tile_blocks, game),
    });
}

fn clone_chunk_tile_blocks(
    chunk_tile_blocks: &ChunkBlocks,
    game: &Arc<GameData>,
) -> ChunkBlocks {
    let mut chunk_tile_blocks_clone = ChunkBlocks::new(&game.blocks);
    for lti in 0..=MAX_LTI {
        chunk_tile_blocks.raw_meta::<()>(lti);
        chunk_tile_blocks_clone.raw_set(lti, chunk_tile_blocks.get(lti), ());
    }
    chunk_tile_blocks_clone
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
