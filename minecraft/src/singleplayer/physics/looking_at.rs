
use crate::{
    singleplayer::physics::{
        world_geometry::{
            WorldGeometry,
            WorldHitscanGeometry,
        },
        collision::{
            point::PointCollisionObject,
            CollisionObject,
        },
    },
    game_data::GameData,
};
use chunk_data::{
    Getter,
    PerChunk,
    ChunkBlocks,
    TileKey,
    Face,
};
use vek::*;


/// Compute what tile is being looked at from the given perspective.
pub fn compute_looking_at(
    start: Vec3<f32>,
    dir: Vec3<f32>,
    max_dist: f32,
    getter: &Getter,
    tile_blocks: &PerChunk<ChunkBlocks>,
    game: &GameData,
) -> Option<LookingAt>
{
    let world = WorldHitscanGeometry {
        getter,
        tile_blocks,
        game,
    };
    if let Some((_, tile)) = world.pos_inside(start) {
        Some(LookingAt {
            tile,
            pos: start,
            dist: 0.0,
            face: None,
        })
    } else {
        PointCollisionObject
            .first_collision(0.0, max_dist, start, dir, &world)
            .map(|collision| LookingAt {
                tile: collision.barrier_id,
                pos: start + dir * collision.dt,
                dist: collision.dt,
                face: Some(collision.barrier_face),
            })

    }

    /*
    assert!(dir != Vec3::from(0.0));

    let mut pos = start;
    let mut gtc = pos.map(|n| n.floor() as i64);
    let mut dist = 0.0;
    let mut face = None;

    while dist < max_dist {
        if let Some(tile) = getter.gtc_get(gtc) {
            let bid = tile.get(tile_blocks).get();
            let hitscan_logic = game
                .blocks_hitscan_logic
                .get(bid);
            match hitscan_logic {
                BlockHitscanLogic::Vacuous => (),
                BlockHitscanLogic::BasicCube | BlockHitscanLogic::Door => {
                    return Some(LookingAt {
                        tile,
                        pos,
                        dist,
                        face,
                    });
                }
            }
        }

        let basis: Vec3<Vec3<i64>> = Vec3 {
            x: Vec3::new(1, 0, 0),
            y: Vec3::new(0, 1, 0),
            z: Vec3::new(0, 0, 1),
        };

        let (gtc_delta, dist_delta) = basis.zip(pos).zip(dir).zip(gtc)
            .into_iter()
            .filter_map(|(((basis_n, pos_n), dir_n), gtc_n)|
                if dir_n > 0.0 {
                    Some((basis_n, pos_n, dir_n, gtc_n as f32 + 1.0))
                } else if dir_n < 0.0 {
                    Some((-basis_n, pos_n, dir_n, gtc_n as f32))
                } else {
                    None
                }
            )
            .map(|(gtc_delta, x1, v, x2)| (gtc_delta, (x2 - x1) / v))
            .min_by(|&(_, dist1), &(_, dist2)|
                PartialOrd::partial_cmp(
                    &dist1,
                    &dist2,
                ).unwrap())
            .unwrap();

        pos += dir * dist_delta;
        gtc += gtc_delta;
        dist += dist_delta;
        face = Some(Face::from_vec(-gtc_delta).unwrap());
    }

    None
    */
}

/// Information on which tile is being looked at from some perspective.
#[derive(Debug, Copy, Clone)]
pub struct LookingAt {
    /// Tile being looked at.
    pub tile: TileKey,
    /// Exact position where "looking" ray hits visible geometry.
    pub pos: Vec3<f32>,
    /// Distance from camera to pos.
    pub dist: f32,
    /// Which face of the tile is being looked at. May be `None` if the camera
    /// is inside a block, and thus not looking at any particular face of it.
    pub face: Option<Face>,
}
