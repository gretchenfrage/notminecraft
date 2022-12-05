
pub mod aa_box;
pub mod point;

use super::{
    world_geometry::WorldGeometry,
};
use chunk_data::Face;
use vek::*;


/// Geometric object which supports swept collision with world geometry.
pub trait CollisionObject: Sized {
    fn first_collision<W: WorldGeometry>(
        &self,
        min_dt: f32,
        max_dt: f32,
        pos: Vec3<f32>,
        vel: Vec3<f32>,
        world_geometry: &W,
    ) -> Option<Collision<W::ObjectId>>;
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Collision<I> {
    pub dt: f32,
    pub world_obj_face: Face,
    pub world_obj_id: I,
}
