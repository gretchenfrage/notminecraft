
use game_binschema_derive::GameBinschema;
use crate::{
    game_data::content_module_prelude::*,
    util::array::array_from_fn,
};


#[derive(Debug)]
pub struct ContentModule {
    pub bid_chest: BlockId<ChestBlockMeta>,
}

impl ContentModule {
    pub fn init(builder: &mut GameDataBuilder) -> Self {
        let bid_chest = builder.register_block(
            "chest",
            BlockMeshLogic::basic_cube_faces({
                let mut faces = PerFace::repeat(BTI_CHEST_SIDE);
                faces[Face::PosY] = BTI_CHEST_TOP_BOTTOM;
                faces[Face::NegY] = BTI_CHEST_TOP_BOTTOM;
                faces[Face::NegZ] = BTI_CHEST_FRONT;
                faces
            }),
        );

        ContentModule {
            bid_chest
        }
    }
}

/// Metadata for chest blocks.
#[derive(Debug, Clone, GameBinschema)]
pub struct ChestBlockMeta {
    pub slots: [ItemSlot; 27],
}

impl Default for ChestBlockMeta {
    fn default() -> Self {
        ChestBlockMeta {
            slots: array_from_fn(|_| None)
        }
    }
}
