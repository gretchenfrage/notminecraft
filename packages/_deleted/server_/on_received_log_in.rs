
use super::{
    Server,
    OnReceived,
    per_connection::*,
    util_load_range::{
        char_load_range,
        dist_sorted_ccs,
    },
};
use crate::{
    message::*,
    item::ItemStack,
    util::array::array_from_fn,
    save_file::read_key,
};
use slab::Slab;
use anyhow::*;


impl OnReceived<up::LogIn> for Server {
    type Ck = UninitConnKey;

    fn on_received(&mut self, msg: up::LogIn, ck: UninitConnKey) -> Result<()> {
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

        // look up in save file to decide its initial state
        // TODO: do this asynchronously
        let (
            pos,
            inventory_slots,
        ) = self.save.read(read_key::Player(username.clone()))?
            .map(|player_data| (
                player_data.pos,
                player_data.inventory_slots,
            ))
            .unwrap_or_else(|| (
                [0.0, 80.0, 0.0].into(),
                array_from_fn(|_| None),
            ));
        let char_state = CharState {
            pos,
            pitch: f32::to_radians(-30.0),
            yaw: f32::to_radians(0.0),
            pointing: false,
            load_dist: 6,
        };

        // accept login
        self.connections[ck].send(down::AcceptLogin {
            inventory_slots: inventory_slots.clone(),
        });

        // transition connection state
        let ck = self.conn_states.transition_to_client(ck);

        // insert into data structures TODO factor this elsewhere
        self.in_game.insert(ck, false);
        self.player_saved.insert(ck, false);
        self.char_states.insert(ck, char_state);
        self.inventory_slots.insert(ck, inventory_slots);
        self.open_game_menu.insert(ck, None);
        self.held.insert(ck, None);

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

        // debugging
        self.connections[ck].send(down::ApplyEdit {
            ack: None,
            edit: edit::InventorySlot {
                slot_idx: 2,
                edit: inventory_slot_edit::SetInventorySlot {
                    slot_val: Some(ItemStack::new(
                        self.game.content.stone.iid_stone,
                        (),
                    )),
                }.into(),
            }.into(),
        });

        Ok(())
    }
}
