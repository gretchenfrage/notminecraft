//! Processing pre-join and post-join messages from the server. Mostly integration logic.

use crate::{
    client::*,
    message::*,
};
use chunk_data::*;
use anyhow::Result;


/// Process a pre join msg received from the network. Error indicates server protocol violation.
pub fn process_pre_join_msg(client: &mut PreJoinClient, msg: PreJoinDownMsg) -> Result<()> {
    match msg {
        // add player to world
        PreJoinDownMsg::AddPlayer(DownMsgAddPlayer {
            player_idx,
            username,
            pos,
            pitch,
            yaw,
        }) => {
            let pk = client.players.on_add_player(player_idx)?;
            client.player_username.insert(pk, username);
            client.player_pos.insert(pk, pos);
            client.player_pitch.insert(pk, pitch);
            client.player_yaw.insert(pk, yaw);
        }
        // remove player from world
        PreJoinDownMsg::RemovePlayer(DownMsgRemovePlayer { player_idx }) => {
            let pk = client.players.on_remove_player(player_idx)?;
            client.player_username.remove(pk);
            client.player_pos.remove(pk);
            client.player_pitch.remove(pk);
            client.player_yaw.remove(pk);
        }
        // add chunk to world
        PreJoinDownMsg::AddChunk(DownMsgAddChunk { chunk_idx, cc, chunk_tile_blocks }) => {
            let (ci, _getter) = client.chunks.on_add_chunk(chunk_idx, cc)?.get(&client.chunks);
            client.tile_blocks.add(cc, ci, chunk_tile_blocks);
            client.chunk_mesh_mgr.add_chunk(cc, ci, &client.chunks, &client.tile_blocks);
        }
        // remove chunk from world
        PreJoinDownMsg::RemoveChunk(DownMsgRemoveChunk { chunk_idx }) => {
            let (cc, ci) = client.chunks.on_remove_chunk(chunk_idx)?;
            client.tile_blocks.remove(cc, ci);
            client.chunk_mesh_mgr.remove_chunk(cc, ci);
        }
        // apply edit
        PreJoinDownMsg::ApplyEdit(edit) => match edit {
            // set tile block
            Edit::SetTileBlock { chunk_idx, lti, bid_meta } => {
                let (cc, ci, getter) = client.chunks.lookup(chunk_idx)?;
                let tile = TileKey { cc, ci, lti };
                tile.get(&mut client.tile_blocks).erased_set(bid_meta);
                client.chunk_mesh_mgr.mark_adj_dirty(&getter, tile.gtc());
            }
            // set char state
            Edit::SetPlayerCharState { player_idx, pos, yaw, pitch } => {
                let pk = client.players.lookup(player_idx)?;
                client.player_pos[pk] = pos;
                client.player_yaw[pk] = yaw;
                client.player_pitch[pk] = pitch;
            }
            // set item slot
            Edit::SetItemSlot { item_slot, slot_content } => {
                // TODO
            }
        }
    }
    Ok(())
}
