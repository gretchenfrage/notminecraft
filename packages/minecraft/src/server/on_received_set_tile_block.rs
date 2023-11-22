
use super::{
    Server,
    OnReceived,
    per_connection::*,
};
use crate::message::*;
use anyhow::*;


impl OnReceived<up::SetTileBlock> for Server {
    type Ck = ClientConnKey;

    fn on_received(&mut self, msg: up::SetTileBlock, _: ClientConnKey) -> Result<()> {
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
                    edit: edit::Tile {
                        ci: clientside_ci,
                        lti: tile.lti,
                        edit: tile_edit::SetTileBlock {
                            bid_meta: self.game.clone_erased_tile_block(&bid_meta),
                        }.into(),
                    }.into(),
                });
            }
        }

        // set tile block
        tile.get(&mut self.tile_blocks).erased_set(bid_meta);

        // mark chunk as unsaved    
        self.chunk_mgr.mark_unsaved(tile.cc, tile.ci);

        Ok(())
    }
}
