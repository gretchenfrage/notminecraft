
use crate::game_data::content_module_prelude::*;


#[derive(Debug)]
pub struct ContentModule {
    pub bid_stone: BlockId<vek::Rgb<u8>>,
}

impl ContentModule {
    pub fn init(builder: &mut GameDataBuilder) -> Self {
        let bid_stone = builder.register_block(
            "stone",
            BlockMeshLogic::basic_cube_rgb_u8_meta(BTI_STONE),
        );

        ContentModule {
            bid_stone,
        }
    }
}
