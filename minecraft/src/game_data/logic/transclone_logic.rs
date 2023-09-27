
use crate::{
    game_data::GameData,
    game_binschema::GameBinschema,
};
use chunk_data::*;
use binschema::{*, error::*};
use std::{
    sync::Arc,
    any::TypeId,
};

pub trait TransclonerFor: Sized {
    fn transcloner_for() -> Transcloner;
}

macro_rules! transcloner {
    ($( $variant:ident $type:ty, )*)=>{
        /// Transcoder / cloner of type erased type.
        #[derive(Debug)]
        pub enum Transcloner {$(
            $variant,
        )*}

        impl Transcloner {
            /// Get the type id of the represented type.
            pub fn instance_type_id(&self) -> TypeId {
                match self {$(
                    &Transcloner::$variant => TypeId::of::<$type>(),
                )*}
            }

            /// Clone the bid + meta at one tile block to another. 
            pub fn clone_tile_block_meta(&self, a: TileBlockRead, mut b: TileBlockWrite) {
                match self {$(
                    &Transcloner::$variant => {
                        b.raw_set(
                            a.get(),
                            <$type as Clone>::clone(a.raw_meta::<$type>()),
                        );
                    }
                )*}
            }

            /// Clone an `ErasedBlockMeta`.
            pub fn clone_erased_block_meta(&self, a: &ErasedBlockMeta) -> ErasedBlockMeta {
                match self {$(
                    &Transcloner::$variant => ErasedBlockMeta::new(
                        <$type as Clone>::clone(a.cast::<$type>())
                    ),
                )*}
            }

            /// Get the schema of the represented type.
            pub fn instance_schema(&self, game: &Arc<GameData>) -> Schema {
                match self {$(
                    &Transcloner::$variant => <$type as GameBinschema>::schema(game),
                )*}
            }

            /// Encode the metadata of a tile block (just the metadata, not the
            /// surrounding enum).
            pub fn encode_tile_block_meta(
                &self,
                tile: TileBlockRead,
                encoder: &mut Encoder<Vec<u8>>,
                game: &Arc<GameData>,
            ) -> Result<()> {
                match self {$(
                    &Transcloner::$variant => <$type as GameBinschema>::encode(
                        tile.raw_meta::<$type>(),
                        encoder,
                        game,
                    ),
                )*}
            }

            /// Decode the metadata of a tile block (just the metadata, not the
            /// surrounding enum) and write it + the given bid to a tile block.
            pub fn decode_tile_block_meta(
                &self,
                bid: RawBlockId,
                mut tile: TileBlockWrite,
                decoder: &mut Decoder<&[u8]>,
                game: &Arc<GameData>,
            ) -> Result<()> {
                match self {$(
                    &Transcloner::$variant => tile.raw_set(
                        bid,
                        <$type as GameBinschema>::decode(decoder, game)?,
                    ),
                )*}
                Ok(())
            }

            /// Encode an `ErasedBlockMeta` (just the metadata, not the
            /// surrounding enum).
            pub fn encode_erased_block_meta(
                &self,
                meta: &ErasedBlockMeta,
                encoder: &mut Encoder<Vec<u8>>,
                game: &Arc<GameData>,
            ) -> Result<()> {
                match self {$(
                    &Transcloner::$variant => <$type as GameBinschema>::encode(
                        meta.cast::<$type>(),
                        encoder,
                        game,
                    ),
                )*}
            }

            /// Decode an `ErasedBlockMeta` (just the metadata, not the
            /// surrounding enum).
            pub fn decode_erased_block_meta(
                &self,
                decoder: &mut Decoder<&[u8]>,
                game: &Arc<GameData>,
            ) -> Result<ErasedBlockMeta> {
                Ok(match self {$(
                    &Transcloner::$variant => ErasedBlockMeta::new(
                        <$type as GameBinschema>::decode(decoder, game)?
                    ),
                )*})
            }
        }

        $(
            impl TransclonerFor for $type {
                fn transcloner_for() -> Transcloner {
                    Transcloner::$variant
                }
            }
        )*
    };
}

transcloner!(
    Unit (),
    RgbU8 vek::Rgb<u8>,
);
