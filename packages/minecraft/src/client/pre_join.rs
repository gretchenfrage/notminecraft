//! Integration code for pre-join logic only.

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
            //let meshing = client.chunk_mesher.trigger_mesh(
            //    // TODO it would be possibly to change logic in ways that avoid this clone
            //    cc, ci, client.game.clone_chunk_blocks(&chunk_tile_blocks)
            //);
            //client.chunk_mesh_state.add(cc, ci, meshing.into());
            client.tile_blocks.add(cc, ci, chunk_tile_blocks);
            //client.mesh_block_update_queue.add_chunk(cc, ci);
        }
        // remove chunk from world
        PreJoinDownMsg::RemoveChunk(DownMsgRemoveChunk { chunk_idx }) => {
            let (cc, ci) = client.chunks.on_remove_chunk(chunk_idx)?;
            client.tile_blocks.remove(cc, ci);
            //client.chunk_mesh_state.remove(cc, ci);
            //client.mesh_block_update_queue.remove_chunk(cc, ci);
        }
        // apply edit
        PreJoinDownMsg::ApplyEdit(edit) => match edit {
            // set tile block
            Edit::SetTileBlock { chunk_idx, lti, bid_meta } => {
                let (cc, ci, _getter) = client.chunks.lookup(chunk_idx)?;
                TileKey { cc, ci, lti }.get(&mut client.tile_blocks).erased_set(bid_meta);
                // TODO: block remesh queue
            }
            // set char state
            Edit::SetPlayerCharState { player_idx, pos, yaw, pitch } => {
                let pk = client.players.lookup(player_idx)?;
                client.player_pos[pk] = pos;
                client.player_yaw[pk] = yaw;
                client.player_pitch[pk] = pitch;
            }
        }
    }
    Ok(())
}
