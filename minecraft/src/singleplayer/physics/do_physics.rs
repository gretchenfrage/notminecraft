
use super::{
    collision::CollisionObject,
    world_geometry::WorldGeometry,
};
use chunk_data::Face;
use vek::*;


/// Do a tick of physics to a physics object.
pub fn do_physics<C, W>(
    mut dt: f32,
    pos: &mut Vec3<f32>,
    vel: &mut Vec3<f32>,
    collision_obj: &C,
    world_geom: &W,
) -> DidPhysics
where
    C: CollisionObject,
    W: WorldGeometry,
{
    const EPSILON: f32 = 0.0001;

    let mut on_ground = false;

    while dt > EPSILON {
        if let Some(collision) = collision_obj.first_collision(
            -EPSILON,
            dt,
            *pos,
            *vel,
            world_geom,
        ) {
            if collision.barrier_face == Face::PosY {
                on_ground = true;
            }

            *pos += *vel * collision.dt;
            vel[collision.barrier_face.to_axis() as usize] = 0.0;
            if collision.dt > 0.0 {
                dt -= collision.dt;
            }
        } else {
            *pos += *vel * dt;
            dt = 0.0;
        }
    }

    DidPhysics {
        on_ground,
    }
}

/// Information returned from `do_physics`.
#[derive(Debug, Clone)]
pub struct DidPhysics {
    /// Whether player collided with the ground at all.
    pub on_ground: bool,
}
