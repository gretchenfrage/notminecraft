//! Handling of the key/value types, schemas, and transcoding of such for the
//! save file.

use crate::{
    game_data::GameData,
    game_binschema::GameBinschema,
};
use binschema::{*, error::Result};
use chunk_data::*;
use vek::*;
use std::sync::Arc;


/// Type of key that can be read from the save file. Implementations macro-generated.
pub trait ReadKey {
    type Val;

    /// Encode self's key value, including beginning enum, wherein variant 0 is reserved
    /// for save file system itself.
    fn encode_key(&self, encoder: &mut Encoder<Vec<u8>>, game: &Arc<GameData>) -> Result<()>;

    /// Decode val value, _not_ including beginning enum, wherein variant 0 is reserved
    /// for save file system itself.
    fn decode_val(decoder: &mut Decoder<&[u8]>, game: &Arc<GameData>) -> Result<Self::Val>
    where
        Self: Sized;

    /// Key type index of this key, starting at 0.
    fn key_type_index() -> usize;
}

macro_rules! read_key_types {
    ( $ord:expr; )=>{};
    (
        $ord:expr;
        $key:ident($key_ty:ty) => $val_ty:ty,
        $($tail:tt)*
    )=>{
        /// Macro-generated type for reading this key type from the save file.
        #[derive(Debug)]
        pub struct $key($key_ty);

        impl ReadKey for $key {
            type Val = $val_ty;

            fn encode_key(&self, encoder: &mut Encoder<Vec<u8>>, game: &Arc<GameData>) -> Result<()> {
                encoder.begin_enum($ord, stringify!($key))?;
                <$key_ty as GameBinschema>::encode(&self.0, encoder, game)
            }

            fn decode_val(decoder: &mut Decoder<&[u8]>, game: &Arc<GameData>) -> Result<Self::Val> {
                <$val_ty as GameBinschema>::decode(decoder, game)
            }

            fn key_type_index() -> usize {
                $ord - 1
            }
        }

        read_key_types!(
            $ord + 1;
            $( $tail )*
        );
    };
}

macro_rules! encode_key {
    (
        $ord:expr, $self:ident, $encoder:ident, $game:ident, { $($accum:tt)* };
    )=>{
        match $self { $($accum)* }
    };
    (
        $ord:expr, $self:ident, $encoder:ident, $game:ident, { $($accum:tt)* };
        $key:ident($key_ty:ty) => $val_ty:ty,
        $($tail:tt)*
    )=>{
        encode_key!(
            $ord + 1, $self, $encoder, $game, {
                $($accum)*
                &WriteEntry::$key(ref key, _) => {
                    $encoder.begin_enum($ord, stringify!($key))?;
                    <$key_ty as GameBinschema>::encode(key, $encoder, $game)
                }
            };
            $( $tail )*
        )
    };
}

macro_rules! key_type_index {
    (
        $idx:expr, $self:ident, { $($accum:tt)* };
    )=>{
        match $self { $($accum)* }
    };
    (
        $idx:expr, $self:ident, { $($accum:tt)* };
        $key:ident,
        $($tail:tt)*
    )=>{
        key_type_index!(
            $idx + 1, $self, {
                $($accum)*
                &WriteEntry::$key(_, _) => $idx,
            };
            $( $tail )*
        )
    };
}

macro_rules! key_types {
    ($(
        $key:ident($key_ty:ty) => $val_ty:ty,
    )*)=>{
        /// Types for reading from the save file.
        pub mod read_key {
            use super::*;

            read_key_types!(
                1;
                $( $key($key_ty) => $val_ty, )*
            );
        }

        /// A key/value entry to be written to the save file.
        #[derive(Debug)]
        pub enum WriteEntry {$(
            $key($key_ty, $val_ty),
        )*}

        impl WriteEntry {
            /// Encode self's key value, including beginning enum, wherein variant 0 is
            /// reserved for save file system itself.
            pub fn encode_key(&self, encoder: &mut Encoder<Vec<u8>>, game: &Arc<GameData>) -> Result<()> {
                encode_key!(
                    1, self, encoder, game, {};
                    $( $key($key_ty) => $val_ty, )*
                )
            }

            /// Encode self's val value. Does not automatically involve enums.
            pub fn encode_val(&self, encoder: &mut Encoder<Vec<u8>>, game: &Arc<GameData>) -> Result<()> {
                match self {$(
                    &WriteEntry::$key(_, ref val) => {
                        <$val_ty as GameBinschema>::encode(val, encoder, game)
                    }
                )*}
            }

            /// Key type index of this entry, starting at 0.
            pub fn key_type_index(&self) -> usize {
                key_type_index!(
                    0, self, {};
                    $( $key, )*
                )
            }
        }

        /// Get save file schema as list of (key name, key schema, val schema) tuples.
        pub fn key_types(game: &Arc<GameData>) -> Vec<(String, Schema, Schema)> {
            vec![$(
                (
                    stringify!($key).into(),
                    <$key_ty as GameBinschema>::schema(game),
                    <$val_ty as GameBinschema>::schema(game),
                )
            )*]
        }
    };
}


key_types!(
    Chunk(Vec3<i64>) => ChunkBlocks,
);
