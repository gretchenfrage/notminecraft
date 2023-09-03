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
}

macro_rules! key_types {
    ($(
        $key:ident($key_ty:ty) => $val_ty:ty,
    )*)=>{
        /// Types for reading from the save file.
        pub mod read_key {
            use super::*;

            key_types!(
                @read_key_stuff ord=1;
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
                key_types!(
                    @encode_entry_stuff ord=1, self=self, encoder=encoder, game=game;
                    bind_pattern=(ref key, _), bound_variable=key;
                    {};
                    $( $key($key_ty) => $val_ty, )*
                )
            }

            /// Encode self's val value.
            pub fn encode_val(&self, encoder: &mut Encoder<Vec<u8>>, game: &Arc<GameData>) -> Result<()> {
                key_types!(
                    @encode_entry_stuff ord=1, self=self, encoder=encoder, game=game;
                    bind_pattern=(_, ref val), bound_variable=val;
                    {};
                    $( $key($key_ty) => $val_ty, )*
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
    ( @read_key_stuff ord=$ord:expr; )=>{};
    (
        @read_key_stuff ord=$ord:expr;
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
        }

        key_types!(
            @read_key_stuff ord=$ord + 1;
            $( $tail )*
        );
    };
    (
        @encode_entry_stuff ord=$ord:expr, self=$self:ident, encoder=$encoder:ident, game=$game:ident;
        bind_pattern=($($bind_pattern:tt)*), bound_variable=$bound_variable:ident;
        {$( $accum:tt )*};
    )=>{
        match $self {
            $( $accum )*
        }
    };
    (
        @encode_entry_stuff ord=$ord:expr, self=$self:ident, encoder=$encoder:ident, game=$game:ident;
        bind_pattern=($($bind_pattern:tt)*), bound_variable=$bound_variable:ident;
        {$( $accum:tt )*};
        $key:ident($key_ty:ty) => $val_ty:ty,
        $($tail:tt)*
    )=>{
        key_types!(
            @encode_entry_stuff ord=$ord + 1, self=$self, encoder=$encoder, game=$game;
            bind_pattern=($($bind_pattern)*), bound_variable=$bound_variable;
            {
                $( $accum )*
                &WriteEntry::$key( $($bind_pattern)* ) => {
                    $encoder.begin_enum($ord, stringify!($key))?;
                    GameBinschema::encode($bound_variable, $encoder, $game)
                },
            };
            $( $tail )*
        )
    };
}


key_types!(
    Chunk(Vec3<i64>) => ChunkBlocks,
);
