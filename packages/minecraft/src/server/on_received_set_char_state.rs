
use super::{
    OnReceived,
    Server,
    per_connection::*,
    util_load_range::{
        char_load_range,
        dist_sorted_ccs,
    },
};
use crate::message::*;
use std::mem::replace;
use anyhow::*;


impl OnReceived<up::SetCharState> for Server {
    type Ck = ClientConnKey;

    fn on_received(&mut self, msg: up::SetCharState, ck: ClientConnKey) -> Result<()> {
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
            self.chunk_mgr.remove_chunk_client_interest(ck, cc, &self.conn_states);
            self.process_chunk_mgr_effects();
        }
        for cc in dist_sorted_ccs(new_char_load_range.iter_diff(old_char_load_range), char_state.pos) {
            self.chunk_mgr.add_chunk_client_interest(ck, cc, &self.conn_states);
            self.process_chunk_mgr_effects();
        }

        Ok(())
    }
}
