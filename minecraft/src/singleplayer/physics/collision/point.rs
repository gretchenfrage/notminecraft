//! Point collision object.

use super::{
    super::world_geometry::WorldGeometry,
    aa_box_face::AaBoxFace,
    CollisionObject,
    Collision,
};
use chunk_data::{
    AXES,
    Face,
    Pole,
    Sign,
    PerAxis,
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
    ) -> Option<Collision<W::BarrierId>>
    {
        let vel_poles: PerAxis<Option<Pole>> = AXES
            .map(|axis| Pole::from_sign(Sign::of_f32(vel[axis as usize])));
        // nonzero_vel_axes is Iterator<(Axis, Pole)> + Clone
        let nonzero_vel_axes = AXES
            .into_iter()
            .filter_map(|axis| vel_poles[axis]
                .map(|pole| (axis, pole)));

        let mut pos = pos + min_dt * vel;
        let mut gtc = pos.map(|n| n.floor() as i64);
        let mut dt = min_dt;
        let mut entered_face = None;

        while dt < max_dt {
            // determine the first collision in the current tile
            let mut first: Option<Collision<W::BarrierId>> = None;

            world_geometry.tile_geometry(gtc, |aa_box, barrier_id| {
                for (axis, axis_vel_pole) in nonzero_vel_axes.clone() {
                    let other_axes = axis.other_axes();

                    // prep other-axes component arrays
                    let mut obj_other_axes_pos = [0.0; 2];
                    let mut other_axes_vel = [0.0; 2];
                    for i in 0..2 {
                        obj_other_axes_pos[i] = pos[other_axes[i] as usize];
                        other_axes_vel[i] = vel[other_axes[i] as usize];
                    }

                    // face of barrier
                    let barrier_face =
                        Face::from_axis_pole(axis, -axis_vel_pole);
                    let barrier_face_rect =
                        AaBoxFace::new(
                            aa_box.translate(gtc.map(|n| n as f32)),
                            barrier_face,
                        );

                    // see if collides along this axis
                    if let Some(dt) = obj_barrier_collision_dt(
                        pos[axis as usize],
                        obj_other_axes_pos,
                        barrier_face_rect,
                        min_dt,
                        max_dt,
                        vel[axis as usize],
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
                                barrier_id: barrier_id.clone(),
                            });
                        }
                    }
                }
            });

            // return if any such collision exists
            if let Some(first) = first {
                return Some(first);
            }

            // if not, enter the next tile
            let enter = nonzero_vel_axes
                .clone()
                .map(|(axis, pole)| {
                    let axis_pos1 = pos[axis as usize];
                    let axis_pos2 =
                        (
                            gtc[axis as usize]
                            + match pole {
                                Pole::Neg => 0,
                                Pole::Pos => 1,
                            }
                        )
                        as f32;
                    let axis_vel = vel[axis as usize];
                    let enter_dt = (axis_pos2 - axis_pos1) / axis_vel;
                    (
                        (axis, pole),
                        enter_dt,
                    )
                })
                .min_by(|(_, dt1), (_, dt2)| dt1.partial_cmp(dt2).unwrap());
            
            if let Some(((axis, pole), enter_dt)) = enter {
                gtc[axis as usize] += pole.to_int();
                pos += vel * enter_dt;
                dt += enter_dt;
                entered_face = Some(Face::from_axis_pole(axis, pole));
            } else {
                // this case occurs if the velocity is <0,0,0>
                return None;
            }
        }

        None
    }
}

fn obj_barrier_collision_dt(
    obj_axis_pos: f32,
    obj_other_axes_pos: [f32; 2],
    barrier_face: AaBoxFace,
    min_dt: f32,
    max_dt: f32,
    axis_vel: f32,
    other_axes_vel: [f32; 2],
) -> Option<f32> {
    debug_assert!(axis_vel != 0.0);

    // time would collide
    let dt = (barrier_face.axis_pos - obj_axis_pos) / axis_vel;

    // filter by collision time
    if dt < min_dt || dt > max_dt {
        return None;
    }

    // filter by whether would actually collide rather than pass to the side of
    for i in 0..2 {
        let other_axis_obj_col_pos =
            obj_other_axes_pos[i] + other_axes_vel[i] * dt;

        let other_axis_barrier_col_pos_min =
            barrier_face.other_axes_pos[i];
        let other_axis_barrier_col_pos_max =
            other_axis_barrier_col_pos_min + barrier_face.other_axes_ext[i];

        if other_axis_obj_col_pos < other_axis_barrier_col_pos_min {
            return None;
        }
        if other_axis_obj_col_pos > other_axis_barrier_col_pos_max {
            return None;
        }
    }

    // done
    Some(dt)
}
