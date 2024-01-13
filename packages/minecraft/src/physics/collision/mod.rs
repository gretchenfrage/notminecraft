//! Collision objects.

pub mod aa_box_face;
pub mod aa_box;
pub mod point;

use super::{
    world_geometry::WorldGeometry,
};
use chunk_data::Face;
use vek::*;


/// Geometric object which supports swept collision with world geometry.
pub trait CollisionObject: Sized {
    /// Sweep self in a linear path, with the given:
    ///
    /// - Position at time 0.
    /// - Velocity (change in unit position per change in unit time).
    /// - Range of time to consider.
    ///
    /// And return a `Collision` representing the collision of this collision object with the world
    /// geometry at the lowest time value within the range, if any such collisions exist in that
    /// range.
    ///
    /// `min_dt` may be negative.
    fn first_collision<W: WorldGeometry>(
        &self,
        min_dt: f32,
        max_dt: f32,
        pos: Vec3<f32>,
        vel: Vec3<f32>,
        world_geometry: &W,
    ) -> Option<Collision<W::BarrierId>>;
}

/// Collision returned from `CollisionObject`.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Collision<I> {
    /// Time at which collision occured.
    pub dt: f32,
    /// Face of the world geometry barrier that was collided with.
    pub barrier_face: Face,
    /// Identifier of the world geometry barrier that was collided with.
    pub barrier_id: I,
}
