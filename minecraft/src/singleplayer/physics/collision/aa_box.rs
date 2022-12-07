//! Aa box collision object.

use super::{
    super::{
        aa_box::AaBox,
        world_geometry::WorldGeometry,
    },
    aa_box_face::AaBoxFace,
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


#[derive(Debug, Copy, Clone, PartialEq)]
pub struct AaBoxCollisionObject {
    pub ext: Extent3<f32>,
}

impl CollisionObject for AaBoxCollisionObject {
    fn first_collision<W: WorldGeometry>(
        &self,
        min_dt: f32,
        max_dt: f32,
        pos: Vec3<f32>,
        vel: Vec3<f32>,
        world_geometry: &W,
    ) -> Option<Collision<W::BarrierId>>
    {
        let mut first: Option<Collision<W::BarrierId>> = None;

        let gtc_broadphase = gtc_broadphase(
            min_dt,
            max_dt,
            pos,
            vel,
            self.ext,
        );

        // for each axis, with its two complementary axes
        for axis in AXES {
            let other_axes = axis.other_axes();

            // vel along this axis and the other axes
            let axis_vel = vel[axis as usize];
            let other_axes_vel = other_axes.map(|axis2| vel[axis2 as usize]);
            
            // direction of movement along this axis
            // (skip loop iteration if not moving along this axis)
            let axis_vel_pole =
                match Pole::from_sign(Sign::of_f32(axis_vel)) {
                    Some(pole) => pole,
                    None => continue,
                };

            let obj_face = Face::from_axis_pole(axis, axis_vel_pole);
            let barrier_face = -obj_face;

            // face of object in direction of movement
            let obj_rect =
                AaBoxFace::new(
                    AaBox {
                        pos: pos,
                        ext: self.ext,
                    },
                    obj_face,
                );

            // broadphase-visit barrier boxes
            for gtc in gtc_broadphase.clone() {
                world_geometry.tile_geometry(gtc, |aa_box, barrier_id| {
                    // see if collides along this axis
                    let aa_box_face =
                        AaBoxFace::new(
                            aa_box.translate(gtc.map(|n| n as f32)),
                            barrier_face,
                        );
                    if let Some(dt) = obj_barrier_collision_dt(
                        obj_rect,
                        aa_box_face,
                        min_dt,
                        max_dt,
                        axis_vel,
                        other_axes_vel,
                    ) {
                        // compare if does
                        if first
                            .as_ref()
                            .map(|first| dt < first.dt)
                            .unwrap_or(true)
                        {
                            first = Some(Collision {
                                dt,
                                barrier_face,
                                barrier_id,
                            });
                        }
                    }
                });
            }
        }

        first
    }
}

fn obj_barrier_collision_dt(
    obj_rect: AaBoxFace,
    aa_box_face: AaBoxFace,
    min_dt: f32,
    max_dt: f32,
    axis_vel: f32,
    other_axes_vel: [f32; 2],
) -> Option<f32> {
    debug_assert!(axis_vel != 0.0);

    // time would collide
    let dt = (aa_box_face.axis_pos - obj_rect.axis_pos) / axis_vel;

    // filter by collision time
    if dt < min_dt || dt > max_dt {
        return None;
    }

    // filter by whether would actually collide rather than pass to the side of
    for i in 0..2 {
        // tangential (to rect) axis obj/barrier collision position min/max
        let other_axis_obj_col_pos_min =
            obj_rect.other_axes_pos[i] + other_axes_vel[i] * dt;
        let other_axis_obj_col_pos_max =
            other_axis_obj_col_pos_min + obj_rect.other_axes_ext[i];
        let other_axis_barrier_col_pos_min =
            aa_box_face.other_axes_pos[i];
        let other_axis_barrier_col_pos_max =
            other_axis_barrier_col_pos_min + aa_box_face.other_axes_ext[i];

        if other_axis_obj_col_pos_max < other_axis_barrier_col_pos_min {
            return None;
        }
        if other_axis_obj_col_pos_min > other_axis_barrier_col_pos_max {
            return None;
        }
    }

    // done
    Some(dt)
}

fn gtc_broadphase(
    min_dt: f32,
    max_dt: f32,
    pos: Vec3<f32>,
    vel: Vec3<f32>,
    ext: Extent3<f32>,
) -> impl Iterator<Item=Vec3<i64>> + Clone {
    // start and end positions
    let pos1 = pos + vel * min_dt;
    let pos2 = pos + vel * max_dt;

    // xyz min and max positions
    let min = pos1.zip(pos2).map(|(a, b)| f32::min(a, b));
    let max = pos1.zip(pos2).map(|(a, b)| f32::max(a, b));

    // xyz min and max gtcs may intersect with
    let min = min.map(|n| n.floor() as i64);
    let max = (max + ext).map(|n| n.ceil() as i64 - 1);

    // permute
    (min.z..=max.z)
        .flat_map(move |z| (min.y..=max.y)
            .flat_map(move |y| (min.x..=max.x)
                .map(move |x| Vec3 { x, y, z })))
}
