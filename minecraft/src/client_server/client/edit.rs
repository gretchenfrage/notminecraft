
use crate::{
    block_update_queue::BlockUpdateQueue,
    game_binschema::GameBinschema,
};
use chunk_data::*;
use vek::*;


macro_rules! edit_enum {
    ($(
        $variant:ident($struct:ident),
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
    SetTileBlock(EditSetTileBlock),
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
}
