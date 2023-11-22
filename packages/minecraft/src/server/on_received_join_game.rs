
use super::{
    Server,
    OnReceived,
    per_connection::*,
};
use crate::message::*;
use anyhow::*;


impl OnReceived<up::JoinGame> for Server {
    type Ck = ClientConnKey;

    fn on_received(&mut self, msg: up::JoinGame, ck: ClientConnKey) -> Result<()> {
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
}
