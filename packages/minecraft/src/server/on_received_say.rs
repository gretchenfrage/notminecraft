
use super::{
    Server,
    OnReceived,
    per_connection::*,
};
use crate::message::*;
use anyhow::*;


impl OnReceived<up::Say> for Server {
    type Ck = ClientConnKey;

    fn on_received(&mut self, msg: up::Say, ck: ClientConnKey) -> Result<()> {
        let up::Say { text } = msg;

        self.broadcast_chat_line(format!("<{}> {}", &self.usernames[ck], text));

        Ok(())
    }
}
