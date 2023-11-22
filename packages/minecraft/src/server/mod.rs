
pub mod event;
pub mod connection;
mod chunk_loader;
mod chunk_manager;
mod per_connection;
mod on_received_accept_more_chunks;
mod on_received_game_menu;
mod on_received_join_game;
mod on_received_log_in;
mod on_received_say;
mod on_received_set_char_state;
mod on_received_set_tile_block;
mod util_load_range;


use self::{
    connection::{
        Connection,
        NetworkEvent,
        NetworkServer,
        ToSocketAddrs,
    },
    chunk_loader::{
        ChunkLoader,
        LoadChunkEvent,
    },
    chunk_manager::ChunkManager,
    event::{
        Event,
        EventSenders,
        EventSender,
        EventReceiver,
        event_channel,
        control::ControlEvent,
    },
    per_connection::*,
    util_load_range::{
        char_load_range,
        LOAD_Y_START,
        LOAD_Y_END,
        INITIAL_LOAD_DISTANCE,
    },
};
use crate::{
    game_data::GameData,
    util::{
        chunk_range::ChunkRange,
        array::array_from_fn,
    },
    save_file::{
        content::PlayerData,
        SaveFile,
        WriteEntry,
        read_key,
    },
    thread_pool::ThreadPool,
    client,
    message::*,
    item::*,
};
use chunk_data::*;
use std::{
    sync::Arc,
    time::{
        Duration,
        Instant,
    },
    collections::HashMap,
    thread::{
        self,
        JoinHandle,
    },
    mem::replace,
};
use tokio::runtime::Handle;
use anyhow::*;
use slab::Slab;
use vek::*;


pub use self::connection::NetworkBindGuard;


const TICK: Duration = Duration::from_millis(50);
const TICKS_BETWEEN_SAVES: u64 = 10 * 20;


/// Handle to a running server thread.
#[derive(Debug)]
pub struct ServerHandle {
    thread: Option<JoinHandle<()>>,
    send_control: EventSender<ControlEvent>,

    tokio: Handle,
    game: Arc<GameData>,

    network_server: NetworkServer,
}

impl ServerHandle {
    /// Start the server thread.
    pub fn start(
        save: SaveFile,
        game: &Arc<GameData>,
        tokio: &Handle,
        thread_pool: &ThreadPool,
    ) -> Self {
        let (send_event, recv_event) = event_channel();
        let network_server = NetworkServer::new(send_event.network_sender());
        let thread = thread::spawn({
            let send_event = send_event.clone();
            let game = Arc::clone(game);
            let thread_pool = thread_pool.clone();
            move || Server::new(save, send_event, recv_event, &thread_pool, &game).run()
        });
        ServerHandle {
            thread: Some(thread),
            send_control: send_event.control_sender(),
            
            tokio: tokio.clone(),
            game: Arc::clone(game),

            network_server,
        }
    }

    /// Stop the server cleanly and wait for it to shut down.
    pub fn stop(mut self) {
        self.inner_stop();
    }

    fn inner_stop(&mut self) {
        self.send_control.send(ControlEvent::Stop);
        if self.thread.take().unwrap().join().is_err() {
            error!("server thread panicked");
        }
    }

    /// Open up the server to the network.
    pub fn open_to_network(
        &self,
        bind_to: impl ToSocketAddrs + Send + Sync + 'static,
    ) -> NetworkBindGuard {
        self.network_server.open_to_network(bind_to, &self.tokio, &self.game)
    }

    /// Open up a connection for a client within the same process.
    pub fn internal_connection(&self) -> client::connection::Connection {
        self.network_server.create_in_mem_client().into()
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        if self.thread.is_some() {
            warn!("ServerHandle dropped without being stopped (stopping now)");
            self.inner_stop();
        }
    }
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
    player_saved: PerClientConn<bool>,
    char_states: PerClientConn<CharState>,
    inventory_slots: PerClientConn<[ItemSlot; 36]>,
    open_game_menu: PerClientConn<Option<OpenGameMenu>>,

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

#[derive(Debug, Copy, Clone)]
struct OpenGameMenu {
    menu: GameMenu,
    open_menu_msg_idx: u64,
    valid: bool,
}


#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
struct IsClosingErr;

// kinda a hack since std::ops::Try isn't stabilized
type IsClosing = std::result::Result<(), IsClosingErr>;

const CLOSING: IsClosing = std::result::Result::Err(IsClosingErr);

const NOT_CLOSING: IsClosing = std::result::Result::Ok(());


trait OnReceived<M> {
    type Ck;

    fn on_received(&mut self, msg: M, ck: Self::Ck) -> Result<()>;
}

impl Server {
    /// Construct. This is expected to be immediately followed by `run`.
    fn new(
        save: SaveFile,
        send_event: EventSenders,
        recv_event: EventReceiver,
        thread_pool: &ThreadPool,
        game: &Arc<GameData>,
    ) -> Self {
        Server {
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
            player_saved: PerClientConn::new(),
            char_states: PerClientConn::new(),
            inventory_slots: PerClientConn::new(),
            open_game_menu: PerClientConn::new(),

            clientside_client_keys: PerClientConn::new(),
            client_clientside_keys: PerClientConn::new(),

            usernames: PerClientConn::new(),
            username_clients: HashMap::new(),
        }
    }

    /// Run the server indefinitely.
    fn run(&mut self) {
        self.request_load_initial_chunks();
    
        while self.run_loop_iteration() == NOT_CLOSING {}

        for ck in self.conn_states.iter_any() {
            // privacy concern
            if ck.state() == ConnState::Closed {
                continue;
            }

            self.connections[ck].send(down::Close {
                message: "Server shutting down.".into(),
            });
        }
    }

    fn run_loop_iteration(&mut self) -> IsClosing {
        trace!("doing tick");
        self.do_tick();
        self.update_time_stuff_after_doing_tick();
        self.maybe_save();
        self.process_events_until_next_tick()?;
        NOT_CLOSING
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
        if self.tick - self.last_tick_saved >= TICKS_BETWEEN_SAVES {
            self.last_tick_saved = self.tick;

            self.save();
        
            for (cc, ci) in self.chunk_mgr
                .iter_unsaved()
                .map(|(cc, ci, _)| (cc, ci)).collect::<Vec<_>>() // TODO: bleh
            {
                self.chunk_mgr.mark_saved(cc, ci, &self.conn_states);
                self.process_chunk_mgr_effects();
            }

            for ck in self.conn_states.iter_client() {
                self.player_saved[ck] = true;
            }
        }
    }

    /// Save unsaved chunks. Only does the actual save operation, doesn't update
    /// other accounting information.
    fn save(&mut self) {
        trace!("saving");
        // TODO: avoid actually looping through the whole world basically
        let writes = self.chunk_mgr.iter_unsaved()
            .map(|(cc, ci, _)| WriteEntry::Chunk(
                cc,
                self.game.clone_chunk_blocks(self.tile_blocks.get(cc, ci)),
            ))
            .chain(self.conn_states.iter_client()
                .filter(|&ck| !self.player_saved[ck])
                .map(|ck| WriteEntry::Player(
                    self.usernames[ck].clone(),
                    PlayerData {
                        pos: self.char_states[ck].pos,
                        inventory_slots: self.inventory_slots[ck].clone(),
                    },
                )));
        self.save.write(writes).unwrap(); // TODO: don't panic
    }

    /// Wait for and process events until `self.next_tick`.
    fn process_events_until_next_tick(&mut self) -> IsClosing {
        while let Some(event) = self.recv_event.recv_any_by(self.next_tick) {
            match event {
                Event::Control(event) => self.on_control_event(event)?,
                Event::Network(event) => self.on_first_available_network_event(event),
                Event::LoadChunk(event) => self.on_load_chunk_event(event),
            }
        }
        NOT_CLOSING
    }

    fn on_control_event(&mut self, event: ControlEvent) -> IsClosing {
        match event {
            ControlEvent::Stop => {
                self.save();
                return CLOSING;
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
        self.player_saved.remove(ck);
        let char_state = self.char_states.remove(ck);
        self.inventory_slots.remove(ck);
        self.open_game_menu.remove(ck);
        
        self.clientside_client_keys.remove(ck);
        self.client_clientside_keys.remove(ck);
        
        for ck2 in self.conn_states.iter_client() {
            if let Some(clientside_client_key) = self.client_clientside_keys[ck2].remove(ck) {
                self.clientside_client_keys[ck2].remove(clientside_client_key);
                // remove from other clients while we're at it
                self.connections[ck2].send(down::RemoveClient {
                    client_key: clientside_client_key,
                });
            }
        }
        
        let username = self.usernames.remove(ck);
        self.username_clients.remove(&username).unwrap();

        // tell chunk manager it's gone
        self.chunk_mgr.remove_client(ck, char_load_range(char_state).iter(), &self.conn_states);
        self.process_chunk_mgr_effects();

        // announce
        self.broadcast_chat_line(&format!("{} left the game", username));
    }

    fn on_received(&mut self, raw_key: usize, msg: UpMessage) {
        let ck = self.conn_states.lookup(raw_key);
        
        if let Err(e) = self.on_received_inner(ck, msg){
            error!(%e, "closing connection due to error processing its message");
            self.connections[ck].send(down::Close {
                message: "Protocol violation.".into(),
            });
            self.conn_states.transition_to_closed(ck);
            if let AnyConnKey::Client(ck) = ck {
                self.on_client_disconnected(ck);
            }
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
            ($self:ident, $msg:ident, $( $variant:ident, )*)=>{
                match $msg {$(
                    UpMessage::$variant(msg) => {
                        let ck = ck.try_into()
                            .map_err(|WrongConnState { actual, .. }| anyhow!(
                                concat!("received ", stringify!($variant), " from {:?} connection"),
                                actual,
                            ))?;
                        <Server as OnReceived<_>>::on_received(self, msg, ck)?;
                    }
                )*}
            };
        }

        delegate!(
            self, msg,

            LogIn,
            JoinGame,
            AcceptMoreChunks,
            SetTileBlock,
            Say,
            SetCharState,
            OpenGameMenu,
            CloseGameMenu,
            GameMenuAction,
        );

        Ok(())
    }

    fn broadcast_chat_line(&self, line: impl ToString) {
        for ck in self.conn_states.iter_client() {
            self.connections[ck].send(down::ChatLine {
                line: line.to_string(),
            });
        }
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
