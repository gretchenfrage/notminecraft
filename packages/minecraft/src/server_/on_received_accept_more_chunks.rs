
use super::{
    Server,
    OnReceived,
    per_connection::*,
};
use crate::message::*;
use anyhow::*;


impl OnReceived<up::AcceptMoreChunks> for Server {
    type Ck = ClientConnKey;

    fn on_received(&mut self, msg: up::AcceptMoreChunks, ck: ClientConnKey) -> Result<()> {
        let up::AcceptMoreChunks { number } = msg;
        self.chunk_mgr.increase_client_add_chunk_budget(ck, number);
        self.process_chunk_mgr_effects();
        Ok(())
    }
}
