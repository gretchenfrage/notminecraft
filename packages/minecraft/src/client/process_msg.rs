//! Processing pre-join and post-join messages from the server. Mostly integration logic.

use crate::{
    client::*,
    message::*,
};
use chunk_data::*;
use anyhow::{Result, ensure, anyhow};


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
            //steves,
            //pigs,
        }) => {
            let (ci, _getter) = client.chunks.on_add_chunk(chunk_idx, cc)?.get(&client.chunks);
            client.tile_blocks.add(cc, ci, chunk_tile_blocks);
            client.chunk_mesh_mgr.add_chunk(cc, ci, &client.chunks, &client.tile_blocks);
            /*
            // TODO: put this somewhere else
            // TODO: also, this is largely duplicated with the server
            fn install_entities<S>(
                cc: Vec3<i64>,
                ci: usize,
                down_vec: Vec<DownEntity<S>>,
                global_entity_hmap: &mut HashMap<Uuid, usize>,
                global_entity_slab: &mut Slab<GlobalEntityEntry>,
                chunk_entities: &mut PerChunk<Vec<EntityEntry<S>>>,
                kind: EntityKind,
            ) -> Result<()> {
                let client_vec = down_vec
                    .into_iter()
                    .enumerate()
                    .map(|(vector_idx, down_entity)| {
                        let DownEntity {
                            entity_uuid: uuid,
                            rel_pos,
                            state,
                        } = down_entity;

                        let global_idx = global_entity_slab
                            .insert(GlobalEntityEntry { uuid, kind, cc, ci, vector_idx });
                        let collision = global_entity_hmap.insert(uuid, global_idx);
                        ensure!(collision.is_none(), "entity UUID collision {}", uuid);

                        Ok(EntityEntry { uuid, global_idx, rel_pos, state })
                    })
                    .collect::<Result<Vec<_>>>()?;
                chunk_entities.add(cc, ci, client_vec);
                Ok(())
            }

            install_entities(
                cc,
                ci,
                steves,
                &mut client.global_entity_hmap,
                &mut client.global_entity_slab,
                &mut client.chunk_steves,
                EntityKind::Steve,
            )?;
            install_entities(
                cc,
                ci,
                pigs,
                &mut client.global_entity_hmap,
                &mut client.global_entity_slab,
                &mut client.chunk_pigs,
                EntityKind::Pig,
            )?;
            */
        }
        // remove chunk from world
        PreJoinDownMsg::RemoveChunk(DownMsgRemoveChunk { chunk_idx }) => {
            let (cc, ci) = client.chunks.on_remove_chunk(chunk_idx)?;
            client.tile_blocks.remove(cc, ci);
            client.chunk_mesh_mgr.remove_chunk(cc, ci);
            /*
            // TODO move this elsewhere
            // TODO: this is mostly duplicated with server
            fn remove_entities<S>(
                chunk_entities: &mut PerChunk<Vec<EntityEntry<S>>>,
                cc: Vec3<i64>,
                ci: usize,
                global_entity_hmap: &mut HashMap<Uuid, usize>,
                global_entity_slab: &mut Slab<GlobalEntityEntry>,
                kind: EntityKind,
            ) {
                for (vector_idx, entry) in chunk_entities.remove(cc, ci).into_iter().enumerate() {
                    let EntityEntry { uuid, global_idx, .. } = entry;

                    let removed = global_entity_hmap.remove(&uuid);
                    debug_assert_eq!(
                        removed, Some(global_idx),
                        "hmap desync detected removing entity {}", uuid,
                    );
                    let global_entry = global_entity_slab.remove(global_idx);
                    debug_assert_eq!(
                        global_entry, GlobalEntityEntry { uuid, kind, cc, ci, vector_idx },
                        "global entry desync detected removing entity {}", uuid,
                    );
                }
            }

            remove_entities(
                &mut client.chunk_steves,
                cc,
                ci,
                &mut client.global_entity_hmap,
                &mut client.global_entity_slab,
                EntityKind::Steve,
            );
            remove_entities(
                &mut client.chunk_pigs,
                cc,
                ci,
                &mut client.global_entity_hmap,
                &mut client.global_entity_slab,
                EntityKind::Pig,
            );
            */
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
        PreJoinDownMsg::AddEntity { .. } => todo!(),
        PreJoinDownMsg::RemoveEntity { .. } => todo!(),
        PreJoinDownMsg::ChangeEntityOwningChunk { .. } => todo!(),
        PreJoinDownMsg::EditEntity { .. } => todo!(),
        /*
        PreJoinDownMsg::AddEntity { chunk_idx, entity } => {
            let (cc, ci, _) = client.chunks.lookup(chunk_idx)?;
            // TODO factor out?
            fn add_entity<S>(
                entity: DownEntity<S>,
                cc: Vec3<i64>,
                ci: usize,
                chunk_entities: &mut PerChunk<Vec<EntityEntry<S>>>,
                global_entity_hmap: &mut HashMap<Uuid, usize>,
                global_entity_slab: &mut Slab<GlobalEntityEntry>,
                kind: EntityKind,
            ) -> Result<()> {
                let DownEntity { entity_uuid: uuid, rel_pos, state } = entity;

                let entity_vec = chunk_entities.get_mut(cc, ci);
                let vector_idx = entity_vec.len();

                let global_idx = global_entity_slab.insert(GlobalEntityEntry {
                    uuid,
                    kind,
                    cc,
                    ci,
                    vector_idx,
                });
                let removed = global_entity_hmap.insert(uuid, global_idx);
                ensure!(removed.is_none(), "entity uuid collision {}", uuid);

                entity_vec.push(EntityEntry { uuid, global_idx, rel_pos, state });

                Ok(())
            }
            match entity {
                AnyDownEntity::Steve(steve) => add_entity(
                    steve,
                    cc,
                    ci,
                    &mut client.chunk_steves,
                    &mut client.global_entity_hmap,
                    &mut client.global_entity_slab,
                    EntityKind::Steve,
                ),
                AnyDownEntity::Pig(pig) => add_entity(
                    pig,
                    cc,
                    ci,
                    &mut client.chunk_pigs,
                    &mut client.global_entity_hmap,
                    &mut client.global_entity_slab,
                    EntityKind::Pig,
                ),
            }?;
        }
        PreJoinDownMsg::RemoveEntity { chunk_idx, entity_kind, entity_idx } => {
            let (cc, ci, _) = client.chunks.lookup(chunk_idx)?;
            fn remove_entity<S>(
                cc: Vec3<i64>,
                ci: usize,
                entity_idx: usize,
                chunk_entities: &mut PerChunk<Vec<EntityEntry<S>>>,
                global_entity_hmap: &mut HashMap<Uuid, usize>,
                global_entity_slab: &mut Slab<GlobalEntityEntry>,
            ) -> Result<()> {
                let entity_vec = chunk_entities.get_mut(cc, ci);
                ensure!(entity_idx < entity_vec.len(), "remove entity index out of bounds");
                let removed_entry = entity_vec.swap_remove(entity_idx);
                let removed_hmap_entry = global_entity_hmap.remove(&removed_entry.uuid);
                debug_assert!(removed_hmap_entry.is_some());
                global_entity_slab.remove(removed_entry.global_idx);
                if let Some(moved_entry) = entity_vec.get(entity_idx) {
                    global_entity_slab[moved_entry.global_idx].vector_idx = entity_idx;
                }
                Ok(())
            }
            match entity_kind {
                EntityKind::Steve => remove_entity(
                    cc,
                    ci,
                    entity_idx,
                    &mut client.chunk_steves,
                    &mut client.global_entity_hmap,
                    &mut client.global_entity_slab,
                ),
                EntityKind::Pig => remove_entity(
                    cc,
                    ci,
                    entity_idx,
                    &mut client.chunk_pigs,
                    &mut client.global_entity_hmap,
                    &mut client.global_entity_slab,
                ),
            }?;
        }
        PreJoinDownMsg::ChangeEntityOwningChunk {
            old_chunk_idx,
            entity_kind,
            entity_idx,
            new_chunk_idx,
        } => {
            let (old_cc, old_ci, _) = client.chunks.lookup(old_chunk_idx)?;
            let (new_cc, new_ci, _) = client.chunks.lookup(new_chunk_idx)?;
            fn move_entity<S>(
                old_cc: Vec3<i64>,
                old_ci: usize,
                new_cc: Vec3<i64>,
                new_ci: usize,
                old_entity_idx: usize,
                chunk_entities: &mut PerChunk<Vec<EntityEntry<S>>>,
                global_entity_slab: &mut Slab<GlobalEntityEntry>,
            ) -> Result<()> {
                // remove from old
                let old_entity_vec = chunk_entities.get_mut(old_cc, old_ci);
                ensure!(
                    old_entity_idx < old_entity_vec.len(),
                    "change entity owning chunk index out of bounds",
                );
                let mut entry = old_entity_vec.swap_remove(old_entity_idx);
                let global_idx = entry.global_idx;

                // adjust relative position
                entry.rel_pos -= ((new_cc - old_cc) * CHUNK_EXTENT).map(|n| n as f32);

                // update backlink for displaced
                if let Some(moved_entry) = old_entity_vec.get(old_entity_idx) {
                    global_entity_slab[moved_entry.global_idx].vector_idx = old_entity_idx;
                }

                // add to new
                let new_entity_vec = chunk_entities.get_mut(new_cc, new_ci);
                let new_entity_idx = new_entity_vec.len();
                new_entity_vec.push(entry);

                // update backlink for moved
                let global_entry = &mut global_entity_slab[global_idx];
                global_entry.cc = new_cc;
                global_entry.ci = new_ci;
                global_entry.vector_idx = new_entity_idx;

                Ok(())
            }
            match entity_kind {
                EntityKind::Steve => move_entity(
                    old_cc,
                    old_ci,
                    new_cc,
                    new_ci,
                    entity_idx,
                    &mut client.chunk_steves,
                    &mut client.global_entity_slab,
                ),
                EntityKind::Pig => move_entity(
                    old_cc,
                    old_ci,
                    new_cc,
                    new_ci,
                    entity_idx,
                    &mut client.chunk_pigs,
                    &mut client.global_entity_slab,
                ),
            }?
        }
        // edit entity
        PreJoinDownMsg::EditEntity { chunk_idx, entity_idx, edit } => {
            let (cc, ci, _getter) = client.chunks.lookup(chunk_idx)?;
            // TODO: refactor somehow
            fn apply_edit<S, F: FnOnce(&mut EntityEntry<S>)>(
                chunk_entities: &mut PerChunk<Vec<EntityEntry<S>>>,
                cc: Vec3<i64>,
                ci: usize,
                entity_idx: usize,
                f: F,
            ) -> Result<()> {
                let entity_vec = chunk_entities.get_mut(cc, ci);
                let entity_entry = entity_vec
                    .get_mut(entity_idx)
                    .ok_or_else(|| anyhow!("entity_idx out of bounds"))?;
                f(entity_entry);
                Ok(())
            }
            match edit {
                EntityEdit::SetStevePosVel(EntityEditSetStevePosVel { rel_pos, vel }) => {
                    apply_edit(
                        &mut client.chunk_steves, cc, ci, entity_idx,
                        |steve| {
                            steve.rel_pos = rel_pos;
                            steve.state.vel = vel;
                        }
                    )
                }
                EntityEdit::SetSteveName(EntityEditSetSteveName { name }) => {
                    apply_edit(
                        &mut client.chunk_steves, cc, ci, entity_idx,
                        |steve| {
                            steve.state.name = name;
                        }
                    )
                }
                EntityEdit::SetPigPosVel(EntityEditSetPigPosVel { rel_pos, vel }) => {
                    apply_edit(
                        &mut client.chunk_pigs, cc, ci, entity_idx,
                        |pig| {
                            pig.rel_pos = rel_pos;
                            pig.state.vel = vel;
                        }
                    )
                }
                EntityEdit::SetPigColor(EntityEditSetPigColor { color }) => {
                    apply_edit(
                        &mut client.chunk_pigs, cc, ci, entity_idx,
                        |pig| {
                            pig.state.color = color;
                        }
                    )
                }
            }?;
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
