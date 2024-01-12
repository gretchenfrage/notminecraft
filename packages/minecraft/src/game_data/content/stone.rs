
use crate::game_data::content_module_prelude::*;


#[derive(Debug)]
pub struct ContentModule {
    pub bid_stone: BlockId<()>,
    pub iid_stone: ItemId<()>,
}

impl ContentModule {
    pub fn init(builder: &mut GameDataBuilder) -> Self {
        let bid_stone = builder.register_block(
            "stone",
            #[cfg(feature = "client")]
            BlockMeshLogic::basic_cube(BTI_STONE),
        );

        let iid_stone = builder.register_item(
            "stone",
            #[cfg(feature = "client")]
            LangKey::tile_stone_name,
            #[cfg(feature = "client")]
            ItemMeshLogic::basic_cube(BTI_STONE),
        );
        
        ContentModule {
            bid_stone,
            iid_stone,
        }
    }
}
