//! Process player messages.
//!
//! This is a "game logic" module moreso than a "system" module.

use crate::{
    server::{
        per_player::*,
        SyncWorld,
    },
    message::*,
};


/// Process a player message from a joined player.
pub fn process_player_msg(world: &mut SyncWorld, pk: JoinedPlayerKey, msg: PlayerMsg) {
    match msg {
        PlayerMsg::SetCharState(inner) => inner.process(world, pk),
        PlayerMsg::SetTileBlock(inner) => inner.process(world, pk),
    }
}

// internal trait for player msg variants
trait Process {
    fn process(self, world: &mut SyncWorld, pk: JoinedPlayerKey);
}

impl Process for PlayerMsgSetCharState {
    // set char state
    fn process(self, world: &mut SyncWorld, pk: JoinedPlayerKey) {
        // as a temporary situation we kind of handle this manually
        let PlayerMsgSetCharState { pos, yaw, pitch } = self;

        world.server_only.player_pos[pk] = pos;
        world.server_only.player_yaw[pk] = yaw;
        world.server_only.player_pitch[pk] = pitch;

        for pk2 in world.sync_ctx.conn_mgr.players().iter() {
            world.sync_ctx.conn_mgr.send(pk2, DownMsg::ApplyEdit(Edit::SetPlayerCharState {
                player_idx: DownPlayerIdx(
                    world.sync_ctx.conn_mgr.player_to_clientside(pk, pk2)
                ),
                pos,
                yaw,
                pitch,
            }));
        }
    }
}

impl Process for PlayerMsgSetTileBlock {
    // set tile block
    fn process(self, world: &mut SyncWorld, _pk: JoinedPlayerKey) {
        let PlayerMsgSetTileBlock { gtc, bid_meta } = self;

        if let Some(tile) = world.getter.gtc_get(gtc) {
            tile.get(&mut world.tile_blocks).erased_set(bid_meta);
        }
    }
}
