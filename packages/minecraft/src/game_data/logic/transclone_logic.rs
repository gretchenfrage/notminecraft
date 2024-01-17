
use crate::{
    game_data::{
        GameData,
        content,
    },
    game_binschema::GameBinschema,
    item::ItemMeta,
};
use chunk_data::*;
use binschema::{*, error::*};
use std::{
    sync::Arc,
    any::TypeId,
    io::Cursor,
};


macro_rules! transcloner {
    (
        $transcloner:ident $transcloner_for:ident {$(
            $variant:ident $type:ty,
        )*}
    )=>{

        #[derive(Debug)]
        pub enum $transcloner {$(
            $variant,
        )*}
        
        pub trait $transcloner_for: Sized {
            fn transcloner_for() -> $transcloner;
        }

        impl $transcloner {
            /// Get the type id of the represented type.
            pub fn instance_type_id(&self) -> TypeId {
                match self {$(
                    &$transcloner::$variant => TypeId::of::<$type>(),
                )*}
            }

            /// Get the schema of the represented type.
            pub fn instance_schema(&self, game: &Arc<GameData>) -> Schema {
                match self {$(
                    &$transcloner::$variant => <$type as GameBinschema>::schema(game),
                )*}
            }
        }

        $(
            impl $transcloner_for for $type {
                fn transcloner_for() -> $transcloner {
                    $transcloner::$variant
                }
            }
        )*
    };
}

macro_rules! block_transcloner {
    ($( $variant:ident $type:ty, )*)=>{
        transcloner!(BlockTranscloner BlockTransclonerFor {$(
            $variant $type,
        )*});

        impl BlockTranscloner {
            /// Clone the bid + meta at one tile block to another. 
            pub fn clone_tile_block_meta(&self, a: TileBlockRead, mut b: TileBlockWrite) {
                match self {$(
                    &BlockTranscloner::$variant => {
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
                    &BlockTranscloner::$variant => ErasedBlockMeta::new(
                        <$type as Clone>::clone(a.cast::<$type>())
                    ),
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
                    &BlockTranscloner::$variant => <$type as GameBinschema>::encode(
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
                decoder: &mut Decoder<Cursor<&[u8]>>,
                game: &Arc<GameData>,
            ) -> Result<()> {
                match self {$(
                    &BlockTranscloner::$variant => tile.raw_set(
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
                    &BlockTranscloner::$variant => <$type as GameBinschema>::encode(
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
                decoder: &mut Decoder<Cursor<&[u8]>>,
                game: &Arc<GameData>,
            ) -> Result<ErasedBlockMeta> {
                Ok(match self {$(
                    &BlockTranscloner::$variant => ErasedBlockMeta::new(
                        <$type as GameBinschema>::decode(decoder, game)?
                    ),
                )*})
            }
        }
    };
}

macro_rules! item_transcloner {
    ($( $variant:ident $type:ty, )*)=>{
        transcloner!(ItemTranscloner ItemTransclonerFor {$(
            $variant $type,
        )*});

        impl ItemTranscloner {
            /// Encode an `ItemMeta` (just the metadata, not the
            /// surrounding enum).
            pub fn encode_item_meta(
                &self,
                meta: &ItemMeta,
                encoder: &mut Encoder<Vec<u8>>,
                game: &Arc<GameData>,
            ) -> Result<()> {
                match self {$(
                    &ItemTranscloner::$variant => <$type as GameBinschema>::encode(
                        meta.cast::<$type>(),
                        encoder,
                        game,
                    ),
                )*}
            }

            /// Decode an `ItemMeta` (just the metadata, not the
            /// surrounding enum).
            pub fn decode_item_meta(
                &self,
                decoder: &mut Decoder<Cursor<&[u8]>>,
                game: &Arc<GameData>,
            ) -> Result<ItemMeta> {
                Ok(match self {$(
                    &ItemTranscloner::$variant => ItemMeta::new(
                        <$type as GameBinschema>::decode(decoder, game)?
                    ),
                )*})
            }
        }
    };
}

block_transcloner!(
    Unit (),
    ChestBlockMeta content::chest::ChestBlockMeta,
);

item_transcloner!(
    Unit (),
);
