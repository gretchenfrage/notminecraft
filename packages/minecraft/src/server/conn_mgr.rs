//! See `ConnMgr`.

use crate::{
    server::{
        per_player::*,
        network::*,
        save_content::*,
    },
    util_abort_handle::*,
    util_must_drain::MustDrain,
    message::*,
};
use std::{
    collections::{
        HashMap,
        VecDeque,
    },
    mem::replace,
};
use slab::Slab;
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
/// See the `per_player` module for data structures to keep track of these sets of players and
/// their associated state.
///
/// In terms of how this interacts with clients, the key points are:
///
/// - Clients only have a concept of players which have joined the game. Thus, only joined players
///   may be loaded onto clients.
/// - This manager guarantees that all joined players will be loaded for all players of any state.
/// - Therefore, player A has player B loaded iff player B has joined the game.
///
/// ---
///
/// The sequence of events for a client logging in and joining the game, which this manager
/// implements most of, is as such:
///
/// 1. A network connection is created. It gets a connection index within this manager, but the
///    rest of the server doesn't learn about it.
/// 2. LogIn is received from client.
///
///    - Player is created. Rest of the server must update.
///    - AcceptLogIn is sent to client.
///    - All joined players are loaded into client.
///    - Chunks begin being loaded into client.
///    - Request is submitted to save file to read player's saved state.
///
/// 3. Saved player state arrives from save file. It is stored for later.
/// 4. Server reaches necessary conditions for allowing player to join. ShouldJoinGame is sent to
///    client.
/// 5. JoinGame is received from client.
///
///    - Player is joined. Rest of the server must update.
///    - Player is loaded into all clients.
///    - Save file player state taken from where it was stashed and gets used and consumed.
///    - Body within the world starts being simulated.
///    - FinalizeJoinGame is sent to client, including which player is them and any state about
///      themself that only gets loaded for them eg their inventory.
#[derive(Default)]
pub struct ConnMgr {
    pub effects: VecDeque<ConnMgrEffect>,
    // state for each network connection
    connections: Slab<ConnectionState>,
    // allocates player keys. players are subset of connections.
    players: PlayerKeySpace,
    // for each player, their network connection index
    player_conn_idx: PerPlayer<usize>,
    // for each player, their username
    player_username: PerPlayer<String>,
    // inverse of player_username
    username_player: HashMap<String, PlayerKey>,
    // for each player, its state in terms of loading its save state from the save file
    player_load_save_state_state: PerPlayer<PlayerLoadSaveStateState>,
    // for each player, their clientside space of players.
    // player A -> A's clientside player idx for B, if exists (ie. if B joined) -> player B
    player_clientside_players: PerPlayer<Slab<JoinedPlayerKey>>,
    // some sort of inverse-like thing of player_clientside_players.
    // player A -> player B -> A's clientside player idx for B, if exists (ie. if B joined).
    player_player_clientside_player_idx: PerPlayer<PerJoinedPlayer<usize>>,
}

// state for each network connection
struct ConnectionState {
    // handle to the actual connection
    connection: Connection,
    // message index from the client the server has processed up to and including.
    // although this is inclusive, message indexes start at 1, so 0 still represents none.
    last_processed: u64,
    // whether the current value of last_processed has been acked to the client
    last_processed_acked: bool,
    // corresponding player key, if logged in.
    pk: Option<PlayerKey>,
    // whether this connection previously logged in but then was killed. prevents subsequent log in
    // attempts. is mutually exclusive with pk.
    killed: bool,
    // whether a ShouldJoinGame message has been sent to the client and a JoinGame message has not
    // yet been received back from it. this should necessarily imply:
    // - pk is Some
    // - player_load_save_state_state is Stashed
    should_join_game: bool,
}

// state for a player in terms of loading its save state from the save file
enum PlayerLoadSaveStateState {
    // request to load from the save file is pending
    Loading(AbortGuard),
    // is loaded from the save file and ready for when the player joins
    Stashed(Option<PlayerSaveVal>),
    // player has joined and thus ownership of the state has been taken
    Taken,
}

/// Effect flowing from the `ConnMgr` to the rest of the server.
#[derive(Debug)]
pub enum ConnMgrEffect {
    /// A new player was connected. Initialize it in `PerPlayer` structures. Also, set in motion
    /// the process of loading the player's save state, so that `on_player_save_state_ready` is
    /// called in the future, unless aborted.
    AddPlayerRequestLoad {
        pk: PlayerKey,
        save_key: PlayerSaveKey,
        aborted: AbortHandle,
    },
    /// Process an pre join message from an existing player.
    PreJoinMsg(PlayerKey, PreJoinUpMsg),
    /// A previously added player is now joining the game. Initialize it in `PerJoinedPlayer`
    /// structures.
    ///
    /// Do not yet send clients messages about this player, because player cross-linking has not
    /// yet been done. This effect will be immediately followed by the necessary sequence of
    /// AddPlayerToClient effects to do that cross-linking, and then by a `FinalizeJoinPlayer` to
    /// complete the process.
    BeginJoinPlayer {
        /// The newly upgraded key for the joined player.
        pk: JoinedPlayerKey,
        /// The saved player state from the save file, or `None` if the save file did not contain
        /// an entry for this player's identity.
        save_state: Option<PlayerSaveVal>,
    },
    /// See `BeginJoinPlayer`. Send the player a `FinalizeJoinGame` message. It is now safe to send
    /// clients messages about this player.
    FinalizeJoinPlayer {
        /// The player in question.
        pk: JoinedPlayerKey,
        /// The client's clientside player index for itself.
        self_clientside_player_idx: usize,
    },
    /// A player has been added to another player (in terms of cross-linking them) and been
    /// assigned for that client a clientside player idx. Tell the client to add the player.
    AddPlayerToClient {
        /// Client to add the joined player to.
        add_to: PlayerKey,
        /// The joined player to add to the client.
        to_add: JoinedPlayerKey,
        /// The client's newly allocated clientside player index for to_add.
        clientside_player_idx: usize,
    },
    /// Process a message from an existing joined player.
    PlayerMsg(JoinedPlayerKey, PlayerMsg),
    /// Remove an existing player, which may or may not have joined.
    RemovePlayer {
        /// Player key being removed.
        pk: PlayerKey,
        /// Corresponding joined player key, if player joined.
        jpk: Option<JoinedPlayerKey>,
        /// Corresponding username.
        username: String,
    },
}

impl ConnMgr {
    /// Construct.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the player key space.
    pub fn players(&self) -> &PlayerKeySpace {
        &self.players
    }

    /// Enqueue message to be transmitted to player client.
    ///
    /// Never blocks or errors. See `Connection::send` for details.
    pub fn send<K: Into<PlayerKey>, M: Into<DownMsg>>(&self, pk: K, msg: M) {
        self.connections[self.player_conn_idx[pk.into()]].connection.send(msg.into());
    }

    /// Close the player client connection and remove the player.
    pub fn kick<K: Into<PlayerKey>>(&mut self, pk: K) -> MustDrain {
        self.kill_connection(self.player_conn_idx[pk.into()]);
        MustDrain
    }

    /// Get the given player's username.
    pub fn player_username<K: Into<PlayerKey>>(&self, pk: K) -> &str {
        &self.player_username[pk.into()]
    }

    /// Try to look up a player by username.
    pub fn username_player(&self, username: &str) -> Option<PlayerKey> {
        self.username_player.get(username).copied()
    }

    /// Get the clientside player idx by which `subject` refers to `object`.
    pub fn player_to_clientside(&self, object: JoinedPlayerKey, subject: PlayerKey) -> usize {
        self.player_player_clientside_player_idx[subject][object]
    }

    /// Process a received network event and optionally return an effect for the caller to process.
    pub fn handle_network_event(&mut self, network_event: NetworkEvent) -> MustDrain {
        match network_event {
            NetworkEvent::AddConnection(conn_idx, connection) => {
                // connection created, add it
                let conn_idx2 = self.connections.insert(ConnectionState {
                    connection,
                    last_processed: 0,
                    last_processed_acked: true,
                    pk: None,
                    killed: false,
                    should_join_game: false,
                });
                debug_assert_eq!(conn_idx, conn_idx2, "NetworkEvent::AddConnection idx mismatch");
            }
            NetworkEvent::Message(conn_idx, msg) => {
                // received message
                // try process
                let result = self.try_handle_msg(conn_idx, msg);
                if let Err(e) = result {
                    // on error, kill connection
                    warn!(%e, "client protocol error, closing connection");
                    self.kill_connection(conn_idx);
                }
            }
            NetworkEvent::RemoveConnection(conn_idx) => {
                // connection stopped, remove it
                let pk = self.connections.remove(conn_idx).pk;
                if let Some(pk) = pk {
                    // remove associated player
                    self.remove_player(pk);
                }
            }
        }
        MustDrain
    }

    /// If additional messages from the client were processed since processed messages were last
    /// marked as acked, return the current last_processed value and mark it as acked.
    ///
    /// Does not itself actually transmit the ack to the client, as the desired way to do that is
    /// situational. Rather, if returns some, the caller should somehow transmit the returned value
    /// to the client as an ack.
    #[must_use]
    pub fn ack_last_processed<K: Into<PlayerKey>>(&mut self, pk: K) -> Option<u64> {
        let pk = pk.into();
        if !self.connections[self.player_conn_idx[pk]].last_processed_acked {
            self.connections[self.player_conn_idx[pk]].last_processed_acked = true;
            Some(self.connections[self.player_conn_idx[pk]].last_processed)
        } else {
            None
        }
    }

    // internal method to try to process a message from a network connection. error indicates that
    // the network connection should be terminated.
    fn try_handle_msg(&mut self, conn_idx: usize, msg: UpMsg) -> Result<()> {
        if self.connections[conn_idx].killed {
            // this is likely superfluous, but possibly a good idea for defensiveness.
            return Ok(());
        }
        // increment last processed and mark as not acked
        self.connections[conn_idx].last_processed += 1;
        self.connections[conn_idx].last_processed_acked = false;
        // delegate for further processing
        match msg {
            UpMsg::LogIn(msg) => {
                self.try_handle_log_in(conn_idx, msg)
            }
            UpMsg::PreJoin(msg) => {
                self.try_handle_pre_join_msg(conn_idx, msg)
            }
            UpMsg::JoinGame => {
                self.try_handle_join_game(conn_idx)
            }
            UpMsg::PlayerMsg(msg) => {
                self.try_handle_player_msg(conn_idx, msg)
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

        // transmit AcceptLogIn to client
        self.connections[conn_idx].connection.send(DownMsg::AcceptLogIn);
        
        // initialize player key
        let pk = self.players.add();
        self.connections[conn_idx].pk = Some(pk);
        self.player_conn_idx.insert(pk, conn_idx);

        // initialize in username tracking structures
        self.player_username.insert(pk, username.clone());
        self.username_player.insert(username.clone(), pk);

        // initialize in structure for tracking the loading of its save state
        let aborted_1 = AbortGuard::new();
        let aborted_2 = aborted_1.new_handle();
        self.player_load_save_state_state.insert(pk, PlayerLoadSaveStateState::Loading(aborted_1));

        // add the player to the rest of the server and trigger its loading from the save file
        self.effects.push_back(ConnMgrEffect::AddPlayerRequestLoad {
            pk,
            save_key: PlayerSaveKey { username: username.clone() },
            aborted: aborted_2,
        });

        // load all joined players into the new player
        let mut clientside_players = Slab::new();
        let player_clientside_player_idx =
            self.players.new_per_joined_player(|b_pk| {
                let b_clientside_player_idx = clientside_players.insert(b_pk);
                self.effects.push_back(ConnMgrEffect::AddPlayerToClient {
                    add_to: pk,
                    to_add: b_pk,
                    clientside_player_idx: b_clientside_player_idx,
                });
                b_clientside_player_idx
            });
        self.player_clientside_players.insert(pk, clientside_players);
        self.player_player_clientside_player_idx.insert(pk, player_clientside_player_idx);

        Ok(())
    }

    // internal method to try to process a pre join message from a non-killed connection. error
    // indicates that the network connection should be terminated.
    fn try_handle_pre_join_msg(&mut self, conn_idx: usize, msg: PreJoinUpMsg) -> Result<()> {
        // prepare and validate
        let pk = self.connections[conn_idx].pk
            .ok_or_else(|| anyhow!("wrong time to send pre join msg"))?;

        // tell caller to process msg
        self.effects.push_back(ConnMgrEffect::PreJoinMsg(pk, msg));

        Ok(())
    }

    /// Call upon the result of a previously triggered player save state loading operation being
    /// ready, unless aborted.
    pub fn on_player_save_state_ready(&mut self, pk: PlayerKey, save_val: Option<PlayerSaveVal>) {
        // store
        self.player_load_save_state_state[pk] = PlayerLoadSaveStateState::Stashed(save_val);

        // TODO
        //
        // in the future we'd like to also make it so that we wait for the chunk manager to be
        // satisfied with a sufficient amount of chunks being loaded into the client before sending
        // it this. but for now, we'll just tell the player to join as soon as their player save
        // state is retrieved.
        self.connections[self.player_conn_idx[pk]].connection.send(DownMsg::ShouldJoinGame);
        self.connections[self.player_conn_idx[pk]].should_join_game = true;
    }

    // internal method to try to process a join game message from a non-killed connection. error
    // indicates that the network connection should be terminated.
    fn try_handle_join_game(&mut self, conn_idx: usize) -> Result<()> {
        // prepare and validate
        ensure!(self.connections[conn_idx].should_join_game, "wrong time to send join game");
        self.connections[conn_idx].should_join_game = false;
        let pk = self.connections[conn_idx].pk.unwrap();
        let pls3 = replace(
            &mut self.player_load_save_state_state[pk],
            PlayerLoadSaveStateState::Taken,
        );
        let save_state = match pls3 {
            PlayerLoadSaveStateState::Stashed(save_state) => save_state,
            _ => unreachable!(),
        };

        // begin joining the player
        let pk = self.players.join(pk);
        self.effects.push_back(ConnMgrEffect::BeginJoinPlayer { pk, save_state });

        // add the player to all clients, including itself
        for pk2 in self.players.iter() {
            let clientside_player_idx = self.player_clientside_players[pk2].insert(pk);
            self.player_player_clientside_player_idx[pk2].insert(pk, clientside_player_idx);
            self.effects.push_back(ConnMgrEffect::AddPlayerToClient {
                add_to: pk2,
                to_add: pk,
                clientside_player_idx,
            });
        }

        // finish joining the player
        self.effects.push_back(ConnMgrEffect::FinalizeJoinPlayer {
            pk,
            self_clientside_player_idx: self.player_player_clientside_player_idx[pk][pk],
        });

        Ok(())
    }

    // internal method to try to process a player message from a non-killed connection. error
    // indicates that the network connection should be terminated.
    fn try_handle_player_msg(&mut self, conn_idx: usize, msg: PlayerMsg) -> Result<()> {
        // prepare and validate
        let pk = self.connections[conn_idx].pk
            .and_then(|pk| self.players.to_jpk(pk))
            .ok_or_else(|| anyhow!("wrong time to send player msg"))?;

        // tell caller to process msg
        self.effects.push_back(ConnMgrEffect::PlayerMsg(pk, msg));

        Ok(())
    }

    // internal method to actively terminate a connection and remove the associated player if there
    // is one.
    fn kill_connection(&mut self, conn_idx: usize) {
        // tell the network connection to die. this will abandon allocation of resources to it and
        // will trigger a corresponding `NetworkEvent::RemoveConnection` to happen soon. 
        self.connections[conn_idx].connection.kill();
        self.connections[conn_idx].killed = true;

        // remove the associated player if there is one
        let pk = self.connections[conn_idx].pk.take();
        if let Some(pk) = pk {
            self.remove_player(pk);
        }
    }

    // internal method to fully remove and deinitialize a possibly joined player and enqueue a
    // remove player effect for it, other than changing self.connections in an appropriate way,
    // which is dependent on the situation in which this was called.
    fn remove_player(&mut self, pk: PlayerKey) {
        let jpk = self.players.to_jpk(pk);
        let username = self.deinit_player(pk);
        if let Some(jpk) = jpk {
            self.deinit_joined_player(jpk);
        }
        self.effects.push_back(ConnMgrEffect::RemovePlayer { pk, jpk, username });
    }

    // internal method to clean up internally when a player is removed. returns its username.
    fn deinit_player(&mut self, pk: PlayerKey) -> String {
        // remove from internal data structures
        self.players.remove(pk);
        self.player_conn_idx.remove(pk);
        let username = self.player_username.remove(pk);
        self.username_player.remove(&username).unwrap();
        self.player_clientside_players.remove(pk);
        self.player_player_clientside_player_idx.remove(pk);
        username
    }

    // internal method to clean up internally when a joined player is removed
    fn deinit_joined_player(&mut self, pk: JoinedPlayerKey) {
        // unlink from other clients
        for pk2 in self.players.iter() {
            let clientside_player_idx = self.player_player_clientside_player_idx[pk2].remove(pk);
            self.player_clientside_players[pk2].remove(clientside_player_idx);
            self.connections[self.player_conn_idx[pk2]].connection
                .send(DownMsg::PreJoin(PreJoinDownMsg::RemovePlayer(DownMsgRemovePlayer {
                    player_idx: DownPlayerIdx(clientside_player_idx),
                })));
        }
    }
}

// temporary method until there's a better decentralized identity system
fn uniqueify_username(username: String, usernames: &HashMap<String, PlayerKey>) -> String {
    if usernames.contains_key(&username) {
        let mut i = 2;
        let mut username2;
        while {
            username2 = format!("{}{}", username, i);
            usernames.contains_key(&username2)
        } { i += 1 }
        username2
    } else {
        username
    }
}
