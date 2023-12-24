//! Definition of the key/value schema of the save file and types to transcode keys and vals.

use crate::{
    save_file::content::*,
    game_data::GameData,
    game_binschema::GameBinschema,
};
use binschema::{*, error::Result};
use chunk_data::*;
use vek::*;
use std::sync::Arc;


// ==== schema definition ====

/// Define the save file key/value schema by expanding to tuples of key type index, key type name,
/// key type, val type.
macro_rules! save_schema {
    ()=>{
        (0, Chunk, ChunkKey, ChunkVal)
        (1, Player, PlayerKey, PlayerVal)
    };
}

/// Save file key schema for chunks.
#[derive(Debug, GameBinschema, Copy, Clone)]
pub struct ChunkKey {
    pub cc: Vec3<i64>,
}

/// Save file val schema for chunks.
#[derive(Debug, GameBinschema)]
pub struct ChunkVal {
    pub chunk_tile_blocks: ChunkBlocks,
}

/// Save file key schema for players.
#[derive(Debug, GameBinschema, Clone)]
pub struct PlayerKey {
    pub username: String,
}

/// Save file val schema for players.
pub struct PlayerVal {
    pub pos: Vec3<f32>,
    pub inventory_slots: [ItemSlot; 36],
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

impl_save_keys!(save_schema!());

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
                    &$name(_, _) => $idx,
                )*}
            }

            /// Get stringified key type name of this entry's key.
            pub fn key_type_name(&self) -> &'static str {
                match self {$(
                    &$name(_, _) => stringify!($name),
                )*}
            }

            /// Call `GameBinschema.encode` on this entry's key.
            pub fn encode_key(
                &self,
                encoder: &mut Encoder<Vec<u8>>,
                game: &Arc<GameData>,
            ) -> Result<()> {
                match self {$(
                    &$name(ref key, _) => key.encode(encoder, game),
                )*}
            }

            /// Call `GameBinschema.encode` on this entry's val.
            pub fn encode_val(
                &self,
                encoder: &mut Encoder<Vec<u8>>,
                game: &Arc<GameData>,
            ) -> Result<()> {
                match self {$(
                    &$name(_, ref val) => val.encode(encoder, game),
                )*}
            }
        }
    };
}

declare_save_entry!(save_schema!());

// define the current_save_schema function

macro_rules! define_current_save_schema {
    ($(($idx:expr, $name:ident, $key:ident, $val:ident))*)=>{
        /// Express the save file expected key/val schema definition for this version of the
        /// program as a vector of (key type name, key schema, val schema) tuples.
        pub fn current_save_schema(game: &Arc<GameData>) -> Vec<(String, Schema, Schema)> {
            vec![$(
                (stringify!($name), $key::schema(game), $val::schema(game)),
            )*]
        }
    }
}
