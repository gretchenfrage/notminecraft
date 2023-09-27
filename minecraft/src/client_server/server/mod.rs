
pub mod event;
pub mod connection;
mod chunk_loader;
mod chunk_manager;
mod per_connection;


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
    per_connection::*,
};
use super::{
    message::*,
    client,
};
use crate::{
    game_data::GameData,
    util::chunk_range::ChunkRange,
    save_file::{
        SaveFile,
        WriteEntry,
    },
    thread_pool::ThreadPool,
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
use anyhow::*;
use slab::Slab;
use vek::*;


const TICK: Duration = Duration::from_millis(50);
const TICKS_BETWEEN_SAVES: u64 = 10 * 20;

const LOAD_Y_START: i64 = 0;
const LOAD_Y_END: i64 = 2;
const INITIAL_LOAD_DISTANCE: i64 = 8;


/// Spawn a new thread which runs a server forever without it being open to
/// the network, only open to an in-mem connection, which is returned.
pub fn spawn_internal_server(
    thread_pool: ThreadPool,
    data_dir: &DataDir,
    game: &Arc<GameData>,
) -> client::connection::Connection {
    let (send_event, recv_event) = event_channel();
    let (_, client) = NetworkServer::new_internal(send_event.network_sender());
    let data_dir = data_dir.clone();
    let game = Arc::clone(&game);
    thread::spawn(move || {
        if let Err(e) = run_server(send_event, recv_event, &thread_pool, &data_dir, &game) {
            error!(?e, "internal server crashed");
        }
    });
    client
}

/// Spawn a server, open it to the network, and attempt to run it forever in
/// the current thread.
pub fn run_networked_server(
    rt: &Handle,
    thread_pool: &ThreadPool,
    data_dir: &DataDir,
    game: &Arc<GameData>,
) -> Result<()> {
    let (send_event, recv_event) = event_channel();
    NetworkServer::new_networked(send_event.network_sender(), "127.0.0.1:35565", rt, game);
    run_server(send_event, recv_event, thread_pool, data_dir, game)
}

fn run_server(
    send_event: EventSenders,
    recv_event: EventReceiver,
    thread_pool: &ThreadPool,
    data_dir: &DataDir,
    game: &Arc<GameData>,
) -> Result<()> {
    Server::new(send_event, recv_event, thread_pool, data_dir, game)?.run()
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

    // tile state
    tile_blocks: PerChunk<ChunkBlocks>,

    // connection management
    conn_states: ConnStates,
    connections: PerAnyConn<Connection>,
    last_processed: PerAnyConn<LastProcessed>,

    in_game: PerClientConn<bool>,
    char_states: PerClientConn<CharState>,

    // client A -> A's clientside client key for B, if exists -> client B
    clientside_client_keys: PerClientConn<Slab<ClientConnKey>>,
    // client A -> client B -> A's clientside client key for B, if exists 
    client_clientside_keys: PerClientConn<PerClientConn<Option<usize>>>,

    usernames: PerClientConn<String>,
    username_clients: HashMap<String, ClientConnKey>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
struct LastProcessed {
    num: u64,
    increased: bool,
}

impl Server {
    /// Construct. This is expected to be immediately followed by `run`.
    fn new(
        send_event: EventSenders,
        recv_event: EventReceiver,
        thread_pool: &ThreadPool,
        data_dir: &DataDir,
        game: &Arc<GameData>,
    ) -> Result<Self> {
        let save = SaveFile::open("server", data_dir, game)?;

        Ok(Server {
            game: Arc::clone(&game),
            recv_event,
            
            tick: 0,
            next_tick: Instant::now(),
            
            chunk_mgr: ChunkManager::new(ChunkLoader::new(thread_pool, || send_event.chunk_sender(), &save, game)),
            save,
            last_tick_saved: 0,

            tile_blocks: PerChunk::new(),

            conn_states: ConnStates::new(),
            connections: PerAnyConn::new(),
            last_processed: PerAnyConn::new(),

            in_game: PerClientConn::new(),
            char_states: PerClientConn::new(),

            clientside_client_keys: PerClientConn::new(),
            client_clientside_keys: PerClientConn::new(),

            usernames: PerClientConn::new(),
            username_clients: HashMap::new(),

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
                chunk_manager::Effect::AddChunk { ready_chunk, ci } => {
                    self.tile_blocks.add(ready_chunk.cc, ci, ready_chunk.chunk_tile_blocks);
                }
                chunk_manager::Effect::RemoveChunk { cc, ci } => {
                    self.tile_blocks.remove(cc, ci);
                }
                chunk_manager::Effect::AddChunkToClient { cc, ci, ck, clientside_ci } => {
                    self.connections[ck].send(down::AddChunk {
                        cc,
                        ci: clientside_ci,
                        chunk_tile_blocks: self.game.clone_chunk_blocks(self.tile_blocks.get(cc, ci)),
                    });
                }
                chunk_manager::Effect::RemoveChunkFromClient { cc, ci: _, ck, clientside_ci } => {
                    self.connections[ck].send(down::RemoveChunk {
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
                self.game.clone_chunk_blocks(self.tile_blocks.get(cc, ci)),
            ));
        self.save.write(writes).unwrap(); // TODO: don't panic
        
        for (cc, ci) in self.chunk_mgr
            .iter_unsaved()
            .map(|(cc, ci, _)| (cc, ci)).collect::<Vec<_>>() // TODO: bleh
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
            NetworkEvent::NewConnection(raw_key, conn) => self.on_new_connection(raw_key, conn),
            NetworkEvent::Disconnected(raw_key) => self.on_disconnected(raw_key),
            NetworkEvent::Received(raw_key, msg) => self.on_received(raw_key, msg),
        }
    }

    /// Process a new connection network event.
    fn on_new_connection(&mut self, raw_key: usize, conn: Connection) {
        let ck = self.conn_states.insert(raw_key);

        self.connections.insert(ck, conn);
        // up msg indices start at 1, so 0 indicates none have been processed
        self.last_processed.insert(ck, LastProcessed { num: 0, increased: false });
    }

    /// Process a disconnection network event.
    fn on_disconnected(&mut self, raw_key: usize) {
        let ck = self.conn_states.lookup(raw_key);
        self.conn_states.remove(ck);
        self.connections.remove(ck);
        self.last_processed.remove(ck);

        match ck {
            AnyConnKey::Uninit(_) => {},
            AnyConnKey::Client(ck) => self.on_client_disconnected(ck),
            AnyConnKey::Closed(_) => {},
        }
    }

    /// Process the disconnection of a client connection.
    fn on_client_disconnected(&mut self, ck: ClientConnKey) {
        // remove from data structures
        self.in_game.remove(ck);
        let char_state = self.char_states.remove(ck);
        
        self.clientside_client_keys.remove(ck);
        self.client_clientside_keys.remove(ck);
        
        for ck2 in self.conn_states.iter_client() {
            if let Some(clientside_client_key) = self.client_clientside_keys[ck2].remove(ck) {
                // remove from other clients while we're at it
                self.connections[ck2].send(down::RemoveClient {
                    client_key: clientside_client_key,
                });
            }
        }
        
        let username = self.usernames.remove(ck);
        self.username_clients.remove(&username).unwrap();

        // tell chunk manager it's gone
        self.chunk_mgr.remove_client(ck, char_load_range(char_state).iter());
        self.process_chunk_mgr_effects();

        // announce
        self.broadcast_chat_line(&format!("{} left the game", username));
    }

    fn on_received(&mut self, raw_key: usize, msg: UpMessage) {
        let ck = self.conn_states.lookup(raw_key);
        
        if let Err(e) = self.on_received_inner(ck, msg) {
            error!(%e, "closing connection due to error processing its message");
            self.connections[ck].send(down::Close {});
            if let AnyConnKey::Client(ck) = ck {
                self.on_client_disconnected(ck);
            }
            self.conn_states.transition_to_closed(ck);
        }
    }

    fn on_received_inner(&mut self, ck: AnyConnKey, msg: UpMessage) -> Result<()> {
        if matches!(ck, AnyConnKey::Closed(_)) {
            trace!("ignoring message from closed connection");
            return Ok(());
        }

        self.last_processed[ck].num += 1;
        self.last_processed[ck].increased = true;

        // there are less awkward alternatives to this
        // but let's not overcomplicate things yet
        if let AnyConnKey::Client(ck) = ck {
            ensure!(
                self.in_game[ck] || matches!(msg, UpMessage::JoinGame(_)),
                "client which hasn't joined game sent message other than JoinGame",
            );
        }

        macro_rules! delegate {
            ($self:ident, $msg:ident, $( $variant:ident $method:ident, )*)=>{
                match $msg {$(
                    UpMessage::$variant(msg) => {
                        let ck = ck.try_into()
                            .map_err(|WrongConnState { actual, .. }| anyhow!(
                                concat!("received ", stringify!($variant), " from {:?} connection"),
                                actual,
                            ))?;
                        self.$method(msg, ck)?;
                    }
                )*}
            };
        }

        delegate!(
            self, msg,

            LogIn on_received_log_in,
            JoinGame on_received_join_game,
            SetTileBlock on_received_set_tile_block,
            Say on_received_say,
            SetCharState on_received_set_char_state,
        );

        Ok(())
    }

    fn on_received_log_in(&mut self, msg: up::LogIn, ck: UninitConnKey) -> Result<()> {
        let up::LogIn { mut username } = msg;

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
        if self.username_clients.contains_key(&username) {
            let mut i = 2;
            let mut username2;
            while {
                username2 = format!("{}{}", username, i);
                self.username_clients.contains_key(&username2)
            } { i += 1 }
            username = username2;
        }

        // transition connection state
        let ck = self.conn_states.transition_to_client(ck);
        
        // decide its initial char state
        let char_state = CharState {
            pos: [0.0, 80.0, 0.0].into(),
            pitch: f32::to_radians(-30.0),
            yaw: f32::to_radians(0.0),
            pointing: false,
            load_dist: 6,
        };

        // insert into data structures
        self.in_game.insert(ck, false);
        self.char_states.insert(ck, char_state);

        self.clientside_client_keys.insert(ck, Slab::new());
        self.client_clientside_keys.insert(ck, self.conn_states.new_mapped_per_client(|_| None));
        for ck2 in self.conn_states.iter_client() {
            if ck2 == ck { continue }
            self.client_clientside_keys[ck2].insert(ck, None);
        }

        self.usernames.insert(ck, username.clone());
        self.username_clients.insert(username, ck);

        self.chunk_mgr.add_client(ck);

        // tell it about every client which has joined the game
        // (which necessarily excludes itself)
        for ck2 in self.conn_states.iter_client() {
            if !self.in_game[ck2] { continue }
            
            let clientside_client_key = self.clientside_client_keys[ck].insert(ck2);
            self.client_clientside_keys[ck][ck2] = Some(clientside_client_key);

            self.connections[ck].send(down::AddClient {
                client_key: clientside_client_key,
                username: self.usernames[ck2].clone(),
                char_state: self.char_states[ck2],
            });
        }

        // tell it about itself
        let own_clientside_client_key = self.clientside_client_keys[ck].insert(ck);
        self.client_clientside_keys[ck][ck] = Some(own_clientside_client_key);

        self.connections[ck].send(down::AddClient {
            client_key: own_clientside_client_key,
            username: self.usernames[ck].clone(),
            char_state: self.char_states[ck],
        });

        // tell chunk manager about it's chunk interests
        // (triggering it to send chunks to the client)
        for cc in dist_sorted_ccs(char_load_range(char_state).iter(), char_state.pos) {
            self.chunk_mgr.add_chunk_client_interest(ck, cc, &self.conn_states);
            self.process_chunk_mgr_effects();
        }

        // tell it to join the game, once it finishes receiving prior messages
        self.connections[ck].send(down::ShouldJoinGame {
            own_client_key: own_clientside_client_key,
        });

        Ok(())
    }

    fn on_received_join_game(&mut self, msg: up::JoinGame, ck: ClientConnKey) -> Result<()> {
        // validate
        let up::JoinGame {} = msg;
        ensure!(
            !self.in_game[ck],
            "client tried to join game redundantly",
        );

        // it's now in the game
        self.in_game[ck] = true;

        // tell every other client about it, not including itself
        for ck2 in self.conn_states.iter_client() {
            if ck2 == ck { continue }

            let clientside_client_key = self.clientside_client_keys[ck2].insert(ck);
            self.client_clientside_keys[ck2][ck] = Some(clientside_client_key);

            self.connections[ck2].send(down::AddClient {
                client_key: clientside_client_key,
                username: self.usernames[ck].clone(),
                char_state: self.char_states[ck],
            });
        }

        // announce
        self.broadcast_chat_line(&format!("{} joined the game", &self.usernames[ck]));

        Ok(())
    }

    fn on_received_set_tile_block(&mut self, msg: up::SetTileBlock, _: ClientConnKey) -> Result<()> {
        let up::SetTileBlock { gtc, bid_meta } = msg;

        // lookup tile
        let tile = match self.chunk_mgr.getter().gtc_get(gtc) {
            Some(tile) => tile,
            None => bail!("client tried SetTileBlock on non-present gtc"),
        };

        // send update to all clients with that chunk loaded
        for ck2 in self.conn_states.iter_client() {
            if let Some(clientside_ci) = self.chunk_mgr.clientside_ci(tile.cc, tile.ci, ck2) {
                let ack = if self.last_processed[ck2].increased {
                    self.last_processed[ck2].increased = false;
                    Some(self.last_processed[ck2].num)
                } else {
                    None
                };
                self.connections[ck2].send(down::ApplyEdit {
                    ack,
                    ci: clientside_ci,
                    edit: edit::SetTileBlock {
                        lti: tile.lti,
                        bid_meta: self.game.clone_erased_tile_block(&bid_meta),
                    }.into(),
                });
            }
        }

        // set tile block
        tile.get(&mut self.tile_blocks).erased_set(bid_meta.bid, bid_meta.meta);

        // mark chunk as unsaved    
        self.chunk_mgr.mark_unsaved(tile.cc, tile.ci);

        Ok(())
    }

    fn on_received_say(&mut self, msg: up::Say, ck: ClientConnKey) -> Result<()> {
        let up::Say { text } = msg;

        self.broadcast_chat_line(format!("<{}> {}", &self.usernames[ck], text));

        Ok(())
    }

    fn broadcast_chat_line(&self, line: impl ToString) {
        for ck in self.conn_states.iter_client() {
            self.connections[ck].send(down::ChatLine {
                line: line.to_string(),
            });
        }
    }

    fn on_received_set_char_state(&mut self, msg: up::SetCharState, ck: ClientConnKey) -> Result<()> {
        let up::SetCharState { char_state } = msg;

        // update
        let old_char_state = replace(&mut self.char_states[ck], char_state);
        
        // broadcast
        for ck2 in self.conn_states.iter_client() {
            if let Some(clientside_client_key) = self.client_clientside_keys[ck2][ck] {
                self.connections[ck2].send(down::SetCharState {
                    client_key: clientside_client_key,
                    char_state,
                });
            }
        }

        // update chunk interests
        let old_char_load_range = char_load_range(old_char_state);
        let new_char_load_range = char_load_range(char_state);
        for cc in old_char_load_range.iter_diff(new_char_load_range) {
            self.chunk_mgr.remove_chunk_client_interest(ck, cc);
            self.process_chunk_mgr_effects();
        }
        for cc in dist_sorted_ccs(new_char_load_range.iter_diff(old_char_load_range), char_state.pos) {
            self.chunk_mgr.add_chunk_client_interest(ck, cc, &self.conn_states);
            self.process_chunk_mgr_effects();
        }

        Ok(())
    }

    /// Called after processing at least one network event and then processing
    /// all subsequent network events that were immediately available without
    /// additional blocking.
    fn after_process_available_network_events(&mut self) {
        // ack
        for ck in self.conn_states.iter_client() {
            if self.last_processed[ck].increased {
                self.connections[ck].send(down::Ack {
                    last_processed: self.last_processed[ck].num,
                });
                self.last_processed[ck].increased = false;
            }
        }
    }

    fn on_load_chunk_event(&mut self, event: LoadChunkEvent) {
        if let Some(ready_chunk) = event.get().unwrap() { // TODO: don't panic (like this)
            self.chunk_mgr.on_ready_chunk(ready_chunk, &self.conn_states);
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
            self.chunk_mgr.incr_load_request_count(cc, &self.conn_states);
        }
    }
}

fn char_load_range(char_state: CharState) -> ChunkRange {
    let char_cc = (char_state.pos / CHUNK_EXTENT.map(|n| n as f32)).map(|n| n.floor() as i64);
    let load_distance = char_state.load_dist as i64;
    ChunkRange {
        start: Vec3 {
            x: char_cc.x - load_distance,
            y: LOAD_Y_START,
            z: char_cc.z - load_distance,
        },
        end: Vec3 {
            x: char_cc.x + load_distance + 1,
            y: LOAD_Y_END,
            z: char_cc.z + load_distance + 1,
        },
    }
}

fn dist_sorted_ccs(ccs: impl IntoIterator<Item=Vec3<i64>>, pos: Vec3<f32>) -> Vec<Vec3<i64>> {
    let mut ccs = ccs.into_iter().collect::<Vec<_>>();
    fn square_dist(a: Vec3<f32>, b: Vec3<f32>) -> f32 {
        (a - b).map(|n| n * n).sum()
    }
    fn cc_square_dist(cc: Vec3<i64>, pos: Vec3<f32>) -> f32 {
        square_dist(
            (cc.map(|n| n as f32) + 0.5) * CHUNK_EXTENT.map(|n| n as f32),
            pos,
        )
    }
    ccs.sort_by(|&cc1, &cc2| cc_square_dist(cc1, pos).total_cmp(&cc_square_dist(cc2, pos)));
    ccs
}
