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
        PreJoinDownMsg::AddChunk(DownMsgAddChunk {
            chunk_idx,
            cc,
            chunk_tile_blocks,
            steves,
            pigs,
        }) => {
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
        // set tile block
        PreJoinDownMsg::SetTileBlock { chunk_idx, lti, bid_meta } => {
            let (cc, ci, getter) = client.chunks.lookup(chunk_idx)?;
            let tile = TileKey { cc, ci, lti };
            tile.get(&mut client.tile_blocks).erased_set(bid_meta);
            client.chunk_mesh_mgr.mark_adj_dirty(&getter, tile.gtc());
        }
        // set char state
        PreJoinDownMsg::SetPlayerCharState { player_idx, pos, yaw, pitch } => {
            let pk = client.players.lookup(player_idx)?;
            client.player_pos[pk] = pos;
            client.player_yaw[pk] = yaw;
            client.player_pitch[pk] = pitch;
        }
        /*PreJoinDownMsg::SetStevePosVel { steve_idx, pos, vel } => {
            let steve = &mut client.steves[steve_idx];
            steve.pos = pos;
            steve.vel = vel;
        }*/
    }
    Ok(())
}

/// Process a post join msg received from the network. Error indicates server protocol violation.
pub fn process_post_join_msg(client: &mut Client, msg: PostJoinDownMsg) -> Result<()> {
    match msg {
        PostJoinDownMsg::Ack { .. } => (), // TODO
        PostJoinDownMsg::InvalidateSyncMenu { up_msg_idx } =>
            client.menu_mgr.on_invalidate_sync_menu_msg(up_msg_idx)?,
        // set item slot
        PostJoinDownMsg::SetItemSlot { item_slot, slot_content } => {
            *match item_slot {
                DownItemSlotRef::Held => &mut client.inventory_slots.held_slot,
                DownItemSlotRef::Inventory(i) =>
                    i.idx_mut(&mut client.inventory_slots.inventory_slots)
            } = slot_content;
        }
    }
    Ok(())
}
