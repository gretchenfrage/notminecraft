
use super::aa_box::AaBox;
use crate::game_data::{
    GameData,
    BlockPhysicsLogic,
    BlockHitscanLogic,
};
use chunk_data::{
    Getter,
    PerChunk,
    ChunkBlocks,
};
use vek::*;


/// Set of AA boxes in the world, each associated (not necessarily uniquely)
/// with some `Self::Object` identifier, and a broadphase for querying them.
pub trait WorldGeometry: Sized {
    type ObjectId;

    /// Visit AA boxes, _relative to `gtc`_, which may be in the given tile.
    fn tile_geometry<V: FnMut(AaBox, Self::ObjectId)>(
        &self,
        gtc: Vec3<i64>,
        visit: V,
    );
}


#[derive(Debug, Copy, Clone)]
pub struct WorldPhysicsGeometry<'a> {
    pub getter: &'a Getter<'a>,
    pub tile_blocks: &'a PerChunk<ChunkBlocks>,
    pub game: &'a GameData,
}

impl<'a> WorldGeometry for WorldPhysicsGeometry<'a> {
    type ObjectId = ();

    fn tile_geometry<V: FnMut(AaBox, Self::ObjectId)>(
        &self,
        gtc: Vec3<i64>,
        mut visit: V,
    ) {
        let physics_logic = self.getter
            .gtc_get(gtc)
            .map(|tile| {
                let bid = tile.get(self.tile_blocks).get();
                self.game.blocks_physics_logic.get(bid)
            })
            .unwrap_or(&BlockPhysicsLogic::BasicCube);
        match physics_logic {
            &BlockPhysicsLogic::NoClip => (),
            &BlockPhysicsLogic::BasicCube | &BlockPhysicsLogic::Door => {
                visit(
                    AaBox {
                        pos: 0.0.into(),
                        ext: 1.0.into(),
                    },
                    (),
                );
            }
        }
    }
}


#[derive(Debug, Copy, Clone)]
pub struct WorldHitscanGeometry<'a> {
    pub getter: &'a Getter<'a>,
    pub tile_blocks: &'a PerChunk<ChunkBlocks>,
    pub game: &'a GameData,
}

impl<'a> WorldGeometry for WorldHitscanGeometry<'a> {
    type ObjectId = Vec3<i64>;

    fn tile_geometry<V: FnMut(AaBox, Self::ObjectId)>(
        &self,
        gtc: Vec3<i64>,
        mut visit: V,
    ) {
        if let Some(tile) = self.getter.gtc_get(gtc) {
            let bid = tile.get(self.tile_blocks).get();
            let hitscan_logic = self.game
                .blocks_hitscan_logic
                .get(bid);
            match hitscan_logic {
                &BlockHitscanLogic::Vacuous => (),
                &BlockHitscanLogic::BasicCube | &BlockHitscanLogic::Door => {
                    visit(
                        AaBox {
                            pos: 0.0.into(),
                            ext: 1.0.into(),
                        },
                        gtc,
                    );
                }
            }
        }
    }
}
