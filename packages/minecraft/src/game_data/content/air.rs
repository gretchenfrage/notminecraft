
use crate::game_data::content_module_prelude::*;


#[derive(Debug)]
pub struct ContentModule;

impl ContentModule {
    pub fn init(builder: &mut GameDataBuilder) -> Self {
        builder.blocks_machine_name.set(AIR, "air".into());
        #[cfg(feature = "client")]
        builder.blocks_mesh_logic.set(AIR, BlockMeshLogic::NoMesh);
        builder.blocks_hitscan_logic.set(AIR, BlockHitscanLogic::Vacuous);
        builder.blocks_physics_logic.set(AIR, BlockPhysicsLogic::NoClip);
        builder.blocks_can_place_over.set(AIR, true);
        ContentModule
    }
}
