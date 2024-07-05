
use crate::game_binschema::GameBinschema;
use uuid::Uuid;
use vek::*;


#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, GameBinschema)]
pub enum EntityKind {
    Steve,
    Pig,
}

#[derive(Debug, Clone, GameBinschema)]
pub struct SteveEntityState {
    pub vel: Vec3<f32>,
    pub name: String,
}

#[derive(Debug, Copy, Clone, GameBinschema)]
pub struct PigEntityState {
    pub vel: Vec3<f32>,
    pub color: Rgb<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GlobalEntityEntry {
    /// Entity stable UUID.
    pub uuid: Uuid,
    /// Entity type.
    pub kind: EntityKind,
    /// Entity owning chunk cc.
    pub cc: Vec3<i64>,
    /// Entity owning chunk ci.
    pub ci: usize,
    /// Entity vector index.
    ///
    /// Entity's currently location within the relevant entity vector of the owning chunk.
    pub vector_idx: usize,
}

#[derive(Debug, Copy, Clone)]
pub struct EntityEntry<T> {
    /// Entity stable UUID.
    pub uuid: Uuid,
    /// Global entity index of this entity.
    pub global_idx: usize,
    /// Spatial position of this entity relative to the chunk that owns it.
    pub rel_pos: Vec3<f32>,
    /// Other entity type-specific entity state.
    pub state: T,
}

