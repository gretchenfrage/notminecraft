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
};
use chunk_data::*;
use uuid::Uuid;
use slab::Slab;
use vek::*;



use crate::{
    game_data::content_module_prelude::*,
    physics::prelude::*,
    sync_state_steve::*,
};
pub fn do_steve_physics(
    dt: f32,
    cc: Vec3<i64>,
    rel_pos: &mut Vec3<f32>,
    vel: &mut Vec3<f32>,
    getter: &Getter,
    tile_blocks: &PerChunk<ChunkBlocks>,
    game: &Arc<GameData>,
    mut server: Option<&mut SteveEntityServerState>,
) {
    const GRAVITY_ACCEL: f32 = 32.0;
    //const FALL_SPEED_DECAY: f32 = 0.98;
    const WALK_DECEL: f32 = 30.0;
    const GROUND_DETECTION_PERIOD: f32 = 1.0 / 20.0;



    // jumping
    if let Some(ref mut server) = server {
        if server.time_since_ground < GROUND_DETECTION_PERIOD
            && server.time_since_jumped > GROUND_DETECTION_PERIOD
        {
            vel.y += 9.2;
            //steve.vel.y += 20.0;
            server.time_since_jumped = 0.0;
        }
    }

    // server state updating
    if let Some(ref mut server) = server {
        server.time_since_jumped += dt;
        server.time_since_ground += dt;
    }

    // gravity
    vel.y -= GRAVITY_ACCEL * dt;
    //steve.vel.y *= f32::exp(20.0 * f32::ln(FALL_SPEED_DECAY) * dt);

    // friction
    let mut vel_xz = Vec2::new(vel.x, vel.z);
    let vel_xz_mag = vel_xz.magnitude();
    let max_delta_vel_xz_mag = WALK_DECEL * dt;
    if max_delta_vel_xz_mag > vel_xz_mag {
        vel_xz = Vec2::from(0.0);
    } else {
        vel_xz -= vel_xz / vel_xz_mag * max_delta_vel_xz_mag;
    }
    vel.x = vel_xz.x;
    vel.z = vel_xz.y;

    // movement
    rel_pos.x -= STEVE_WIDTH / 2.0;
    rel_pos.z -= STEVE_WIDTH / 2.0;
    let did_physics = do_physics(
        dt,
        rel_pos,
        vel,
        &AaBoxCollisionObject {
            ext: [STEVE_WIDTH, STEVE_HEIGHT, STEVE_WIDTH].into(),
        },
        &WorldPhysicsGeometry { getter, tile_blocks, game, cc_rel_to: cc },
    );
    rel_pos.x += STEVE_WIDTH / 2.0;
    rel_pos.z += STEVE_WIDTH / 2.0;

    // server state updating
    if let Some(ref mut server) = server {
        if did_physics.on_ground.is_some() {
            server.time_since_ground = 0.0;
        }
    }
}




pub trait EntityState: Clone {
    const ENTITY_TYPE: EntityType;

    fn into_any(self) -> AnyEntityState;
}

#[derive(Debug, Clone, GameBinschema)]
pub struct SteveEntityState {
    pub vel: Vec3<f32>,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct SteveEntityServerState {
    pub time_since_jumped: f32,
    pub time_since_ground: f32,    
}

impl Default for SteveEntityServerState {
    fn default() -> Self {
        SteveEntityServerState {
            time_since_jumped: f32::INFINITY,
            time_since_ground: f32::INFINITY,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SteveEntityClientState {
    pub pos_display_offset: Vec3<f32>,
}

#[derive(Debug, Clone, GameBinschema)]
pub enum SteveEntityEdit {
    SetVel(Vec3<f32>),
    SetName(String),
}

macro_rules! sync_write_entity_type {
    ($sync_write_entity_logic:ident, $sync_write_entity:ident, $entity_state:ty, $entity_server_state:ty)=>{
        pub struct $sync_write_entity<'a>($crate::sync_state_entities::SyncWriteEntityInner<'a, $entity_state, $entity_server_state>);

        impl<'a> $sync_write_entity<'a> {
            pub fn reborrow<'a2: 'a>(&'a2 mut self) -> $sync_write_entity<'a2> {
                Self(self.0.reborrow())
            }

            pub fn as_ref(&self) -> &EntityData<$entity_state> {
                self.0.as_ref()
            }

            pub fn extra(&self) -> &$entity_server_state {
                self.0.extra()
            }

            pub fn extra_mut(&mut self) -> &mut $entity_server_state {
                self.0.extra_mut()
            }
        }

        pub enum $sync_write_entity_logic {}

        impl<'a> $crate::sync_state_entities::SyncWriteEntityLogic<'a, $entity_state, $entity_server_state> for $sync_write_entity_logic {
            type SyncWriteEntity = $sync_write_entity<'a>;

            fn wrap(inner: $crate::sync_state_entities::SyncWriteEntityInner<'a, $entity_state, $entity_server_state>) -> Self::SyncWriteEntity {
                $sync_write_entity(inner)
            }
        }
    };
}

macro_rules! sync_write_entity_field_setters {
    ($sync_write_entity:ident, $edit_enum:ident, ($(
        $set_field:ident($field:ident: $t:ty) $edit_variant:ident,
    )*))=>{
        impl<'a> $sync_write_entity<'a> {$(
            pub fn $set_field(&mut self, $field: $t) {
                if self.as_ref().state.$field == $field {
                    return;
                }

                self.0.broadcast_edit(|_, _| $edit_enum::$edit_variant(<$t as Clone>::clone(&$field)));
                self.0.mark_unsaved();
                self.0.entity_state_mut().$field = $field;
            }
        )*}
    };
}

sync_write_entity_type!(SyncWriteSteveLogic, SyncWriteSteve, SteveEntityState, SteveEntityServerState);
sync_write_entity_field_setters!(SyncWriteSteve, SteveEntityEdit, (
    set_vel(vel: Vec3<f32>) SetVel,
    set_name(name: String) SetName,
));

#[derive(Debug, Copy, Clone, GameBinschema)]
pub struct PigEntityState {
    pub vel: Vec3<f32>,
    pub color: Rgb<f32>,
}

#[derive(Debug, Clone)]
pub struct PigEntityServerState {
    pub time_since_jumped: f32,
    pub time_since_ground: f32,    
}

impl Default for PigEntityServerState {
    fn default() -> Self {
        PigEntityServerState {
            time_since_jumped: f32::INFINITY,
            time_since_ground: f32::INFINITY,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PigEntityClientState {
    pub pos_display_offset: Vec3<f32>,
}

#[derive(Debug, Clone, GameBinschema)]
pub enum PigEntityEdit {
    SetVel(Vec3<f32>),
    SetColor(Rgb<f32>),
}

sync_write_entity_type!(SyncWritePigLogic, SyncWritePig, PigEntityState, PigEntityServerState);
sync_write_entity_field_setters!(SyncWritePig, PigEntityEdit, (
    set_vel(vel: Vec3<f32>) SetVel,
    set_color(color: Rgb<f32>) SetColor,
));

macro_rules! entity_types {
    ($( $name:ident($state:ty, $edit:ty), )*)=>{
        #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, GameBinschema)]
        pub enum EntityType {$(
            $name,
        )*}

        $(
            impl EntityState for $state {
                const ENTITY_TYPE: EntityType = EntityType::$name;

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

// TODO: rename to EntityTracker?

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
pub struct ChunkEntityEntry<S, E> {
    pub entity: EntityData<S>,
    pub extra: E,
    global_idx: usize,
}


impl LoadedEntities {
    /// Construct empty.
    pub fn new() -> Self {
        Default::default()
    }

    /// Call upon a chunk being added to the world to install its entities.
    pub fn add_chunk<S: EntityState, E, I>(
        &mut self,
        chunk_entities: &mut PerChunk<Vec<ChunkEntityEntry<S, E>>>,
        cc: Vec3<i64>,
        ci: usize,
        entities: I,
    ) -> Result<(), UuidCollision>
    where
        S: EntityState,
        I: IntoIterator<Item=(EntityData<S>, E)>,
    {
        let entities = entities.into_iter()
            .enumerate()
            .map(|(vector_idx, (entity, extra))| {
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
                Ok(ChunkEntityEntry { entity, global_idx, extra })
            })
            .collect::<Result<Vec<_>, _>>()?;
        chunk_entities.add(cc, ci, entities);
        Ok(())
    }

    /// Call upon a chunk being removed from the world to take back its entities.
    pub fn remove_chunk<S, E>(
        &mut self,
        chunk_entities: &mut PerChunk<Vec<ChunkEntityEntry<S, E>>>,
        cc: Vec3<i64>,
        ci: usize,
    ) -> Vec<ChunkEntityEntry<S, E>>
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
    pub fn add_entity<S: EntityState, E>(
        &mut self,
        chunk_entities: &mut PerChunk<Vec<ChunkEntityEntry<S, E>>>,
        entity: EntityData<S>,
        extra: E,
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
        entity_vec.push(ChunkEntityEntry { entity, global_idx, extra });
        Ok(())
    }

    /// Remove entity from chunk.
    ///
    /// Assumes (cc, ci) are valid. Errors if `vector_idx` invalid.
    pub fn remove_entity<S: EntityState, E>(
        &mut self,
        chunk_entities: &mut PerChunk<Vec<ChunkEntityEntry<S, E>>>,
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
    pub fn move_entity<S: EntityState, E>(
        &mut self,
        chunk_entities: &mut PerChunk<Vec<ChunkEntityEntry<S, E>>>,
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
pub struct SyncWriteBufs {
    iter_move_batch_move_ops: Vec<EntityMoveOp>,
    #[cfg(debug_assertions)]
    iter_move_batch_chunk_touched: PerChunk<bool>,
    #[cfg(debug_assertions)]
    iter_move_batch_touched_chunks: Vec<(Vec3<i64>, usize)>,
}

impl SyncWriteBufs {
    pub fn add_chunk(&mut self, cc: Vec3<i64>, ci: usize) {
        #[cfg(debug_assertions)]
        self.iter_move_batch_chunk_touched.add(cc, ci, false);
        let _ = (cc, ci);
    }

    pub fn remove_chunk(&mut self, cc: Vec3<i64>, ci: usize) {
        #[cfg(debug_assertions)]
        self.iter_move_batch_chunk_touched.remove(cc, ci);
        let _ = (cc, ci);
    }
}

#[derive(Debug, Copy, Clone)]
struct EntityMoveOp {
    op_type: EntityMoveOpType,
    ci: usize,
    vector_idx: usize,

    #[cfg(debug_assertions)]
    cc: Vec3<i64>,
    #[cfg(debug_assertions)]
    uuid: Uuid,
}

#[derive(Debug, Copy, Clone)]
enum EntityMoveOpType {
    Move,
    Delete,
}

#[derive(Debug)]
pub struct UuidCollision;

#[derive(Debug)]
pub struct VectorIdxOutOfBounds;

pub struct SyncWrite<'a, S, E, W> {
    ctx: &'a ServerSyncCtx,
    state: &'a mut PerChunk<Vec<ChunkEntityEntry<S, E>>>,
    bufs: &'a mut SyncWriteBufs,
    _p: PhantomData<W>,
}

impl<'a, S, E, W> SyncWrite<'a, S, E, W> {
    pub fn new_manual(
        ctx: &'a ServerSyncCtx,
        state: &'a mut PerChunk<Vec<ChunkEntityEntry<S, E>>>,
        bufs: &'a mut SyncWriteBufs,
    ) -> Self {
        SyncWrite { ctx, state, bufs, _p: PhantomData }
    }

    pub fn as_ref(&self) -> &PerChunk<Vec<ChunkEntityEntry<S, E>>> {
        &self.state
    }

    pub fn get(&mut self, cc: Vec3<i64>, ci: usize) -> SyncWriteChunk<S, E, W> {
        SyncWriteChunk {
            ctx: self.ctx,
            state: self.state.get_mut(cc, ci),
            cc,
            ci,
            _p: PhantomData,
        }
    }
}

impl<'a, S: EntityState, E, W> SyncWrite<'a, S, E, W> {
    pub fn create_entity(
        &mut self,
        cc: Vec3<i64>,
        ci: usize,
        state: S,
        extra: E,
        rel_pos: Vec3<f32>,
    ) {
        // generate uuid
        let uuid = Uuid::new_v4();

        // send messages to clients
        for pk in self.ctx.conn_mgr.players().iter() {
            let clientside_ci = self.ctx.chunk_mgr.chunk_to_clientside(cc, ci, pk);
            if let Some(clientside_ci) = clientside_ci {
                let state = state.clone().into_any();
                self.ctx.conn_mgr.send(pk, DownMsg::PreJoin(PreJoinDownMsg::AddEntity {
                    chunk_idx: DownChunkIdx(clientside_ci),
                    entity: EntityData { uuid, rel_pos, state },
                }));
            }
        }

        // mark chunk as unsaved
        self.ctx.save_mgr.mark_chunk_unsaved(cc, ci);

        // add it
        // unwrap safety: we are randomly generating a Uuid here, so there should not be a
        //                collision.
        self.ctx.entities.borrow_mut().add_entity(
            self.state,
            EntityData { uuid, rel_pos, state },
            extra,
            cc,
            ci,
        ).unwrap();
    }

    pub fn move_entity(
        &mut self,
        old_cc: Vec3<i64>,
        old_ci: usize,
        new_cc: Vec3<i64>,
        new_ci: usize,
        old_vector_idx: usize,
    ) {
        // move it
        // unwrap safety: that returning an error is meant for the client to deal with server protocol
        //                violations. but this is to be called in the server.
        let new_vector_idx = self.ctx.entities.borrow_mut().move_entity(
            &mut self.state,
            old_cc,
            old_ci,
            new_cc,
            new_ci,
            old_vector_idx,
        ).unwrap();

        // mark both chunks as unsaved
        self.ctx.save_mgr.mark_chunk_unsaved(old_cc, old_ci);
        self.ctx.save_mgr.mark_chunk_unsaved(new_cc, new_ci);

        // send messages to clients
        for pk in self.ctx.conn_mgr.players().iter() {
            let msg = match (
                self.ctx.chunk_mgr.chunk_to_clientside(old_cc, old_ci, pk).map(DownChunkIdx),
                self.ctx.chunk_mgr.chunk_to_clientside(new_cc, new_ci, pk).map(DownChunkIdx),
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
                        entity: self.state.get(new_cc, new_ci)[new_vector_idx].entity
                            .clone()
                            .map_state(S::into_any),
                    }
                ),
                (None, None) => None,
            };
            if let Some(msg) = msg {
                self.ctx.conn_mgr.send(pk, DownMsg::PreJoin(msg));
            }
        }
    }

    pub fn delete_entity(&mut self, cc: Vec3<i64>, ci: usize, vector_idx: usize) {
        // remove it
        // unwrap safety: that returning an error is meant for the client to deal with server protocol
        //                violations. but this is to be called in the server.
        self.ctx.entities.borrow_mut().remove_entity(&mut self.state, cc, ci, vector_idx)
            .unwrap();

        // mark chunk as unsaved
        self.ctx.save_mgr.mark_chunk_unsaved(cc, ci);

        // send messages to clients
        for pk in self.ctx.conn_mgr.players().iter() {
            if let Some(clientside_ci) = self.ctx.chunk_mgr.chunk_to_clientside(cc, ci, pk) {
                self.ctx.conn_mgr.send(pk, DownMsg::PreJoin(PreJoinDownMsg::RemoveEntity {
                    chunk_idx: DownChunkIdx(clientside_ci),
                    entity_type: S::ENTITY_TYPE,
                    vector_idx,
                }));
            }
        }
    }

    pub fn iter_move_batch(&mut self) -> IterMoveBatch<S, E, W> {
        IterMoveBatch {
            inner: SyncWrite {
                ctx: &self.ctx,
                state: &mut self.state,
                bufs: &mut self.bufs,
                _p: PhantomData,
            },
        }
    }
}

pub struct SyncWriteChunk<'a, S, E, W> {
    ctx: &'a ServerSyncCtx,
    state: &'a mut Vec<ChunkEntityEntry<S, E>>,
    cc: Vec3<i64>,
    ci: usize,
    _p: PhantomData<W>
}

impl<'a, S, E, W> SyncWriteChunk<'a, S, E, W> {
    pub fn reborrow<'a2>(&'a2 mut self) -> SyncWriteChunk<'a2, S, E, W> {
        SyncWriteChunk {
            ctx: &self.ctx,
            state: &mut self.state,
            cc: self.cc,
            ci: self.ci,
            _p: PhantomData,
        }
    }

    pub fn as_ref(&self) -> &Vec<ChunkEntityEntry<S, E>> {
        &self.state
    }
}

impl<'a, S: EntityState, E, W: SyncWriteEntityLogic<'a, S, E>> SyncWriteChunk<'a, S, E, W> {
    pub fn get(self, vector_idx: usize) -> W::SyncWriteEntity {
        W::wrap(SyncWriteEntityInner {
            ctx: self.ctx,
            state: &mut self.state[vector_idx],
            cc: self.cc,
            ci: self.ci,
            vector_idx,
        })
    }
}

pub struct SyncWriteEntityInner<'a, S, E> {
    ctx: &'a ServerSyncCtx,
    state: &'a mut ChunkEntityEntry<S, E>,
    cc: Vec3<i64>,
    ci: usize,
    vector_idx: usize,
}

impl<'a, S, E> SyncWriteEntityInner<'a, S, E> {
    pub fn reborrow<'a2>(&'a2 mut self) -> SyncWriteEntityInner<'a2, S, E> {
        SyncWriteEntityInner {
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

    pub fn extra(&self) -> &E {
        &self.state.extra
    }

    pub fn extra_mut(&mut self) -> &mut E {
        &mut self.state.extra
    }

    pub fn entity_state_mut(&mut self) -> &mut S {
        &mut self.state.entity.state
    }

    pub fn mark_unsaved(&self) {
        self.ctx.save_mgr.mark_chunk_unsaved(self.cc, self.ci);
    }

    pub fn ctx(&self) -> &ServerSyncCtx {
        &self.ctx
    }

    pub fn broadcast<F, M>(&self, mut f: F)
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

    pub fn broadcast_edit<F, M>(&self, mut f: F)
    where
        F: FnMut(PlayerKey, DownChunkIdx) -> M,
        M: Into<AnyEntityEdit>,
    {
        self.broadcast(|pk, down_chunk_idx| PreJoinDownMsg::EditEntity {
            chunk_idx: down_chunk_idx,
            vector_idx: self.vector_idx,
            edit: f(pk, down_chunk_idx).into(),
        });
    }
}

pub trait SyncWriteEntityLogic<'a, S, E> {
    type SyncWriteEntity;

    fn wrap(inner: SyncWriteEntityInner<'a, S, E>) -> Self::SyncWriteEntity;
}

pub struct IterMoveBatch<'a, S: EntityState, E, W> {
    inner: SyncWrite<'a, S, E, W>,
}

impl<'a, S: EntityState, E, W> IterMoveBatch<'a, S, E, W> {
    pub fn get(&mut self, cc: Vec3<i64>, ci: usize) -> IterMoveChunk<S, E, W> {
        #[cfg(debug_assertions)]
        {
            let touched = self.inner.bufs.iter_move_batch_chunk_touched.get_mut(cc, ci);
            assert!(!*touched, "chunk visited more than once in iter move batch");
            *touched = true;
            self.inner.bufs.iter_move_batch_touched_chunks.push((cc, ci));
        }
        IterMoveChunk {
            inner: SyncWriteChunk {
                ctx: self.inner.ctx,
                state: self.inner.state.get_mut(cc, ci),
                cc,
                ci,
                _p: PhantomData,
            },
            next_vector_idx: 0,
            iter_move_batch_move_ops: &mut self.inner.bufs.iter_move_batch_move_ops,
        }
    }

    pub fn finish_iter_move_batch(self) {
        drop(self);
    }
}

impl<'a, S: EntityState, E, W> Drop for IterMoveBatch<'a, S, E, W> {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        for (cc, ci) in self.inner.bufs.iter_move_batch_touched_chunks.drain(..) {
            *self.inner.bufs.iter_move_batch_chunk_touched.get_mut(cc, ci) = false;
        }
        while let Some(move_op) = self.inner.bufs.iter_move_batch_move_ops.pop() {
            match move_op.op_type {
                EntityMoveOpType::Move => {
                    let old_ci = move_op.ci;
                    let old_cc = self.inner.ctx.chunk_mgr.chunks().ci_to_cc(old_ci).unwrap();
                    #[cfg(debug_assertions)]
                    assert_eq!(old_cc, move_op.cc);
                    let old_vector_idx = move_op.vector_idx;
                    let entity = &self.inner.state.get(old_cc, old_ci)[old_vector_idx].entity;
                    #[cfg(debug_assertions)]
                    assert_eq!(entity.uuid, move_op.uuid);
                    let rel_cc = (entity.rel_pos / CHUNK_EXTENT.map(|n| n as f32))
                        .map(|n| n.floor() as i64);
                    let new_cc = old_cc + rel_cc;
                    let getter = self.inner.ctx.chunk_mgr.chunks().getter_pre_cached(old_cc, old_ci);
                    let new_ci = getter.get(new_cc);
                    let new_ci = match new_ci {
                        Some(new_ci) => new_ci,
                        None => {
                            // TODO: come up with an actual way of dealing with this
                            warn!("entity tried to move into not-loaded chunk!");
                            continue;
                        }
                    };
                    self.inner.move_entity(
                        old_cc,
                        old_ci,
                        new_cc,
                        new_ci,
                        old_vector_idx,
                    );
                }
                EntityMoveOpType::Delete => {
                    let ci = move_op.ci;
                    let cc = self.inner.ctx.chunk_mgr.chunks().ci_to_cc(ci).unwrap();
                    #[cfg(debug_assertions)]
                    {
                        assert_eq!(cc, move_op.cc);
                        assert_eq!(self.inner.state.get(cc, ci)[move_op.vector_idx].entity.uuid, move_op.uuid);
                    }
                    self.inner.delete_entity(cc, ci, move_op.vector_idx);
                }
            }
        }
    }
}

pub struct IterMoveChunk<'a, S, E, W> {
    inner: SyncWriteChunk<'a, S, E, W>,
    next_vector_idx: usize,
    iter_move_batch_move_ops: &'a mut Vec<EntityMoveOp>,
}

impl<'a, S, E, W> IterMoveChunk<'a, S,E,  W> {
    pub fn as_write(&mut self) -> &mut SyncWriteChunk<'a, S, E, W> {
        &mut self.inner
    }

    pub fn next(&mut self) -> Option<IterMoveEntity<S, E, W>> {
        if self.next_vector_idx < self.inner.state.len() {
            let vector_idx = self.next_vector_idx;
            self.next_vector_idx += 1;
            Some(IterMoveEntity {
                inner: SyncWriteEntityInner {
                    ctx: self.inner.ctx,
                    state: &mut self.inner.state[vector_idx],
                    cc: self.inner.cc,
                    ci: self.inner.ci,
                    vector_idx,
                },
                iter_move_batch_move_ops: &mut self.iter_move_batch_move_ops,
                _p: PhantomData,
            })
        } else {
            None
        }
    }
}

pub struct IterMoveEntity<'a, S, E, W> {
    inner: SyncWriteEntityInner<'a, S, E>,
    iter_move_batch_move_ops: &'a mut Vec<EntityMoveOp>,
    _p: PhantomData<W>,
}

impl<'a, S, E, W> IterMoveEntity<'a, S, E, W> {
    pub fn as_write<'s>(&'s mut self) -> W::SyncWriteEntity
    where
        W: SyncWriteEntityLogic<'s, S, E>
    {
        W::wrap(SyncWriteEntityInner {
            ctx: self.inner.ctx,
            state: &mut self.inner.state,
            cc: self.inner.cc,
            ci: self.inner.ci,
            vector_idx: self.inner.vector_idx,
        })
    }
}

impl<'a, S: EntityState, E, W> IterMoveEntity<'a, S, E, W> {
    pub fn set_rel_pos(self, rel_pos: Vec3<f32>) {
        if self.inner.state.entity.rel_pos == rel_pos {
            return;
        }

        self.inner.state.entity.rel_pos = rel_pos;
        self.inner.mark_unsaved();
        self.inner.broadcast_edit(|_, _| AnyEntityEdit::SetRelPos {
            entity_type: S::ENTITY_TYPE,
            rel_pos,
        });

        let rel_cc = (rel_pos / CHUNK_EXTENT.map(|n| n as f32)).map(|n| n.floor());
        if rel_cc != Vec3::from(0.0) {
            self.iter_move_batch_move_ops.push(EntityMoveOp {
                op_type: EntityMoveOpType::Move,
                ci: self.inner.ci,
                vector_idx: self.inner.vector_idx,
                #[cfg(debug_assertions)]
                cc: self.inner.cc,
                #[cfg(debug_assertions)]
                uuid: self.inner.state.entity.uuid,
            });
        }
    }

    pub fn delete(self) {
        self.inner.mark_unsaved();
        self.iter_move_batch_move_ops.push(EntityMoveOp {
            op_type: EntityMoveOpType::Delete,
            ci: self.inner.ci,
            vector_idx: self.inner.vector_idx,
            #[cfg(debug_assertions)]
            cc: self.inner.cc,
            #[cfg(debug_assertions)]
            uuid: self.inner.state.entity.uuid,
        });
    }
}




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
///    after being offloaded to the save file then loaded back from it. Both the client and server
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
