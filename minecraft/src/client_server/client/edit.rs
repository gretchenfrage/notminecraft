
use crate::{
    block_update_queue::BlockUpdateQueue,
    game_data::GameData,
    game_binschema::GameBinschema,
};
use binschema::{error::Result, *};
use chunk_data::*;
use std::sync::Arc;
use vek::*;


macro_rules! edit_enum {
    ($(
        $ordinal:expr => $variant:ident($struct:ident),
    )*)=>{
        #[derive(Debug, GameBinschema)]
        pub enum Edit {$(
            $variant($struct),
        )*}

        impl Edit {
            pub fn apply(self, ctx: EditCtx) -> Edit {
                match self {$(
                    Edit::$variant(inner) => inner.apply(ctx)
                )*}
            }
            /*
            pub fn schema() -> Schema {
                schema!(
                    enum {$(
                        $variant(%$struct::schema()),
                    )*}
                )
            }

            pub fn encode(&self, encoder: &mut Encoder<Vec<u8>>, game: &Arc<GameData>) -> Result<()> {
                match self {$(
                    &Edit::$variant(ref inner) => {
                        encoder.begin_enum($ordinal, stringify!($variant))?;
                        inner.encode(encoder, game)
                    }
                )*}
            }

            pub fn decode(decoder: &mut Decoder<&[u8]>, game: &Arc<GameData>) -> Result<Self> {
                Ok(match decoder.begin_enum()? {
                    $(
                        $ordinal => {
                            decoder.begin_enum_variant(stringify!($variant))?;
                            Edit::$variant($struct::decode(decoder, game)?)
                        }
                    )*
                    _ => unreachable!()
                })
            }
            */
        }

        $(
            impl From<$struct> for Edit {
                fn from(inner: $struct) -> Self {
                    Edit::$variant(inner)
                }
            }
        )*
    };
}

edit_enum!(
    0 => SetTileBlock(EditSetTileBlock),
);


pub struct EditCtx<'a> {
    pub cc: Vec3<i64>,
    pub ci: usize,
    pub getter: Getter<'a>,
    pub tile_blocks: &'a mut PerChunk<ChunkBlocks>,
    pub block_updates: &'a mut BlockUpdateQueue,
}


#[derive(Debug, GameBinschema)]
pub struct EditSetTileBlock {
    pub lti: u16,
    pub bid: RawBlockId,
}

impl EditSetTileBlock {
    pub fn apply(self, ctx: EditCtx) -> Edit {
        let (old_bid, old_meta) = ctx.tile_blocks
            .get(ctx.cc, ctx.ci)
            .replace(self.lti, BlockId::new(self.bid), ());
        old_meta.cast::<()>();
        let gtc = cc_ltc_to_gtc(ctx.cc, lti_to_ltc(self.lti));
        ctx.block_updates.enqueue(gtc, &ctx.getter);
        for face in FACES {
            ctx.block_updates.enqueue(gtc + face.to_vec(), &ctx.getter);
        }
        EditSetTileBlock {
            lti: self.lti,
            bid: old_bid,
        }.into()
    }
/*
    pub fn schema() -> Schema {
        schema!(
            struct {
                (lti: u16),
                (bid: u16),
            }
        )
    }

    pub fn encode(&self, encoder: &mut Encoder<Vec<u8>>, _game: &Arc<GameData>) -> Result<()> {
        encoder.begin_struct()?;
        encoder.begin_struct_field("lti")?;
        encoder.encode_u16(self.lti)?;
        encoder.begin_struct_field("bid")?;
        encoder.encode_u16(self.bid.0)?;
        encoder.finish_struct()
    }

    pub fn decode(decoder: &mut Decoder<&[u8]>, _game: &Arc<GameData>) -> Result<Self> {
        decoder.begin_struct()?;
        let value = EditSetTileBlock {
            lti: {
                decoder.begin_struct_field("lti")?;
                decoder.decode_u16()?
            },
            bid: {
                decoder.begin_struct_field("bid")?;
                RawBlockId(decoder.decode_u16()?)
            },
        };
        decoder.finish_struct()?;
        Ok(value)
    }*/
}
