
use super::{
    world_geometry::{
        WorldGeometry,
        WorldHitscanGeometry,
    },
    collision::{
        point::PointCollisionObject,
        CollisionObject,
    },
};
use crate::{
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
