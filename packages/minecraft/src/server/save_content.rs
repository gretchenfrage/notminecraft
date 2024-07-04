//! Definition of the key/value schema of the save file and types to transcode keys and vals.

use crate::{
    game_data::GameData,
    game_binschema::GameBinschema,
    item::*,
    entity::*,
};
use binschema::{*, error::Result};
use chunk_data::*;
use std::sync::Arc;
use uuid::Uuid;
use vek::*;


// ==== schema definition ====

/// Define the save file key/value schema by applying to the provided macro name tuples of key type
/// index, key type name, key type, val type.
macro_rules! save_schema {
    ($macro:ident)=>{
        $macro! {
            (0, Chunk, ChunkSaveKey, ChunkSaveVal)
            (1, Player, PlayerSaveKey, PlayerSaveVal)
            (2, EntityLocation, EntityLocationSaveKey, EntityLocationSaveVal)
        }
    };
}

/// Save file key schema for chunks.
#[derive(Debug, GameBinschema, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ChunkSaveKey {
    pub cc: Vec3<i64>,
}

/// Save file val schema for chunks.
#[derive(Debug, GameBinschema)]
pub struct ChunkSaveVal {
    /// The chunk's blocks and block metadata.
    pub chunk_tile_blocks: ChunkBlocks,

    // TODO: invert these for the save file so it's just one nice big (dynamic?) enum?

    /// Steve entities in the chunk.
    pub steves: Vec<EntitySaveEntry<SteveEntitySaveState>>,
    /// Pig entities in the chunk.
    pub pigs: Vec<EntitySaveEntry<PigEntitySaveState>>,
}

/// Save file schema for entry in one of a chunk's entity vectors.
#[derive(Debug, GameBinschema)]
pub struct EntitySaveEntry<T> {
    /// Stable UUID of entity.
    pub entity_uuid: Uuid,
    /// Entity's spatial position relative to chunk.
    pub rel_pos: Vec3<f32>,
    /// Other entity-specific entity state
    pub state: T,
}

#[derive(Debug, GameBinschema)]
pub struct SteveEntitySaveState {
    pub name: String,
}

#[derive(Debug, GameBinschema)]
pub struct PigEntitySaveState {
    pub color: Rgb<f32>,
}

/// Save file key schema for players.
#[derive(Debug, GameBinschema, Clone, Eq, PartialEq, Hash)]
pub struct PlayerSaveKey {
    pub username: String,
}

/// Save file val schema for players.
#[derive(Debug, GameBinschema)]
pub struct PlayerSaveVal {
    pub pos: Vec3<f32>,
    pub yaw: f32,
    pub pitch: f32,
    pub inventory_slots: [Option<ItemStack>; 36],
    pub held_slot: Option<ItemStack>,
}

/// Save file key schema for entity location indexing.
#[derive(Debug, GameBinschema, Clone, Eq, PartialEq, Hash)]
pub struct EntityLocationSaveKey {
    /// Stable UUID of entity.
    pub entity_uuid: Uuid,
}

/// Save file val schema for entity location indexing.
#[derive(Debug, GameBinschema)]
pub struct EntityLocationSaveVal {
    /// What type of entity it is.
    pub entity_kind: EntityKind,
    /// Chunk coord of chunk that owns the entity.
    pub owning_cc: Vec3<i64>,
    /// Entity index within its entity vector.
    pub entity_idx: usize,
    /// Entity's spatial position relative to the chunk.
    pub rel_pos: Vec3<f32>,
}

// ==== transcoding stuff ====

/// A type of key for reading from the save file.
pub trait SaveKey: GameBinschema {
    /// The associated value type.
    type Val: GameBinschema;

    /// Key type index of this key, starting at 0.
    fn key_type_idx() -> usize;

    /// Stringified key type name of this key.
    fn key_type_name() -> &'static str;
}

// implement SaveKey for all key types

macro_rules! impl_save_keys {
    ($(($idx:expr, $name:ident, $key:ident, $val:ident))*)=>{$(
        impl SaveKey for $key {
            type Val = $val;

            fn key_type_idx() -> usize { $idx }

            fn key_type_name() -> &'static str { stringify!($name) }
        }
    )*};
}

save_schema!(impl_save_keys);

// generate the SaveEntry enum

macro_rules! declare_save_entry {
    ($(($idx:expr, $name:ident, $key:ident, $val:ident))*)=>{
        /// A key/value entry for writing to a save file.
        #[derive(Debug)]
        pub enum SaveEntry {$(
            $name($key, $val),
        )*}

        impl SaveEntry {
            /// Get key type index of this entry's key, starting at 0.
            pub fn key_type_idx(&self) -> usize {
                match self {$(
                    &SaveEntry::$name(_, _) => $idx,
                )*}
            }

            /// Get stringified key type name of this entry's key.
            pub fn key_type_name(&self) -> &'static str {
                match self {$(
                    &SaveEntry::$name(_, _) => stringify!($name),
                )*}
            }

            /// Call `GameBinschema.encode` on this entry's key.
            pub fn encode_key(
                &self,
                encoder: &mut Encoder<Vec<u8>>,
                game: &Arc<GameData>,
            ) -> Result<()> {
                match self {$(
                    &SaveEntry::$name(ref key, _) => key.encode(encoder, game),
                )*}
            }

            /// Call `GameBinschema.encode` on this entry's val.
            pub fn encode_val(
                &self,
                encoder: &mut Encoder<Vec<u8>>,
                game: &Arc<GameData>,
            ) -> Result<()> {
                match self {$(
                    &SaveEntry::$name(_, ref val) => val.encode(encoder, game),
                )*}
            }
        }
    };
}

save_schema!(declare_save_entry);

// define the current_save_schema function

macro_rules! define_current_save_schema {
    ($(($idx:expr, $name:ident, $key:ident, $val:ident))*)=>{
        /// Express the save file expected key/val schema definition for this version of the
        /// program as a vector of (key type name, key schema, val schema) tuples.
        pub fn current_save_schema(game: &Arc<GameData>) -> Vec<(String, Schema, Schema)> {
            vec![$(
                (stringify!($name).to_owned(), $key::schema(game), $val::schema(game)),
            )*]
        }
    }
}

save_schema!(define_current_save_schema);
