
use super::aa_box::AaBox;
use crate::game_data::{
    GameData,
    physics_logic::BlockPhysicsLogic,
    hitscan_logic::BlockHitscanLogic,
};
use chunk_data::{
    Getter,
    PerChunk,
    ChunkBlocks,
    TileKey,
};
use vek::*;


/// Set of AA boxes in the world, each associated (not necessarily uniquely)
/// with some `Self::BarrierId` identifier, and a broadphase for querying them.
pub trait WorldGeometry: Sized {
    type BarrierId: Clone;

    /// Visit AA boxes, _relative to `gtc`_, which may be in the given tile.
    fn tile_geometry<V: FnMut(AaBox, Self::BarrierId)>(
        &self,
        gtc: Vec3<i64>,
        visit: V,
    );

    /// Check whether `pos` is inside of an AA box.
    fn pos_inside(
        &self,
        pos: Vec3<f32>,
    ) -> Option<(AaBox, Self::BarrierId)> {
        let mut found = None;

        let gtc = pos.map(|n| n.floor() as i64);
        let rel_pos = pos.map(|n| n % 1.0);
        self.tile_geometry(gtc, |aa_box, barrier_id| {
            if aa_box.contains(rel_pos) {
                found = Some((aa_box, barrier_id));
            }
        });

        found
    }

    /// Check whether `obj` is intersecting with another AA box.
    fn box_intersects(&self, obj: AaBox) -> bool {
        let gtc_min = obj.pos.map(|n| n.floor() as i64);
        let gtc_max = (obj.pos + obj.ext).map(|n| n.floor() as i64);
        for z in gtc_min.z..=gtc_max.z {
            for y in gtc_min.y..=gtc_max.y {
                for x in gtc_min.x..=gtc_max.x {
                    let gtc = Vec3 { x, y, z };
                    let mut intersects = false;
                    self.tile_geometry(
                        gtc,
                        |mut aa_box, _| if aa_box
                            .translate(gtc.map(|n| n as f32))
                            .intersects(obj)
                        {
                            intersects = true
                        }
                    );
                    if intersects {
                        return true;
                    }
                }
            }
        }
        false
    }
}


#[derive(Debug, Copy, Clone)]
pub struct WorldPhysicsGeometry<'a> {
    pub getter: &'a Getter<'a>,
    pub tile_blocks: &'a PerChunk<ChunkBlocks>,
    pub game: &'a GameData,
}

impl<'a> WorldGeometry for WorldPhysicsGeometry<'a> {
    type BarrierId = ();

    fn tile_geometry<V: FnMut(AaBox, Self::BarrierId)>(
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
            &BlockPhysicsLogic::BasicCube => {
                visit(
                    AaBox::UNIT_BOX,
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
    type BarrierId = TileKey;

    fn tile_geometry<V: FnMut(AaBox, Self::BarrierId)>(
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
                &BlockHitscanLogic::BasicCube => {
                    visit(
                        AaBox::UNIT_BOX,
                        tile,
                    );
                }
            }
        }
    }
}
