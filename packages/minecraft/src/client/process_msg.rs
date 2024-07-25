//! Processing pre-join and post-join messages from the server. Mostly integration logic.

use crate::{
    client::*,
    message::*,
    sync_state_entities::*,
    server::tick_mgr::TICK,
};
use chunk_data::*;
use anyhow::{Result, ensure, anyhow};


/// Process a pre join msg received from the network. Error indicates server protocol violation.
pub fn process_pre_join_msg(client: &mut PreJoinClient, msg: PreJoinDownMsg) -> Result<()> {
    match msg {
        // finalize tick
        PreJoinDownMsg::TickDone { next_tick_num, skip_next } => {
            client.next_tick_num = client.next_tick_num.checked_add(1)
                .ok_or_else(|| anyhow!("tick num overflowed"))?;
            ensure!(client.next_tick_num == next_tick_num, "unexpected tick num");
            client.next_tick_instant = skip_next
                .checked_add(1)
                .and_then(|i| u32::try_from(i).ok())
                .and_then(|i| TICK.checked_mul(i))
                .and_then(|d| client.next_tick_instant.checked_add(d))
                .ok_or_else(|| anyhow!("tick instant overflowed"))?;

            client.caught_up_to = client.next_tick_instant - TICK;
            client.next_catch_up_tick = client.next_tick_instant;
            client.tick_just_finished = true;
            /*
            client.just_finished_tick = Some(client.next_tick_instant);*/
        },
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
            client.chunk_newly_added.add(cc, ci, true);
            client.tile_blocks.add(cc, ci, chunk_tile_blocks);
            client.chunk_mesh_mgr.add_chunk(cc, ci, &client.chunks, &client.tile_blocks);
            client.entities
                .add_chunk(
                    &mut client.chunk_steves,
                    cc, ci,
                    steves.into_iter()
                        .map(|entity| (SteveEntityClientState::new(cc, &entity), entity)),
                )
                .map_err(|_| anyhow!("server inserted entities with colliding uuids"))?;
            client.entities
                .add_chunk(
                    &mut client.chunk_pigs,
                    cc, ci,
                    pigs.into_iter()
                        .map(|entity| (PigEntityClientState::new(cc, &entity), entity)),
                )
                .map_err(|_| anyhow!("server inserted entities with colliding uuids"))?;
        }
        // remove chunk from world
        PreJoinDownMsg::RemoveChunk(DownMsgRemoveChunk { chunk_idx }) => {
            let (cc, ci) = client.chunks.on_remove_chunk(chunk_idx)?;
            client.chunk_newly_added.remove(cc, ci);
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
        PreJoinDownMsg::AddEntity { chunk_idx, entity } => {
            let (cc, ci, _getter) = client.chunks.lookup(chunk_idx)?;
            let EntityData { uuid, rel_pos, state } = entity;
            match state {
                AnyEntityState::Steve(state) => {
                    let entity = EntityData { uuid, rel_pos, state };
                    client.entities.add_entity(
                        &mut client.chunk_steves,
                        SteveEntityClientState::new(cc, &entity),
                        entity,
                        cc, ci,
                    )
                },
                AnyEntityState::Pig(state) => {
                    let entity = EntityData { uuid, rel_pos, state };
                    client.entities.add_entity(
                        &mut client.chunk_pigs,
                        PigEntityClientState::new(cc, &entity),
                        entity,
                        cc, ci,
                    )
                },
            }.map_err(|sync_state_entities::UuidCollision|
                anyhow!("server added entity with duplicate uuid")
            )?;
        }
        PreJoinDownMsg::RemoveEntity { chunk_idx, entity_type, vector_idx } => {
            let (cc, ci, _getter) = client.chunks.lookup(chunk_idx)?;
            match entity_type {
                EntityType::Steve => client.entities
                    .remove_entity(&mut client.chunk_steves, cc, ci, vector_idx),
                EntityType::Pig => client.entities
                    .remove_entity(&mut client.chunk_pigs, cc, ci, vector_idx),
            }.map_err(|sync_state_entities::VectorIdxOutOfBounds|
                anyhow!("server removed entity with out of bounds index")
            )?;
        },
        PreJoinDownMsg::ChangeEntityOwningChunk {
            old_chunk_idx,
            entity_type,
            vector_idx,
            new_chunk_idx,
        } => {
            let (old_cc, old_ci, _) = client.chunks.lookup(old_chunk_idx)?;
            let (new_cc, new_ci, _) = client.chunks.lookup(new_chunk_idx)?;
            match entity_type {
                EntityType::Steve => client.entities.move_entity(
                    &mut client.chunk_steves, old_cc, old_ci, new_cc, new_ci, vector_idx,
                ),
                EntityType::Pig => client.entities.move_entity(
                    &mut client.chunk_pigs, old_cc, old_ci, new_cc, new_ci, vector_idx,
                ),
            }.map_err(|sync_state_entities::VectorIdxOutOfBounds|
                anyhow!("server moved entity with out of bounds index")
            )?;
        },
        PreJoinDownMsg::EditEntity { chunk_idx, vector_idx, edit } => {
            fn edit_entity<S, E, F: FnOnce(&mut EntityData<S>, &mut E)>(
                chunk_entities: &mut PerChunk<Vec<sync_state_entities::ChunkEntityEntry<S, E>>>,
                cc: Vec3<i64>,
                ci: usize,
                vector_idx: usize,
                edit: F,
            ) -> Result<()> {
                let entry = chunk_entities.get_mut(cc, ci).get_mut(vector_idx)
                    .ok_or_else(|| anyhow!("server edited entity with out of bounds index"))?;
                edit(&mut entry.entity, &mut entry.extra);
                Ok(())
            }

            let (cc, ci, _getter) = client.chunks.lookup(chunk_idx)?;
            match edit {
                AnyEntityEdit::SetRelPos { entity_type, rel_pos } => match entity_type {
                    EntityType::Steve => edit_entity(
                        &mut client.chunk_steves, cc, ci, vector_idx, |e, _| e.rel_pos = rel_pos
                    ),
                    EntityType::Pig => edit_entity(
                        &mut client.chunk_pigs, cc, ci, vector_idx, |e, _| e.rel_pos = rel_pos
                    ),
                },
                AnyEntityEdit::Steve(edit) => edit_entity(
                    &mut client.chunk_steves, cc, ci, vector_idx, |e, _| match edit {
                        SteveEntityEdit::SetVel(v) => e.state.vel = v,
                        SteveEntityEdit::SetName(v) => e.state.name = v,
                    }
                ),
                AnyEntityEdit::Pig(edit) => edit_entity(
                    &mut client.chunk_pigs, cc, ci, vector_idx, |e, _| match edit {
                        PigEntityEdit::SetVel(v) => e.state.vel = v,
                        PigEntityEdit::SetColor(v) => e.state.color = v,
                    }
                ),
            }?;
        },
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
