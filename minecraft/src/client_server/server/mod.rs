
pub mod event;
pub mod connection;
mod chunk_loader;
mod chunk_manager;


use self::{
    connection::{
        Connection,
        NetworkEvent,
        NetworkServer,
    },
    chunk_loader::{
        ChunkLoader,
        LoadChunkEvent,
    },
    chunk_manager::ChunkManager,
    event::{
        Event,
        EventSenders,
        EventReceiver,
        event_channel,
    },
};
use super::{
    message::*,
    client,
};
use crate::{
    game_data::GameData,
    util::{
        sparse_vec::SparseVec,
        chunk_range::ChunkRange,
    },
    save_file::{
        SaveFile,
        WriteEntry,
    },
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
    thread,
    mem::replace,
};
use tokio::runtime::Handle;
use anyhow::Result;
use slab::Slab;
use vek::*;


const TICK: Duration = Duration::from_millis(50);
const TICKS_BETWEEN_SAVES: u64 = 10 * 20;

const LOAD_Y_START: i64 = 0;
const LOAD_Y_END: i64 = 2;
const INITIAL_LOAD_DISTANCE: i64 = 12;
const LOAD_DISTANCE: i64 = 12;


/// Spawn a new thread which runs a server forever without it being open to
/// the network, only open to an in-mem connection, which is returned.
pub fn spawn_internal_server(
    data_dir: &DataDir,
    game: &Arc<GameData>,
) -> client::connection::Connection {
    let (send_event, recv_event) = event_channel();
    let (network_server, client) = NetworkServer::new_internal(send_event.network_sender());
    let data_dir = data_dir.clone();
    let game = Arc::clone(&game);
    thread::spawn(move || {
        if let Err(e) = run_server(send_event, recv_event, network_server, &data_dir, &game) {
            error!(?e, "internal server crashed");
        }
    });
    client
}

/// Spawn a server, open it to the network, and attempt to run it forever in
/// the current thread.
pub fn run_networked_server(
    rt: &Handle,
    data_dir: &DataDir,
    game: &Arc<GameData>,
) -> Result<()> {
    let (send_event, recv_event) = event_channel();
    let network_server = NetworkServer::new_networked(send_event.network_sender(), "127.0.0.1:35565", rt, game);
    run_server(send_event, recv_event, network_server, data_dir, game)
}

fn run_server(
    send_event: EventSenders,
    recv_event: EventReceiver,
    network_server: NetworkServer,
    data_dir: &DataDir,
    game: &Arc<GameData>,
) -> Result<()> {
    Server::new(send_event, recv_event, data_dir, game)?.run()
}


struct Server {
    game: Arc<GameData>,
    recv_event: EventReceiver,

    // tick management
    tick: u64,
    next_tick: Instant,

    // chunk management
    chunk_mgr: ChunkManager,
    save: SaveFile,
    last_tick_saved: u64,

    // tick state
    tile_blocks: PerChunk<ChunkBlocks>,

    // connection management

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

    // mapping from client to clientside client key spaces
    // which then maps from clientside client key to serverside client key
    client_clientside_client_keys: SparseVec<Slab<usize>>,
    // maping from client A to client B to client A's clientside client key for B
    client_client_clientside_keys: SparseVec<SparseVec<usize>>,

    // client state
    
    // mapping from client to username
    client_username: SparseVec<String>,
    // mapping from username to client
    username_client: HashMap<String, usize>,

    client_char_state: SparseVec<CharState>,
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
        send_event: EventSenders,
        recv_event: EventReceiver,
        data_dir: &DataDir,
        game: &Arc<GameData>,
    ) -> Result<Self> {
        let save = SaveFile::open("server", data_dir, game)?;

        Ok(Server {
            game: Arc::clone(&game),
            recv_event,
            
            tick: 0,
            next_tick: Instant::now(),
            
            chunk_mgr: ChunkManager::new(ChunkLoader::new(send_event.chunk_sender(), &save, game)),
            save,
            last_tick_saved: 0,

            tile_blocks: PerChunk::new(),
            
            all_connections: SparseVec::new(),
            uninit_connections: Slab::new(),
            client_connections: Slab::new(),
            
            conn_last_processed: SparseVec::new(),
            conn_last_processed_increased: SparseVec::new(),
            
            client_clientside_client_keys: SparseVec::new(),
            client_client_clientside_keys: SparseVec::new(),

            client_username: SparseVec::new(),
            username_client: HashMap::new(),
            
            client_char_state: SparseVec::new(),    
        })
    }

    /// Run the server forever.
    fn run(&mut self) -> ! {
        self.request_load_initial_chunks();
    
        loop {
            trace!("doing tick");
            self.do_tick();
            self.update_time_stuff_after_doing_tick();
            self.maybe_save();
            self.process_events_until_next_tick();
        }
    }

    /// Do a tick.
    fn do_tick(&mut self) {
    }

    /// Drain and process chunk manager's effect queue.
    fn process_chunk_mgr_effects(&mut self) {
        while let Some(effect) = self.chunk_mgr.effects.pop_front() {
            match effect {
                chunk_manager::Effect::AddChunk {
                    ready_chunk,
                    ci,
                } => {
                    self.tile_blocks.add(ready_chunk.cc, ci, ready_chunk.chunk_tile_blocks);
                }
                chunk_manager::Effect::RemoveChunk {
                    cc,
                    ci,
                } => {
                    self.tile_blocks.remove(cc, ci);
                }
                chunk_manager::Effect::AddChunkToClient {
                    cc,
                    ci,
                    client_key,
                    clientside_ci,
                } => {
                    self.send_load_chunk_message(
                        cc,
                        clientside_ci,
                        self.tile_blocks.get(cc, ci),
                        &self.client_connections[client_key],
                    );
                }
                chunk_manager::Effect::RemoveChunkFromClient {
                    cc,
                    ci: _,
                    client_key,
                    clientside_ci,
                } => {
                    self.client_connections[client_key].send(down::RemoveChunk {
                        cc,
                        ci: clientside_ci,
                    });
                }
            }
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

        trace!("saving");
        self.last_tick_saved = self.tick;

        let writes = self.chunk_mgr.iter_unsaved()
            .map(|(cc, ci, _)| WriteEntry::Chunk(
                cc,
                clone_chunk_tile_blocks(self.tile_blocks.get(cc, ci), &self.game),
            ));
        self.save.write(writes).unwrap(); // TODO: don't panic
        
        for (cc, ci) in self.chunk_mgr
            .iter_unsaved()
            .map(|(cc, ci, _)| (cc, ci)).collect::<Vec<_>>()
        {
            self.chunk_mgr.mark_saved(cc, ci);
            self.process_chunk_mgr_effects();
        }
    }

    /// Wait for and process events until `self.next_tick`.
    fn process_events_until_next_tick(&mut self) {
        while let Some(event) = self.recv_event.recv_any_by(self.next_tick) {
            match event {
                Event::Network(event) => self.on_first_available_network_event(event),
                Event::LoadChunk(event) => self.on_load_chunk_event(event),
            }
        }
    }

    /// Called when transitioning from not processing network event(s) back-to
    /// back to doing so.
    fn on_first_available_network_event(&mut self, event: NetworkEvent) {
        self.on_network_event(event);
        while let Some(event) = self.recv_event.recv_network_now() {
            self.on_network_event(event);
        }
        self.after_process_available_network_events();
    }

    /// Process any single network event.
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

        // tell chunk manager it's gone
        let char_state = self.client_char_state[client_conn_key];
        self.chunk_mgr.remove_client(client_conn_key, char_load_range(char_state).iter());
        self.process_chunk_mgr_effects();


        // remove client from other clients
        for (client_conn_key2, client_conn2) in self.client_connections.iter() {
            let clientside_client_key = self.client_client_clientside_keys[client_conn_key2].remove(client_conn_key);
            self.client_clientside_client_keys[client_conn_key2].remove(clientside_client_key);
            client_conn2.send(down::RemoveClient {
                client_key: clientside_client_key,
            });
        }

        // remove from other data structures
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

        // uniqueify username
        if self.username_client.contains_key(&username) {
            let mut i = 2;
            let mut username2;
            while {
                username2 = format!("{}{}", username, i);
                self.username_client.contains_key(&username2)
            } { i += 1 }
            username = username2;
        }

        // move from uninit connection space to client connection space
        let conn = self.uninit_connections.remove(uninit_conn_key);
        let client_conn_key = self.client_connections.insert(conn);
        self.all_connections.set(all_conn_key, (ConnectionState::Client, client_conn_key));

        // tell chunk manager it's here
        self.chunk_mgr.add_client(client_conn_key);
        self.process_chunk_mgr_effects();

        // tell chunk manager about it's chunk interests
        for cc in char_load_range(char_state).iter() {
            self.chunk_mgr.add_chunk_client_interest(client_conn_key, cc);
            self.process_chunk_mgr_effects();
        }

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
        let tile = match self.chunk_mgr.getter().gtc_get(gtc) {
            Some(tile) => tile,
            None => {
                info!("client tried SetTileBlock on non-present gtc");
                return;
            }
        };

        // set tile block
        tile.get(&mut self.tile_blocks).raw_set(bid, ());

        // send update to all clients with that chunk loaded
        for clientside in self.chunk_mgr.iter_chunk_clientsides(tile.cc, tile.ci) {
            let ack = if self.conn_last_processed_increased[all_conn_key] {
                self.conn_last_processed_increased[all_conn_key] = false;
                Some(self.conn_last_processed[all_conn_key])
            } else {
                None
            };
            self.client_connections[clientside.client_key].send(down::ApplyEdit {
                ack,
                ci: clientside.clientside_ci,
                edit: edit::SetTileBlock {
                    lti: tile.lti,
                    bid,
                }.into(),
            });
            self.conn_last_processed_increased[all_conn_key] = false;
        }

        // mark chunk as unsaved    
        self.chunk_mgr.mark_unsaved(tile.cc, tile.ci);
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

        // update
        let old_char_state = replace(&mut self.client_char_state[client_conn_key], char_state);
        
        // broadcast
        for (_, client_conn2) in self.client_connections.iter() {
            client_conn2.send(down::SetCharState {
                client_key: client_conn_key,
                char_state,
            });
        }

        // update chunk interests
        let old_char_load_range = char_load_range(old_char_state);
        let new_char_load_range = char_load_range(char_state);
        for cc in old_char_load_range.iter_diff(new_char_load_range) {
            self.chunk_mgr.remove_chunk_client_interest(client_conn_key, cc);
            self.process_chunk_mgr_effects();
        }
        for cc in new_char_load_range.iter_diff(old_char_load_range) {
            self.chunk_mgr.add_chunk_client_interest(client_conn_key, cc);
            self.process_chunk_mgr_effects();
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

    fn on_load_chunk_event(&mut self, event: LoadChunkEvent) {
        if let Some(ready_chunk) = event.get().unwrap() { // TODO: don't panic (like this)
            self.chunk_mgr.on_ready_chunk(ready_chunk);
            self.process_chunk_mgr_effects();
        } 
    }

    /// Request the loading of the initial set of chunks.
    fn request_load_initial_chunks(&mut self) {
        let ccs = ChunkRange {
            start: [-INITIAL_LOAD_DISTANCE, LOAD_Y_START, INITIAL_LOAD_DISTANCE].into(),
            end: [INITIAL_LOAD_DISTANCE, LOAD_Y_END, INITIAL_LOAD_DISTANCE].into(),
        };

        let mut ccs = ccs.iter().collect::<Vec<_>>();
        ccs.sort_by_key(|cc| (cc.x * cc.x + cc.z * cc.z, -cc.y));

        for cc in ccs {
            self.chunk_mgr.incr_load_request_count(cc);
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

fn char_load_range(char_state: CharState) -> ChunkRange {
    let char_cc = (char_state.pos / CHUNK_EXTENT.map(|n| n as f32)).map(|n| n.floor() as i64);
    ChunkRange {
        start: Vec3 {
            x: char_cc.x - LOAD_DISTANCE,
            y: LOAD_Y_START,
            z: char_cc.z - LOAD_DISTANCE,
        },
        end: Vec3 {
            x: char_cc.x + LOAD_DISTANCE + 1,
            y: LOAD_Y_END,
            z: char_cc.z + LOAD_DISTANCE + 1,
        },
    }
}
