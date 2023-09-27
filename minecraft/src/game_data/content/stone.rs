
use crate::game_data::content_module_prelude::*;


#[derive(Debug)]
pub struct ContentModule {
    pub bid_stone: BlockId<()>,
}

impl ContentModule {
    pub fn init(builder: &mut GameDataBuilder) -> Self {
        let bid_stone = builder.register_block(
            "stone",
            BlockMeshLogic::basic_cube(BTI_STONE),
        );

        ContentModule {
            bid_stone,
        }
    }
}
