
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
    ) -> Option<Collision<W::ObjectId>>
    {
        let mut first: Option<Collision<W::ObjectId>> = None;

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

            // pos/vel/ext along this axis and the other axes
            let axis_vel = vel[axis as usize];
            let axis_pos = pos[axis as usize];

            let other_axes_vel = other_axes.map(|axis2| vel[axis2 as usize]);
            let other_axes_pos = other_axes.map(|axis2| pos[axis2 as usize]);
            let other_axes_ext = other_axes
                .map(|axis2| self.ext[axis2 as usize]);
            
            // direction of movement along this axis
            // (skip loop iteration if not moving along this axis)
            let axis_vel_pole =
                match Pole::from_sign(Sign::of_f32(axis_vel)) {
                    Some(pole) => pole,
                    None => continue,
                };

            // change axis_pos to pos of physics obj's face in direction of
            // movement
            let axis_pos = axis_pos
                + match axis_vel_pole {
                    Pole::Neg => 0.0,
                    Pole::Pos => self.ext[axis as usize],
                };

            // face of barrier rects may collide with along this axis
            let world_obj_face = Face::from_axis_pole(axis, -axis_vel_pole);

            // broadphase-visit barrier boxes
            for gtc in gtc_broadphase.clone() {
                world_geometry.tile_geometry(gtc, |aa_box, world_obj_id| {
                    // see if collides along this axis
                    if let Some(dt) = BarrierRect
                        ::new(
                            aa_box.translate(gtc.map(|n| n as f32)),
                            world_obj_face,
                        )
                        .collision_dt(
                            min_dt,
                            max_dt,
                            axis_pos,
                            axis_vel,
                            other_axes_pos,
                            other_axes_vel,
                            other_axes_ext,
                        )
                    {
                        // compare if does
                        if first
                            .as_ref()
                            .map(|first| dt < first.dt)
                            .unwrap_or(true)
                        {
                            first = Some(Collision {
                                dt,
                                world_obj_face,
                                world_obj_id,
                            });
                        }
                    }
                });
            }
        }

        first
    }
}

/// Face of an AA box of world geometry..
///
/// Does not itself contain information about which face is facing (that is,
/// which axis it is normal to and which direction along that axis the barrier
/// faces).
#[derive(Debug, Copy, Clone)]
struct BarrierRect {
    axis_pos: f32,
    other_axes_pos: [f32; 2],
    other_axes_ext: [f32; 2],
}

impl BarrierRect {
    fn new(aa_box: AaBox, face: Face) -> Self {
        let (axis, pole) = face.to_axis_pole();
        let other_axes = axis.other_axes();

        let axis_pos =
            aa_box.pos[axis as usize]
            + match pole {
                Pole::Neg => 0.0,
                Pole::Pos => aa_box.ext[axis as usize],
            };
        let mut other_axes_pos = [0.0; 2];
        let mut other_axes_ext = [0.0; 2];
        for i in 0..2 {
            other_axes_pos[i] = aa_box.pos[other_axes[i] as usize];
            other_axes_ext[i] = aa_box.ext[other_axes[i] as usize];
        }

        BarrierRect {
            axis_pos,
            other_axes_pos,
            other_axes_ext,
        }
    }

    fn collision_dt(
        self,
        min_dt: f32,
        max_dt: f32,
        obj_axis_pos: f32,
        obj_axis_vel: f32,
        obj_other_axes_pos: [f32; 2],
        obj_other_axes_vel: [f32; 2],
        obj_other_axes_ext: [f32; 2],
    ) -> Option<f32> {
        debug_assert!(obj_axis_vel != 0.0);

        // time would collide
        let dt = (self.axis_pos - obj_axis_pos) / obj_axis_vel;

        // filter by collision time
        if dt < min_dt || dt > max_dt {
            return None;
        }

        // filter by whether would actually collide rather than
        // pass to the side of
        for i in 0..2 {
            // tangential (to rect) axis obj/rect collision position min/max
            let tan_axis_obj_col_pos_min =
                obj_other_axes_pos[i] + obj_other_axes_vel[i] * dt;
            let tan_axis_obj_col_pos_max =
                tan_axis_obj_col_pos_min + obj_other_axes_ext[i];
            let tan_axis_rect_col_pos_min =
                self.other_axes_pos[i];
            let tan_axis_rect_col_pos_max =
                tan_axis_rect_col_pos_min + self.other_axes_ext[i];

            if tan_axis_obj_col_pos_max < tan_axis_rect_col_pos_min {
                return None;
            }
            if tan_axis_obj_col_pos_min > tan_axis_rect_col_pos_max {
                return None;
            }
        }

        // done
        Some(dt)
    }
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
                .map(move |x| Vec3  { x, y, z })))
}
