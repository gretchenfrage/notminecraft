
mod connection;
mod chunk_loader;
pub mod save_file; // TODO make private again or like move elsewhere or something


use self::{
    connection::{
        Connection,
        NetworkEvent,
        NetworkServer,
    },
    save_file::{
        SaveFile,
        WriteEntry,
    },
    chunk_loader::ChunkLoader,
};
use super::message::*;
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
    collections::HashMap,
};
use tokio::runtime::Handle;
use anyhow::Result;
use slab::Slab;
use vek::*;


const TICK: Duration = Duration::from_millis(50);
const TICKS_BETWEEN_SAVES: u64 = 10 * 20;


/// Body of the server thread.
pub fn run_server(
    rt: &Handle,
    data_dir: &DataDir,
    game: &Arc<GameData>,
) -> Result<()> {
    Server::new(rt, data_dir, game)?.run()
}


struct Server {
    game: Arc<GameData>,

    chunks: LoadedChunks,
    tile_blocks: PerChunk<ChunkBlocks>,

    // maps from state-invariant connection keys to which state the connection
    // is in and its key within that state's connection key space
    all_connections: SparseVec<(ConnectionState, usize)>,
    
    // state connection key spaces
    uninit_connections: Slab<Connection>,
    client_connections: Slab<Connection>,
    
    // mapping from all connection to highest up message number processed
    conn_last_processed: SparseVec<u64>,
    // remains all false except when used
    conn_last_processed_increased: SparseVec<bool>,

    // mapping from client to clientside ci spaces
    // TODO: could just store a Slab<()> rather than a LoadedChunks in there
    client_loaded_chunks: SparseVec<LoadedChunks>,
    // mapping from chunk to client to clientside ci
    chunk_client_cis: PerChunk<SparseVec<usize>>,

    // mapping from client to clientside client key spaces
    // which then maps from clientside client key to serverside client key
    client_clientside_client_keys: SparseVec<Slab<usize>>,
    // maping from client A to client B to client A's clientside client key for B
    client_client_clientside_keys: SparseVec<SparseVec<usize>>,

    // mapping from client to username
    client_username: SparseVec<String>,
    // mapping from username to client
    username_client: HashMap<String, usize>,

    client_char_state: SparseVec<CharState>,

    save: SaveFile,
    chunk_unsaved: PerChunk<bool>,
    last_tick_saved: u64,

    chunk_loader: ChunkLoader,

    tick: u64,
    next_tick: Instant,
    network_server: NetworkServer,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
enum ConnectionState {
    // state a connection starts in
    Uninit,
    // connection is logged in as a connected client
    Client,
}

impl Server {
    /// Construct. This is expected to be immediately followed by `run`.
    fn new(
        rt: &Handle,
        data_dir: &DataDir,
        game: &Arc<GameData>,
    ) -> Result<Self> {
        let save = SaveFile::open("server", data_dir, game)?;
        let chunk_loader = ChunkLoader::new(&save, game);

        Ok(Server {
            game: Arc::clone(&game),
            chunks: LoadedChunks::new(),
            tile_blocks: PerChunk::new(),
            all_connections: SparseVec::new(),
            uninit_connections: Slab::new(),
            client_connections: Slab::new(),
            conn_last_processed: SparseVec::new(),
            conn_last_processed_increased: SparseVec::new(),
            client_loaded_chunks: SparseVec::new(),
            chunk_client_cis: PerChunk::new(),
            client_clientside_client_keys: SparseVec::new(),
            client_client_clientside_keys: SparseVec::new(),
            client_username: SparseVec::new(),
            username_client: HashMap::new(),
            client_char_state: SparseVec::new(),
            save,
            chunk_unsaved: PerChunk::new(),
            last_tick_saved: 0,
            chunk_loader,
            tick: 0,
            next_tick: Instant::now(),
            network_server: NetworkServer::spawn("127.0.0.1:35565", rt, game),
        })
    }

    /// Run the server forever.
    fn run(&mut self) -> ! {
        self.request_load_chunks();
    
        loop {
            trace!("doing tick");
            self.do_tick();
            self.update_time_stuff_after_doing_tick();
            self.maybe_save();
            self.process_network_events_until_next_tick();
        }
    }

    /// Do a tick.
    fn do_tick(&mut self) {
        while let Some(chunk) = self.chunk_loader.poll_ready() {
            // oh boy, chunk ready to load
            // assign it ci in server chunk space
            let ci = self.chunks.add(chunk.cc);

            let mut client_cis = SparseVec::new();

            for (client_conn_key, conn) in self.client_connections.iter() {
                // for each connection, assign it ci in that client chunk space
                let client_ci = self.client_loaded_chunks[client_conn_key].add(chunk.cc);

                // backlink it in this chunk's new chunk_client_cis entry
                client_cis.set(client_conn_key, client_ci);

                // and send to that client
                self.send_load_chunk_message(
                    chunk.cc,
                    client_ci,
                    &chunk.chunk_tile_blocks,
                    conn,
                );
            }

            // insert into server data structures
            self.tile_blocks.add(chunk.cc, ci, chunk.chunk_tile_blocks);
            self.chunk_client_cis.add(chunk.cc, ci, client_cis);

            self.chunk_unsaved.add(chunk.cc, ci, chunk.unsaved);
        }
    }

    /// Update `tick` and `next_tick` to schedule the happening of the next tick.
    fn update_time_stuff_after_doing_tick(&mut self) {
        self.tick += 1;

        self.next_tick += TICK;
        let now = Instant::now();
        if self.next_tick < now {
            let behind_nanos = (now - self.next_tick).as_nanos();
            // poor man's div_ceil
            let behind_ticks = match behind_nanos % TICK.as_nanos() {
                0 => behind_nanos / TICK.as_nanos(),
                _ => behind_nanos / TICK.as_nanos() + 1,
            };
            let behind_ticks = u32::try_from(behind_ticks).expect("time broke");
            warn!("running too slow, skipping {behind_ticks} ticks");
            self.next_tick += TICK * behind_ticks;
        }
    }

    /// Save unsaved chunks if it's been long enough since the last save.
    fn maybe_save(&mut self) {
        if self.tick - self.last_tick_saved < TICKS_BETWEEN_SAVES {
            return;
        }

        debug!("saving");
        
        self.last_tick_saved = self.tick;

        self.save.write(self.chunks.iter()
            .filter_map(|(cc, ci)| {
                if *self.chunk_unsaved.get(cc, ci) {
                    *self.chunk_unsaved.get_mut(cc, ci) = false;
                    Some(WriteEntry::Chunk(
                        cc,
                        clone_chunk_tile_blocks(self.tile_blocks.get(cc, ci), &self.game),
                    ))
                } else {
                    None
                }
            }))
            .unwrap(); // TODO: don't panic
    }

    /// Wait for and process network events until `self.next_tick`.
    fn process_network_events_until_next_tick(&mut self) {
        while let Some(event) = self.network_server.recv_deadline(self.next_tick) {
            self.on_network_event(event);

            while let Some(event) = self.network_server.poll() {
                self.on_network_event(event);
            }

            self.after_process_available_network_events();
        }
    }

    /// Process any network event.
    fn on_network_event(&mut self, event: NetworkEvent) {
        match event {
            NetworkEvent::NewConnection(all_conn_key, conn) => self.on_new_connection(all_conn_key, conn),
            NetworkEvent::Disconnected(all_conn_key) => self.on_disconnected(all_conn_key),
            NetworkEvent::Received(all_conn_key, msg) => self.on_received(all_conn_key, msg),
        }
    }

    /// Process a new connection network event.
    fn on_new_connection(&mut self, all_conn_key: usize, conn: Connection) {
        let uninit_conn_key = self.uninit_connections.insert(conn);
        self.all_connections.set(all_conn_key, (ConnectionState::Uninit, uninit_conn_key));
        
        // insert other things into the server's data structures
        // (up msg indices starts at 1, so setting last_processed to 0 indicates that
        // no messages from that client have been processed)
        self.conn_last_processed.set(all_conn_key, 0);
        self.conn_last_processed_increased.set(all_conn_key, false);
    }

    /// Process a disconnection network event.
    fn on_disconnected(&mut self, all_conn_key: usize) {
        let (conn_state, state_conn_key) = self.all_connections.remove(all_conn_key);

        self.conn_last_processed.remove(all_conn_key);
        self.conn_last_processed_increased.remove(all_conn_key);

        match conn_state {
            ConnectionState::Uninit => self.on_uninit_disconnected(state_conn_key),
            ConnectionState::Client => self.on_client_disconnected(state_conn_key),
        }
    }

    /// Process the disconnection of an uninit connection.
    fn on_uninit_disconnected(&mut self, uninit_conn_key: usize) {
        self.uninit_connections.remove(uninit_conn_key);
    }

    /// Process the disconnection of a client connection.
    fn on_client_disconnected(&mut self, client_conn_key: usize) {
        // remove from list of connections
        self.client_connections.remove(client_conn_key);

        // remove client's clientside ci from all chunks
        for (cc, _) in self.client_loaded_chunks[client_conn_key].iter() {
            let ci = self.chunks.getter().get(cc).unwrap();
            self.chunk_client_cis.get_mut(cc, ci).remove(client_conn_key);
        }

        // remove client from other clients
        for (client_conn_key2, client_conn2) in self.client_connections.iter() {
            let clientside_client_key = self.client_client_clientside_keys[client_conn_key2].remove(client_conn_key);
            self.client_clientside_client_keys[client_conn_key2].remove(clientside_client_key);
            client_conn2.send(down::RemoveClient {
                client_key: clientside_client_key,
            });
        }

        // remove from other data structures
        self.client_loaded_chunks.remove(client_conn_key);

        self.client_clientside_client_keys.remove(client_conn_key);
        self.client_client_clientside_keys.remove(client_conn_key);

        let username = self.client_username.remove(client_conn_key);
        self.username_client.remove(&username);

        self.client_char_state.remove(client_conn_key);

        // announce
        self.broadcast_chat_line(&format!("{} left the game", username));
    }

    /// Process the receipt of a network message.
    fn on_received(&mut self, all_conn_key: usize, msg: UpMessage) {
        self.conn_last_processed[all_conn_key] += 1;
        self.conn_last_processed_increased[all_conn_key] = true;

        let (conn_state, state_conn_key) = self.all_connections[all_conn_key];
        match conn_state {
            ConnectionState::Uninit => self.on_received_uninit(msg, all_conn_key, state_conn_key),
            ConnectionState::Client => self.on_received_client(msg, all_conn_key, state_conn_key),
        }
    }

    /// Process the receipt of a network message from an uninit connection.
    fn on_received_uninit(&mut self, msg: UpMessage, all_conn_key: usize, uninit_conn_key: usize) {
        match msg {
            UpMessage::LogIn(msg) => self.on_received_uninit_log_in(msg, all_conn_key, uninit_conn_key),
            UpMessage::SetTileBlock(_) => {
                error!("uninit connection sent settileblock");
                // TODO: handle this better than just ignoring it lol
            }
            UpMessage::Say(_) => {
                error!("uninit connection sent say");
                // TODO: handle this better than just ignoring it lol
            }
            UpMessage::SetCharState(_) => {
                error!("uninit connection sent set char state");
                // TODO: handle this better than just ignoring it lol
            }
        }
    }

    /// Process the receipt of a `LogIn` message from an uninit connection.
    fn on_received_uninit_log_in(&mut self, msg: up::LogIn, all_conn_key: usize, uninit_conn_key: usize) {
        let up::LogIn { mut username, char_state } = msg;

        // "validate"
        /*
        if username_client.contains_key(&username) {
            uninit_connections[uninit_conn_key]
                .send(DownMessage::RejectLogIn(down::RejectLogIn {
                    message: "client already logged in with same username".into(),
                }));
            return;
        }
        */
        if self.username_client.contains_key(&username) {
            let mut i = 2;
            let mut username2;
            while {
                username2 = format!("{}{}", username, i);
                self.username_client.contains_key(&username2)
            } { i += 1 }
            username = username2;
        }

        let conn = self.uninit_connections.remove(uninit_conn_key);
        let client_conn_key = self.client_connections.insert(conn);
        self.all_connections.set(all_conn_key, (ConnectionState::Client, client_conn_key));

        let mut loaded_chunks = LoadedChunks::new();

        // for each chunk already loaded
        for (cc, ci) in self.chunks.iter() {
            // add it to the client's loaded chunks set
            let client_ci = loaded_chunks.add(cc);

            // backlink it in the chunk's chunk_client_cis entry
            self.chunk_client_cis.get_mut(cc, ci).set(client_conn_key, client_ci);
            
            // send the chunk to the client
            self.send_load_chunk_message(
                cc,
                client_ci,
                self.tile_blocks.get(cc, ci),
                &self.client_connections[client_conn_key],
            );
        }

        // insert the client's new loaded_chunks set into the server's data structures
        self.client_loaded_chunks.set(client_conn_key, loaded_chunks);

        // tell this new client about all other clients (not including this new one)
        let mut clientside_client_keys = Slab::new();
        let mut client_clientside_keys = SparseVec::new();
        for (client_conn_key2, _) in self.client_connections.iter() {
            if client_conn_key2 == client_conn_key {
                continue;
            }

            let clientside_client_key = clientside_client_keys.insert(client_conn_key2);
            client_clientside_keys.set(client_conn_key2, clientside_client_key);
            self.client_connections[client_conn_key].send(down::AddClient {
                client_key: clientside_client_key,
                username: self.client_username[client_conn_key2].clone(),
                char_state: self.client_char_state[client_conn_key2],
            });
        }

        self.client_clientside_client_keys.set(client_conn_key, clientside_client_keys);
        self.client_client_clientside_keys.set(client_conn_key, client_clientside_keys);

        // tell all clients (including this new one) about this new client
        for (client_conn_key2, client_conn2) in self.client_connections.iter() {
            let clientside_client_key = self.client_clientside_client_keys[client_conn_key2].insert(client_conn_key);
            self.client_client_clientside_keys[client_conn_key2].set(client_conn_key, clientside_client_key);
            client_conn2.send(down::AddClient {
                client_key: clientside_client_key,
                username: username.clone(),
                char_state,
            });

            if client_conn_key2 == client_conn_key {
                // when telling it about itself, send the this-is-you
                client_conn2.send(down::ThisIsYou {
                    client_key: clientside_client_key,
                });
            }
        }

        // insert into other server data structures
        self.client_username.set(client_conn_key, username.clone());
        self.username_client.insert(username.clone(), client_conn_key);
        
        self.client_char_state.set(client_conn_key, char_state);

        // announce
        self.broadcast_chat_line(&format!("{} joined the game", username));
    }

    /// Process the receipt of a network message from a client connection.
    fn on_received_client(&mut self, msg: UpMessage, all_conn_key: usize, client_conn_key: usize) {
        match msg {
            UpMessage::LogIn(_) => {
                error!("client connection sent login");
                // TODO: handle this better than just ignoring it lol
            }
            UpMessage::SetTileBlock(msg) => self.on_received_client_set_tile_block(msg, all_conn_key),
            UpMessage::Say(msg) => self.on_received_client_say(msg, client_conn_key),
            UpMessage::SetCharState(msg) => self.on_received_client_set_char_state(msg, client_conn_key),
        }
    }

    /// Process the receipt of a `SetTileBlock` message from a client connection.
    fn on_received_client_set_tile_block(&mut self, msg: up::SetTileBlock, all_conn_key: usize) {
        let up::SetTileBlock { gtc, bid } = msg;

        // lookup tile
        let tile = match self.chunks.getter().gtc_get(gtc) {
            Some(tile) => tile,
            None => {
                info!("client tried SetTileBlock on non-present gtc");
                return;
            }
        };

        // set tile block
        tile.get(&mut self.tile_blocks).raw_set(bid, ());

        // send update to all clients with that chunk loaded
        for (client_conn_key, &client_ci) in self.chunk_client_cis.get(tile.cc, tile.ci).iter() {
            let ack = if self.conn_last_processed_increased[all_conn_key] {
                self.conn_last_processed_increased[all_conn_key] = false;
                Some(self.conn_last_processed[all_conn_key])
            } else {
                None
            };
            self.client_connections[client_conn_key].send(down::ApplyEdit {
                ack,
                ci: client_ci,
                edit: edit::SetTileBlock {
                    lti: tile.lti,
                    bid,
                }.into(),
            });
            self.conn_last_processed_increased[all_conn_key] = false;
            *self.chunk_unsaved.get_mut(tile.cc, tile.ci) = true;
        }
    }

    /// Process the receipt of a `Say` message from a client connection.
    fn on_received_client_say(&mut self, msg: up::Say, client_conn_key: usize) {
        let up::Say { text } = msg;

        let username = &self.client_username[client_conn_key];
        let line = format!("<{}> {}", username, text);
        self.broadcast_chat_line(&line);
    }

    fn broadcast_chat_line(&self, line: &str) {
        for (_, connection) in &self.client_connections {
            connection.send(down::ChatLine {
                line: line.to_owned(),
            });
        }
    }

    /// Process the receipt of a `SetCharState` message from a client connection.
    fn on_received_client_set_char_state(&mut self, msg: up::SetCharState, client_conn_key: usize) {
        let up::SetCharState { char_state } = msg;
        self.client_char_state[client_conn_key] = char_state;
        for (client_conn_key2, client_conn2) in self.client_connections.iter() {
            client_conn2.send(down::SetCharState {
                client_key: client_conn_key,
                char_state,
            });
        }
    }

    /// Called after processing at least one network event and then processing
    /// all subsequent network events that were immediately available without
    /// additional blocking.
    fn after_process_available_network_events(&mut self) {
        for (all_conn_key, client_conn_key) in self.all_connections.iter()
            .filter(|(_, &(conn_state, _))| conn_state == ConnectionState::Client)
            .map(|(all_conn_key, &(_, client_conn_key))| (all_conn_key, client_conn_key))
        {
            let conn = &self.client_connections[client_conn_key];

            if self.conn_last_processed_increased[all_conn_key] {
                conn.send(down::Ack {
                    last_processed: self.conn_last_processed[all_conn_key],
                });
                self.conn_last_processed_increased[all_conn_key] = false;
            }
        }
    }

    /// Send a connection a `LoadChunk` message.
    fn send_load_chunk_message(
        &self,
        cc: Vec3<i64>,
        ci: usize,
        chunk_tile_blocks: &ChunkBlocks,
        connection: &Connection,
    )  {
        connection.send(down::AddChunk {
            cc,
            ci,
            chunk_tile_blocks: clone_chunk_tile_blocks(chunk_tile_blocks, &self.game),
        });
    }

    /// Called during initialization to request that `chunk_loader` load the initial set of chunks.
    fn request_load_chunks(&self) {
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
            self.chunk_loader.request(cc);
        }
    }
}

fn clone_chunk_tile_blocks(chunk_tile_blocks: &ChunkBlocks, game: &Arc<GameData>) -> ChunkBlocks {
    let mut chunk_tile_blocks_clone = ChunkBlocks::new(&game.blocks);
    for lti in 0..=MAX_LTI {
        chunk_tile_blocks.raw_meta::<()>(lti);
        chunk_tile_blocks_clone.raw_set(lti, chunk_tile_blocks.get(lti), ());
    }
    chunk_tile_blocks_clone
}
