//! See `ConnMgr`.

use crate::server::per_player::*;
use std::collection::HashMap;
use anyhow::*;


/// Manages clients and their joining and leaving.
///
/// Bridges from the "connection" layer of abstraction to the "player" layer of abstraction.
///
/// Responsible for:
///
/// - Processing network events and storing network connections.
/// - The serverside part of clients logging in and joining the game.
/// - Tracking state for the server acknowledging to clients their message has been processed.
/// - Cross-linking clients with each other, ie. managing clientside player indexes.
///
/// A network connection can basically be in 1 of 3 states:
///
/// 1. Not a player.
///
///    A connection begins in this state. If a player is kicked their connection may briefly re-
///    enter this state before it is destroyed completely. This is basically entirely untrusted.
///
/// 2. Player, but has not "joined the game".
///
///    This is basically intended to be an intermediate state when the client is loading the world
///    data. Chunks, players, and other state may be loaded into the client when it is in this
///    state, and it may receive information about subsequent state changes to these resources.
///    However, the player's body is not placed into the world, the client cannot perform actions,
///    and their existence is not visible to other players.
///
/// 3. Player, and has "joined the game".
///
///    In this state, the client is fully in the game, can both receive updates about the world
///    state and affect the world state by sending actions, has a body in the world which is
///    simulated, and is known to other players.
///
/// This manager guarantees that client A will be aware of client B iff client B has joined the
/// game or client B is client A. 
pub struct ConnMgr {
    // state for each network connection
    connections: Slab<ConnectionState>
    // allocates player keys. players are subset of connections.
    players: PlayerKeySpace,
    // for each player, their network connection index
    player_conn_idx: PerPlayer<usize>,
    // for each player, their username
    player_username: PerPlayer<String>,
    // inverse of player_username
    username_player: HashMap<String, PlayerKey>,
    // for each player, whether they have joined the game
    player_joined_game: PerPlayer<bool>,
    // for each player, their clientside space of players.
    // player A -> A's clientside player idx for B, if exists -> player B
    player_clientside_players: PerPlayer<Slab<PlayerKey>>,
    // some sort of inverse-like thing of player_clientside_players.
    // player A -> player B -> A's clientside player idx for B, if exists.
    player_player_clientside_player_idx: PerPlayer<PerPlayer<Option<usize>>>,
}

// state for each network connection
struct ConnectionState {
    // handle to the actual connection
    connection: Connection,
    // message index from the client the server has processed up to and including
    last_processed: u64,
    // whether the current value of last_processed has been acked to the client
    last_processed_acked: bool,
    // corresponding player key, if logged in.
    pk: Option<PlayerKey>,
    // whether this connection previously logged in but then was killed. prevents subsequent log in
    // attempts. is mutually exclusive with pk.
    killed: bool,
}

/// Instruction flowing from the `ConnMgr` to the rest of the server.
#[must_use]
pub enum ConnMgrEffect {
    /// Add a new player.
    AddPlayer(PlayerKey),
    /// Process a message from a currently existing player which has joined the game.
    PlayerMsg(PlayerKey, PlayerMsg),
    /// Remove an existing player.
    RemovePlayer(PlayerKey),
}

impl ConnMgr {
    /// Construct with defaults.
    pub fn new() -> Self {
        ConnMgr {
            players: PlayerKeySpace::default(),
        }
    }

    /// Get the player key space.
    pub fn players(&self) -> &PlayerKeySpace {
        &self.players
    }

    /// Process a received network event and optionally return an effect for the caller to process.
    pub fn handle_network_event(&mut self, network_event: NetworkEvent) -> Option<ConnMgrEffect> {
        match network_event {
            NetworkEvent::AddConnection(conn_idx, connection) => {
                // connection created, add it
                let conn_idx2 = self.connections.insert(ConnectionState {
                    connection,
                    last_processed: 0,
                    last_processed_acked: true,
                    pk: None,
                    killed: false,
                });
                debug_assert_eq!(conn_idx, conn_idx2, "NetworkEvent::AddConnection idx mismatch");
                None
            }
            NetworkEvent::Message(conn_idx, msg) => {
                // received message
                // try process
                try_handle_msg(conn_idx, msg)
                    .unwrap_or_else(|e| {
                        // on error, kill connection
                        warn!(%e, "client protocol error, closing connection");
                        self.kill_connection_conn_idx(conn_idx)
                    })
            }
            NetworkEvent::RemoveConnection(conn_idx) => {
                // connection stopped, remove it
                let pk = self.connections.remove(conn_idx).pk;
                if let Some(pk) = pk {
                    // remove associated player
                    self.remove_player(pk);
                    Some(ConnMgrEffect::RemovePlayer(pk))
                } else {
                    None
                }
            }
        }
    }

    // internal method to try to process a message from a network connection. error indicates that
    // the network connection should be terminated.
    fn try_handle_msg(&mut self, conn_idx: usize, msg: UpMsg) -> Result<Option<ConnMgrEffect>> {
        if self.connections[conn_idx].killed {
            // this is likely superfluous, but possibly a good idea for defensiveness.
            return Ok(None);
        }
        match msg {
            UpMsg::LogIn(UpMsgLogIn { username }) => {
                self.try_handle_log_in(conn_idx, msg)?;
                Ok(None)
            }
        }
    }

    // internal method to try to process a log in message from a non-killed connection. error
    // indicates that the network connection should be terminated.
    fn try_handle_log_in(&mut self, conn_idx: usize, msg: UpMsgLogIn) -> Result<()> {
        // prepare and validate
        let UpMsgLogIn { username } = msg;
        ensure!(self.connections[conn_idx].pk.is_none(), "client tried to log in twice");

        // uniqueify username
        let username = uniqueify_username(username, &self.username_player);
    }

    // internal method to actively terminate a connection and remove the associated player if there
    // is one.
    fn kill_connection(&mut self, conn_idx: usize) -> Option<ConnMgrEffect> {
        // tell the network connection to die. this will abandon allocation of resources to it and
        // will trigger a corresponding `NetworkEvent::RemoveConnection` to happen soon. 
        self.connections[conn_idx].kill();
        self.connections[conn_idx].killed = true;

        // remove the associated player if there is one
        let pk = self.connections[conn_idx].pk.take();
        if let Some(pk) = pk {
            self.remove_player(pk);
            Some(ConnMgrEffect::RemovePlayer(pk))
        } else {
            None
        }
    }

    // internal method to clean up internally when a player is removed.
    fn remove_player(&mut self, pk: PlayerKey) {
        // remove from internal data structures
        self.players.remove(pk);
        self.player_conn_idx.remove(pk);
        let username = self.player_username.remove(pk);
        self.username_player.remove(&username).unwrap();
        self.player_joined_game.remove(pk);
        self.player_clientside_players.remove(pk);
        self.player_player_clientside_player_idx.remove(pk); 
        
        // unlink from other clients
        for pk2 in in self.players.iter() {
            let clientside_player_idx = self.player_player_clientside_player_idx[pk2].remove(pk);
            if let Some(clientside_player_idx) = clientside_player_idx {
                self.player_clientside_players[ck2].remove(clientside_player_idx);
                // including telling those other clients to remove it clientside
                self.connections[self.player_conn_idx[pk2]].connection.send(down::RemovePlayer {
                    player_idx: clientside_player_idx,
                });
            }
        }
    }
}

// temporary method until there's a better decentralized identity system
fn uniqueify_username(mut username: String, usernames: &HashMap<String, PlayerKey>) -> String {
    if usernames.contains_key(&username) {
        let mut i = 2;
        let mut username2;
        while {
            username2 = format!("{}{}", username, i);
            usernames.contains_key(&username2);
        } { i += 1 }
        username2
    } else {
        username
    }
}
