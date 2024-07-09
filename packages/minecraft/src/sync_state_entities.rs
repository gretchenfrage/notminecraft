//! Sync state module for all entities.

use crate::{
    game_binschema::GameBinschema,
    server::{
        per_player::PlayerKey,
        ServerSyncCtx,
    },
    message::*,
};
use std::{
    marker::PhantomData,
    fmt::Debug,
    collections::{
        HashMap,
        hash_map,
    },
    cell::RefCell,
};
use chunk_data::*;
use uuid::Uuid;
use slab::Slab;
use vek::*;


pub trait EntityState: Clone {
    const ENTITY_TYPE: EntityType;

    //type Edit: EntityEdit<Self>;

    fn into_any(self) -> AnyEntityState;
}


/*
pub trait EntityEdit<S>: Into<AnyEntityState> {
    fn apply(self, entity: &mut EntityData<S>);
}
*/
#[derive(Debug, Clone, GameBinschema)]
pub struct SteveEntityState {
    pub vel: Vec3<f32>,
    pub name: String,
}

#[derive(Debug, Clone, GameBinschema)]
pub enum SteveEntityEdit {
    SetVel(Vec3<f32>),
    SetName(String),
}
/*
impl EntityEdit<SteveEntityState> for SteveEntityEdit {
    fn apply(self, entity: &mut EntityData<SteveEntityState>) {
        match self {
            SteveEntityEdit::SetRelPos(rel_pos) => entity.rel_pos = rel_pos,
            SteveEntityEdit::SetVal(vel) => entity.state.vel = vel,
            SteveEntityEdit::SetName(name) => entity.state.name = name,
        }
    }
}
*/
#[derive(Debug, Copy, Clone, GameBinschema)]
pub struct PigEntityState {
    pub vel: Vec3<f32>,
    pub color: Rgb<f32>,
}

#[derive(Debug, Clone, GameBinschema)]
pub enum PigEntityEdit {
    SetVel(Vec3<f32>),
    SetColor(Rgb<f32>),
}
/*
impl EntityEdit<PigEntityState> for PigEntityEdit {
    fn apply(self, entity: &mut EntityData<PigEntityState>) {
        match self {
            PigEntityEdit::SetRelPos(rel_pos) => entity.rel_pos = rel_pos,
            PigEntityEdit::SetVel(vel) => entity.state.vel = vel,
            PigEntityEdit::SetColor(color) => entity.state.color = color,
        }
    }
}
*/
macro_rules! entity_types {
    ($( $name:ident($state:ty, $edit:ty), )*)=>{
        #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, GameBinschema)]
        pub enum EntityType {$(
            $name,
        )*}

        $(
            impl EntityState for $state {
                const ENTITY_TYPE: EntityType = EntityType::$name;

                //type Edit = $edit;

                fn into_any(self) -> AnyEntityState {
                    AnyEntityState::$name(self)
                }
            }
        )*

        #[derive(Debug, Clone, GameBinschema)]
        pub enum AnyEntityState {$(
            $name($state),
        )*}

        #[derive(Debug, Clone, GameBinschema)]
        pub enum AnyEntityEdit {
            SetRelPos {
                entity_type: EntityType,
                rel_pos: Vec3<f32>,
            },
            $(
                $name($edit),
            )*
        }

        $(
            impl From<$edit> for AnyEntityEdit {
                fn from(edit: $edit) -> Self {
                    AnyEntityEdit::$name(edit)
                }
            }
        )*
    };
}

entity_types!(
    Steve(SteveEntityState, SteveEntityEdit),
    Pig(PigEntityState, PigEntityEdit),
);

/// State defining an entity other than what chunk owns it and related tracking data.
#[derive(Debug, Clone, GameBinschema)]
pub struct EntityData<S> {
    pub uuid: Uuid,
    pub rel_pos: Vec3<f32>,
    pub state: S,
}

impl<S> EntityData<S> {
    pub fn map_state<S2, F: FnOnce(S) -> S2>(self, f: F) -> EntityData<S2> {
        EntityData {
            uuid: self.uuid,
            rel_pos: self.rel_pos,
            state: f(self.state),
        }
    }
}

/// Client/server global tracking state for all entities loaded into world.
#[derive(Default, Debug, Clone)]
pub struct LoadedEntities {
    /// Hash map from entity UUID to global entity index.
    hmap: HashMap<Uuid, usize>,
    /// Slab from global entity index to entity current memory location.
    slab: Slab<GlobalEntityEntry>,
}

/// Entry for entity in global entity slab tracking its current memory location.
#[derive(Debug, Clone, PartialEq)]
struct GlobalEntityEntry {
    /// Entity UUID.
    uuid: Uuid,
    /// Entity type.
    etype: EntityType,
    /// Current cc of chunk owning entity.
    cc: Vec3<i64>,
    /// Current ci of chunk owning entity.
    ci: usize,
    /// Current vector index of entity within its chunk's entity vector for its type.
    vector_idx: usize,
}

/// Struct representing an entity in a chunk.
pub struct ChunkEntityEntry<S> {
    pub entity: EntityData<S>,
    global_idx: usize,
}


impl LoadedEntities {
    /// Construct empty.
    pub fn new() -> Self {
        Default::default()
    }

    /// Call upon a chunk being added to the world to install its entities.
    pub fn add_chunk<S: EntityState, I>(
        &mut self,
        chunk_entities: &mut PerChunk<Vec<ChunkEntityEntry<S>>>,
        cc: Vec3<i64>,
        ci: usize,
        entities: I,
    ) -> Result<(), UuidCollision>
    where
        S: EntityState,
        I: IntoIterator<Item=EntityData<S>>,
    {
        let entities = entities.into_iter()
            .enumerate()
            .map(|(vector_idx, entity)| {
                let hmap_entry = match self.hmap.entry(entity.uuid) {
                    hash_map::Entry::Occupied(_) => return Err(UuidCollision),
                    hash_map::Entry::Vacant(entry) => entry,
                };
                let global_idx = self.slab.insert(GlobalEntityEntry {
                    uuid: entity.uuid,
                    etype: S::ENTITY_TYPE,
                    cc,
                    ci,
                    vector_idx,
                });
                hmap_entry.insert(global_idx);
                Ok(ChunkEntityEntry { entity, global_idx })
            })
            .collect::<Result<Vec<_>, _>>()?;
        chunk_entities.add(cc, ci, entities);
        Ok(())
    }

    /// Call upon a chunk being removed from the world to take back its entities.
    pub fn remove_chunk<S>(
        &mut self,
        chunk_entities: &mut PerChunk<Vec<ChunkEntityEntry<S>>>,
        cc: Vec3<i64>,
        ci: usize,
    ) -> Vec<ChunkEntityEntry<S>>
    {
        let entries = chunk_entities.remove(cc, ci);
        for entry in &entries {
            self.slab.remove(entry.global_idx);
            self.hmap.remove(&entry.entity.uuid);
        }
        entries
    }

    /// Add entity to chunk.
    ///
    /// Assumes (cc, ci) are valid. Errors on UUID collision.
    pub fn add_entity<S: EntityState>(
        &mut self,
        chunk_entities: &mut PerChunk<Vec<ChunkEntityEntry<S>>>,
        entity: EntityData<S>,
        cc: Vec3<i64>,
        ci: usize,
    ) -> Result<(), UuidCollision> {
        let hmap_entry = match self.hmap.entry(entity.uuid) {
            hash_map::Entry::Occupied(_) => return Err(UuidCollision),
            hash_map::Entry::Vacant(entry) => entry,
        };
        let entity_vec = chunk_entities.get_mut(cc, ci);
        let vector_idx = entity_vec.len();
        let global_idx = self.slab.insert(GlobalEntityEntry {
            uuid: entity.uuid,
            etype: S::ENTITY_TYPE,
            cc,
            ci,
            vector_idx,
        });
        hmap_entry.insert(global_idx);
        entity_vec.push(ChunkEntityEntry { entity, global_idx });
        Ok(())
    }

    /// Remove entity from chunk.
    ///
    /// Assumes (cc, ci) are valid. Errors if `vector_idx` invalid.
    pub fn remove_entity<S: EntityState>(
        &mut self,
        chunk_entities: &mut PerChunk<Vec<ChunkEntityEntry<S>>>,
        cc: Vec3<i64>,
        ci: usize,
        vector_idx: usize,
    ) -> Result<(), VectorIdxOutOfBounds> {
        let entity_vec = chunk_entities.get_mut(cc, ci);
        if vector_idx >= entity_vec.len() {
            return Err(VectorIdxOutOfBounds);
        }
        let entry = entity_vec.swap_remove(vector_idx);
        self.slab.remove(entry.global_idx);
        self.hmap.remove(&entry.entity.uuid);
        if let Some(displaced) = entity_vec.get(vector_idx) {
            self.slab[displaced.global_idx].vector_idx = vector_idx;
        }
        Ok(())
    }

    /// Change which chunk owns entity. Automatically re-relativizes its position.
    ///
    /// Assumes (old_cc, old_ci) and (new_cc, new_ci) are valid. Errors if `old_vector_idx` is
    /// invalid.
    ///
    /// Returns new vector index.
    pub fn move_entity<S: EntityState>(
        &mut self,
        chunk_entities: &mut PerChunk<Vec<ChunkEntityEntry<S>>>,
        old_cc: Vec3<i64>,
        old_ci: usize,
        new_cc: Vec3<i64>,
        new_ci: usize,
        old_vector_idx: usize,
    ) -> Result<usize, VectorIdxOutOfBounds> {
        let old_entity_vec = chunk_entities.get_mut(old_cc, old_ci);
        if old_vector_idx >= old_entity_vec.len() {
            return Err(VectorIdxOutOfBounds);
        }
        let mut entry = old_entity_vec.swap_remove(old_vector_idx);
        entry.entity.rel_pos -= ((new_cc - old_cc) * CHUNK_EXTENT).map(|n| n as f32);
        if let Some(displaced) = old_entity_vec.get(old_vector_idx) {
            self.slab[displaced.global_idx].vector_idx = old_vector_idx;
        }
        let new_entity_vec = chunk_entities.get_mut(new_cc, new_ci);
        let new_vector_idx = new_entity_vec.len();
        let global_entry = &mut self.slab[entry.global_idx];
        global_entry.cc = new_cc;
        global_entry.ci = new_ci;
        global_entry.vector_idx = new_vector_idx;
        new_entity_vec.push(entry);
        Ok(new_vector_idx)
    }
}

#[derive(Debug, Default)]
pub struct ServerEntitiesCtx {
    entities: RefCell<LoadedEntities>,
}

pub struct ServerState<S> {
    chunk_entities: PerChunk<Vec<ChunkEntityEntry<S>>>,
    iter_move_batch_to_move: Vec<EntityToMove>,
    #[cfg(debug_assertions)]
    iter_move_batch_chunk_touched: PerChunk<bool>,
    #[cfg(debug_assertions)]
    iter_move_batch_touched_chunks: Vec<(Vec3<i64>, usize)>,
}

#[derive(Debug, Copy, Clone)]
struct EntityToMove {
    ci: usize,
    vector_idx: usize,

    #[cfg(debug_assertions)]
    cc: Vec3<i64>,
    #[cfg(debug_assertions)]
    uuid: Uuid,
}

#[derive(Debug)]
pub struct UuidCollision;

#[derive(Debug)]
pub struct VectorIdxOutOfBounds;

pub fn sync_move_entity<S: EntityState>(
    ctx: &ServerSyncCtx,
    state: &mut ServerState<S>,
    old_cc: Vec3<i64>,
    old_ci: usize,
    new_cc: Vec3<i64>,
    new_ci: usize,
    old_vector_idx: usize,
) {
    // move it
    // unwrap safety: that returning an error is meant for the client to deal with server protocol
    //                violations. but this is to be called in the server.
    let new_vector_idx = ctx.entities.entities.borrow_mut().move_entity(
        &mut state.chunk_entities,
        old_cc,
        old_ci,
        new_cc,
        new_ci,
        old_vector_idx,
    ).unwrap();

    // mark both chunks as unsaved
    ctx.save_mgr.mark_chunk_unsaved(old_cc, old_ci);
    ctx.save_mgr.mark_chunk_unsaved(new_cc, new_ci);

    // send messages to clients
    for pk in ctx.conn_mgr.players().iter() {
        let msg = match (
            ctx.chunk_mgr.chunk_to_clientside(old_cc, old_ci, pk).map(DownChunkIdx),
            ctx.chunk_mgr.chunk_to_clientside(new_cc, new_ci, pk).map(DownChunkIdx),
        ) {
            (Some(old_chunk_idx), Some(new_chunk_idx)) => Some(
                PreJoinDownMsg::ChangeEntityOwningChunk {
                    old_chunk_idx,
                    entity_type: S::ENTITY_TYPE,
                    vector_idx: old_vector_idx,
                    new_chunk_idx,
                }
            ),
            (Some(chunk_idx), None) => Some(
                PreJoinDownMsg::RemoveEntity {
                    chunk_idx,
                    entity_type: S::ENTITY_TYPE,
                    vector_idx: old_vector_idx,
                }
            ),
            (None, Some(chunk_idx)) => Some(
                PreJoinDownMsg::AddEntity {
                    chunk_idx,
                    entity: state.chunk_entities.get(new_cc, new_ci)[new_vector_idx].entity
                        .clone()
                        .map_state(S::into_any),
                }
            ),
            (None, None) => None,
        };
        if let Some(msg) = msg {
            ctx.conn_mgr.send(pk, DownMsg::PreJoin(msg));
        }
    }
}

macro_rules! sync_write_types {
    ($state:ty)=>{
        pub struct SyncWrite<'a> {
            ctx: &'a ServerSyncCtx,
            state: &'a mut ServerState<$state>,
        }

        impl<'a> SyncWrite<'a> {
            pub fn new_manual(
                ctx: &'a ServerSyncCtx,
                state: &'a mut ServerState<$state>,
            ) -> Self {
                SyncWrite { ctx, state }
            }

            pub fn as_ref(&self) -> &PerChunk<Vec<ChunkEntityEntry<$state>>> {
                &self.state.chunk_entities
            }

            pub fn get(&mut self, cc: Vec3<i64>, ci: usize) -> SyncWriteChunk {
                SyncWriteChunk {
                    ctx: self.ctx,
                    state: self.state.chunk_entities.get_mut(cc, ci),
                    cc,
                    ci,
                }
            }

            pub fn begin_iter_move_batch<'s: 'a>(&'s mut self) -> IterMoveBatch<'s> {
                debug_assert!(self.state.iter_move_batch_to_move.is_empty());
                IterMoveBatch {
                    sync_write: self,
                }
            }

            pub fn move_entity(
                &mut self,
                old_cc: Vec3<i64>,
                old_ci: usize,
                new_cc: Vec3<i64>,
                new_ci: usize,
                old_vector_idx: usize,
            ) {
                sync_move_entity(
                    self.ctx,
                    self.state,
                    old_cc,
                    old_ci,
                    new_cc,
                    new_ci,
                    old_vector_idx,
                );
            }
        }

        pub struct SyncWriteChunk<'a> {
            ctx: &'a ServerSyncCtx,
            state: &'a mut Vec<ChunkEntityEntry<$state>>,
            cc: Vec3<i64>,
            ci: usize,
        }

        impl<'a> SyncWriteChunk<'a> {
            pub fn reborrow<'a2>(&'a2 mut self) -> SyncWriteChunk<'a2> {
                SyncWriteChunk {
                    ctx: &self.ctx,
                    state: &mut self.state,
                    cc: self.cc,
                    ci: self.ci,
                }
            }

            pub fn as_ref(&self) -> &Vec<ChunkEntityEntry<$state>> {
                &self.state
            }

            pub fn get(self, vector_idx: usize) -> SyncWriteEntity<'a> {
                SyncWriteEntity {
                    ctx: &self.ctx,
                    state: &mut self.state[vector_idx],
                    cc: self.cc,
                    ci: self.ci,
                    vector_idx,
                }
            }
        }

        pub struct SyncWriteEntity<'a> {
            ctx: &'a ServerSyncCtx,
            state: &'a mut ChunkEntityEntry<$state>,
            cc: Vec3<i64>,
            ci: usize,
            vector_idx: usize,
        }

        impl<'a> SyncWriteEntity<'a> {
            pub fn reborrow<'a2>(&'a2 mut self) -> SyncWriteEntity<'a2> {
                SyncWriteEntity {
                    ctx: &self.ctx,
                    state: &mut self.state,
                    cc: self.cc,
                    ci: self.ci,
                    vector_idx: self.vector_idx,
                }
            }

            pub fn as_ref(&self) -> &EntityData<$state> {
                &self.state.entity
            }

            fn mark_unsaved(&self) {
                self.ctx.save_mgr.mark_chunk_unsaved(self.cc, self.ci);
            }

            fn broadcast<F, M>(&self, mut f: F)
            where
                F: FnMut(PlayerKey, DownChunkIdx) -> M,
                M: Into<PreJoinDownMsg>,
            {
                for pk in self.ctx.conn_mgr.players().iter() {
                    if let Some(clientside_ci) =
                        self.ctx.chunk_mgr.chunk_to_clientside(self.cc, self.ci, pk)
                    {
                        let msg = f(pk, DownChunkIdx(clientside_ci));
                        self.ctx.conn_mgr.send(pk, DownMsg::PreJoin(msg.into()));
                    }
                }
            }

            fn broadcast_edit<F, E>(&self, mut f: F)
            where
                F: FnMut(PlayerKey, DownChunkIdx) -> E,
                E: Into<AnyEntityEdit>,
            {
                self.broadcast(|pk, down_chunk_idx| PreJoinDownMsg::EditEntity {
                    chunk_idx: down_chunk_idx,
                    vector_idx: self.vector_idx,
                    edit: f(pk, down_chunk_idx).into(),
                });
            }
            /*
            pub fn set_rel_pos(&mut self, rel_pos: Vec3<f32>) {
                self.state.entity.rel_pos = rel_pos;
                self.mark_unsaved();
                self.clonecast_edit(AnyEntityEdit::SetRelPos {
                    entity_type: <$state as EntityState>::ENTITY_TYPE,
                    rel_pos,
                });
            }*/
        }

        pub struct IterMoveBatch<'a> {
            sync_write: &'a mut SyncWrite<'a>,
        }

        impl<'a> IterMoveBatch<'a> {
            pub fn as_sync_write<'s: 'a>(&'s mut self) -> &'s mut SyncWrite {
                self.sync_write
            }

            pub fn iter_chunk<'s: 'a>(&'s mut self, cc: Vec3<i64>, ci: usize) -> IterMoveBatchChunk<'s> {
                #[cfg(debug_assertions)]
                {
                    let chunk_touched =
                        self.sync_write.state.iter_move_batch_chunk_touched.get_mut(cc, ci);
                    assert!(!*chunk_touched, "chunk visited twice in iter move batch");
                    *chunk_touched = true;
                    self.sync_write.state.iter_move_batch_touched_chunks.push((cc, ci));
                }
                IterMoveBatchChunk {
                    sync_write: SyncWriteChunk {
                        ctx: self.sync_write.ctx,
                        state: self.sync_write.state.chunk_entities.get_mut(cc, ci),
                        cc,
                        ci,
                    },
                    next_vector_idx: 0,
                    iter_move_batch_to_move: &mut self.sync_write.state.iter_move_batch_to_move,
                }
            }

            pub fn finish(self) {
                drop(self);
            }
        }

        pub struct IterMoveBatchChunk<'a> {
            sync_write: SyncWriteChunk<'a>,
            next_vector_idx: usize,
            iter_move_batch_to_move: &'a mut Vec<EntityToMove>,
        }

        impl<'a> IterMoveBatchChunk<'a> {
            pub fn as_sync_write<'s: 'a>(&'s mut self) -> &'s mut SyncWriteChunk {
                &mut self.sync_write
            }

            pub fn next(&mut self) -> Option<IterMoveBatchEntity> {
                if self.next_vector_idx < self.sync_write.state.len() {
                    let vector_idx = self.next_vector_idx;
                    self.next_vector_idx += 1;
                    Some(IterMoveBatchEntity {
                        sync_write: self.sync_write.reborrow().get(vector_idx),
                        iter_move_batch_to_move: self.iter_move_batch_to_move,
                    })
                } else {
                    None
                }
            }
        }

        pub struct IterMoveBatchEntity<'a> {
            sync_write: SyncWriteEntity<'a>,
            iter_move_batch_to_move: &'a mut Vec<EntityToMove>,
        }

        impl<'a> IterMoveBatchEntity<'a> {
            pub fn as_sync_write<'s: 'a>(&'s mut self) -> &'s mut SyncWriteEntity {
                &mut self.sync_write
            }

            pub fn set_rel_pos(self, rel_pos: Vec3<f32>) {
                self.sync_write.state.entity.rel_pos = rel_pos;
                self.sync_write.mark_unsaved();
                self.sync_write.broadcast_edit(|_, _| AnyEntityEdit::SetRelPos {
                    entity_type: <$state as EntityState>::ENTITY_TYPE,
                    rel_pos,
                });

                let rel_cc = (rel_pos / CHUNK_EXTENT.map(|n| n as f32)).map(|n| n.floor());
                if rel_cc != Vec3::from(0.0) {
                    self.iter_move_batch_to_move.push(EntityToMove {
                        ci: self.sync_write.ci,
                        vector_idx: self.sync_write.vector_idx,
                        #[cfg(debug_assertions)]
                        cc: self.sync_write.cc,
                        #[cfg(debug_assertions)]
                        uuid: self.sync_write.state.entity.uuid,
                    });
                }
            }
        }

        impl<'a> Drop for IterMoveBatch<'a> {
            fn drop(&mut self) {
                #[cfg(debug_assertions)]
                {
                    for (cc, ci) in self.sync_write.state.iter_move_batch_touched_chunks.drain(..) {
                        *self.sync_write.state.iter_move_batch_chunk_touched.get_mut(cc, ci) = false;
                    }
                    while let Some(to_move) = self.sync_write.state.iter_move_batch_to_move.pop() {
                        let old_ci = to_move.ci;
                        let old_cc = self.sync_write.ctx.chunk_mgr.chunks().ci_to_cc(old_ci).unwrap();
                        #[cfg(debug_assertions)]
                        assert_eq!(old_cc, to_move.cc);
                        let old_vector_idx = to_move.vector_idx;
                        let entity = &self.sync_write.state.chunk_entities.get(old_cc, old_ci)[old_vector_idx].entity;
                        #[cfg(debug_assertions)]
                        assert_eq!(entity.uuid, to_move.uuid);
                        let rel_cc = (entity.rel_pos / CHUNK_EXTENT.map(|n| n as f32)).map(|n| n.floor() as i64);
                        let new_cc = old_cc + rel_cc;
                        let getter = self.sync_write.ctx.chunk_mgr.chunks().getter_pre_cached(old_cc, old_ci);
                        let new_ci = getter.get(new_cc);
                        let new_ci = match new_ci {
                            Some(new_ci) => new_ci,
                            None => {
                                // TODO: come up with an actual way of dealing with this
                                warn!("entity tried to move into not-loaded chunk!");
                                continue;
                            }
                        };
                        self.sync_write.move_entity(
                            old_cc,
                            old_ci,
                            new_cc,
                            new_ci,
                            old_vector_idx,
                        );
                    }
                }
            }
        }
    };
}

macro_rules! sync_write_field_setter {
    ($sync_write_entity:ident $set_field:ident($field:ident: $t:ty) $($edit:tt)*)=>{
        impl<'a> $sync_write_entity<'a> {
            pub fn $set_field(&mut self, $field: $t) {
                self.broadcast_edit(|_, _| $($edit)*(<$t as Clone>::clone(&$field)));
                self.mark_unsaved();
                self.state.entity.state.$field = $field;
            }
        }
    };
}

macro_rules! sync_write_field_setters {
    ($sync_write_entity:ident $edit_enum:ident ($(
        $set_field:ident($field:ident: $t:ty) $edit_variant:ident,
    )*))=>{$(
        sync_write_field_setter!($sync_write_entity $set_field($field: $t) $edit_enum::$edit_variant);
    )*};
}

pub mod steve {
    use super::*;

    sync_write_types!(SteveEntityState);
    sync_write_field_setters!(SyncWriteEntity SteveEntityEdit (
        set_vel(vel: Vec3<f32>) SetVel,
        set_name(name: String) SetName,
    ));
}

pub mod pig {
    use super::*;

    sync_write_types!(PigEntityState);
    sync_write_field_setters!(SyncWriteEntity PigEntityEdit (
        set_vel(vel: Vec3<f32>) SetVel,
        set_color(color: Rgb<f32>) SetColor,
    ));
}

/*
pub struct SyncWrite<'a, S> {
    ctx: &'a ServerSyncCtx,
    state: &'a mut PerChunk<Vec<ChunkEntityEntry<S>>>,
}

impl<'a, S> SyncWrite<'a, S> {
    pub fn new_manual(
        ctx: &'a ServerSyncCtx,
        state: &'a mut PerChunk<Vec<ChunkEntityEntry<S>>>,
    ) -> Self {
        SyncWrite { ctx, state }
    }

    pub fn as_ref(&self) -> &PerChunk<Vec<ChunkEntityEntry<S>>> {
        &self.state
    }

    pub fn get(&mut self, cc: Vec3<i64>, ci: usize) -> SyncWriteChunk<'a, S> {
        SyncWriteChunk {
            ctx: self.ctx,
            state: self.state.get(cc, ci),
            cc,
            ci,
        }
    }
}

pub struct SyncWriteChunk<'a, S> {
    ctx: &'a ServerSyncCtx,
    state: &'a mut Vec<ChunkEntityEntry<S>>,
    cc: Vec3<i64>,
    ci: usize,
}

impl<'a, S> SyncWriteChunk<'a, S> {
    pub fn reborrow<'a2>(&'a2 mut self) -> SyncWriteChunk<'a2> {
        SyncWriteChunk {
            ctx: &self.ctx,
            state: &mut self.state,
            cc: self.cc,
            ci: self.ci,
        }
    }

    pub fn as_ref(&self) -> &Vec<ChunkEntityEntry<S>> {
        &self.state
    }

    pub fn get(self, vector_idx: usize) -> SyncWriteEntity<'a, S>
        SyncWriteEntity {
            ctx: &self.ctx,
            state: &mut self.state[vector_idx],
            cc: self.cc,
            ci: self.ci,
            vector_idx,
        }
    }
}

pub struct SyncWriteEntity<'a, S> {
    ctx: &'a ServerSyncCtx,
    state: &'a mut ChunkEntityEntry<S>,
    cc: Vec3<i64>,
    ci: usize,
    vector_idx: usize,
}

impl<'a, S> SyncWriteEntity<'a, S> {
    pub fn reborrow<'a2>(&'a2 mut self) -> SyncWriteEntity<'a2> {
        SyncWriteEntity {
            ctx: &self.ctx,
            state: &mut self.state,
            cc: self.cc,
            ci: self.ci,
            vector_idx: self.vector_idx,
        }
    }

    pub fn as_ref(&self) -> &EntityData<S> {
        &self.state.entity
    }

    pub fn edit_manual<F1, F2>(&mut self, modify: F1)
    where
        F1: FnOnce(&mut EntityData) -> F2,
        F2: FnMut()
}
*/


/*
/// Both server-side and client-side state for tracking all loaded entities.
///
/// Each entity is considered to be owned by a particular chunk. The entity is loaded/unloaded from
/// the server/client when that chunk is loaded/unloaded from the server/client. An entity can move
/// between chunks and thus which chunk owns the entity change while still maintaining continuity
/// of the entity's identity.
///
/// There are multiple different data types of entities. For each entity type, each chunk maintains
/// a vector of all entities of that type which are owned by that chunk. Whereas most things in the
/// world of which there can be multiple are stored in slabs, entities are just stored in vectors
/// and removed via swap-removal. This makes their indices much more unstable, but simplifies
/// client synchronization and memory consumption.
///
/// In addition to the per-chunk vectors of entities, the client and the server both maintain a
/// unitary slab of all entities currently loaded into their version of the world, thus creating a
/// space of "global entity indexes" which are stable for as long as that entity is still loaded
/// in the client/server's world. This global slab is not specific to entity type.
///
/// As such, each entity has three forms of location / identification:
///
/// 1. The entity UUID, a permanently stable identifier for that entity that stays the same even
///    after being pushed to the save file then loaded back from it. Both the client and server
///    maintain a hash map from entity UUID to global index.
/// 2. The entity's global index, as mentioned above, which can be used to efficiently find the
///    entity's current storage location for as long as the entity remains continuously loaded in
///    the world.
/// 3. The entity's current storage location, consisting of which chunk it's in, the entity's type
///    (determining which of the chunk's entity vectors it's in), and the entity's index within
///    that vector. This is very unstable as it can be changed not only by changes to what chunk
///    own the entity, but also by the removal or movement of _other_ entities, since entities
///    removed from their current entity vectors are removed via swap-removal.
///
/// All entities have a position, a vector of three floats representing their position _relative_
/// to the chunk that owns them. It is ontologically possible for the chunk the entity is in
/// spatially to diverge from the chunk that owns the entity, but this system tries to keep the
/// two synchronized.
///
/// Finally, each entity may have some additional state, defined by its entity type. As such, an
/// entity loaded into the client/server's world has the following properties:
///
/// - Entity UUID (never changes by definition).
/// - Global index (may change if unloaded and loaded again).
/// - Entity type (not supposed to change).
/// - Entity state (may be mutated intentionally).
/// - Relative position (may be changed intentionally).
/// - Owning chunk (may be changed intentionally, generally by changing position).
/// - Vector index (may be changed intentionally or due to other entities shifting around).
pub struct Entities {
    pub global_entity_hmap: HashMap<Uuid, usize>,
}
*/
