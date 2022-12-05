
use super::{
    super::{
        aa_box::AaBox,
        world_geometry::WorldGeometry,
    },
    CollisionObject,
    Collision,
};
use chunk_data::{
    AXES,
    Face,
    Pole,
    Sign,
};
use vek::*;


#[derive(Debug, Copy, Clone)]
pub struct PointCollisionObject;

impl CollisionObject for PointCollisionObject {
    fn first_collision<W: WorldGeometry>(
        &self,
        min_dt: f32,
        max_dt: f32,
        pos: Vec3<f32>,
        vel: Vec3<f32>,
        world_geometry: &W,
    ) -> Option<Collision<W::ObjectId>>
    {
        unimplemented!()
    }
}
