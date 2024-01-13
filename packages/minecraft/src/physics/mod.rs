//! Physics simulation system.
//!
//! This physics system is fundamentally built around axis-aligned boxes, and is able to make
//! considerable logic simplifications by taking advantage of that. The gist is:
//!
//! - The "world geometry" is an collection of unmoveable axis-aligned boxes organized such that a
//!   broadphase exists to efficiently query them, reified by the `WorldGeometry` trait.
//! - "Physics objects" are geometric objects that can be asked to do swept collision detection
//!   against the world in a linear path, reified by the `CollisionObject` trait.
//! - The `do_physics` function can be asked to drive forward a physics object's position and
//!   velocity by a certain time step such that it is simulated with continuous collision with some
//!   world geometry.

pub mod aa_box;
pub mod world_geometry;
pub mod collision;
pub mod do_physics;
pub mod looking_at;


/// Physics system common re-exports.
pub mod prelude {
    pub use super::{
        aa_box::AaBox,
        do_physics::do_physics,
        collision::{
            aa_box::AaBoxCollisionObject,
            point::PointCollisionObject,
        },
        world_geometry::{
            WorldGeometry,
            WorldPhysicsGeometry,
        },
        looking_at::compute_looking_at,
    };
}
