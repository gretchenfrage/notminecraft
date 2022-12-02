
use crate::game_data::{
    GameData,
    BlockPhysicsLogic,
};
use chunk_data::{
    AXES,
    Getter,
    PerChunk,
    ChunkBlocks,
    Face,
    Pole,
    Sign,
};
use vek::*;


/// Do a tick of physics to a physics object.
pub fn do_physics(
    mut dt: f32,
    pos: &mut Vec3<f32>,
    vel: &mut Vec3<f32>,
    ext: Extent3<f32>,
    getter: &Getter,
    tile_blocks: &PerChunk<ChunkBlocks>,
    game: &GameData,
) -> DidPhysics {
    const EPSILON: f32 = 0.0001;

    let mut on_ground = false;

    while dt > EPSILON {
        if let Some(collision) = first_collision(
            -EPSILON,
            dt,
            *pos,
            *vel,
            ext,
            getter,
            tile_blocks,
            game,
        ) {
            if collision.face == Face::PosY {
                on_ground = true;
            }

            *pos += *vel * collision.dt;
            vel[collision.face.to_axis() as usize] = 0.0;
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

/// Collision which will occur if a physics object continues to move at its
/// current velocity in the current world geometry.
#[derive(Debug, Copy, Clone)]
struct Collision {
    dt: f32,
    face: Face,
}

/// Compute the first collision that would occur for a physics object.
fn first_collision(
    min_dt: f32,
    max_dt: f32,
    pos: Vec3<f32>,
    vel: Vec3<f32>,
    ext: Extent3<f32>,
    getter: &Getter,
    tile_blocks: &PerChunk<ChunkBlocks>,
    game: &GameData,
) -> Option<Collision> {
    let mut first: Option<Collision> = None;
    collisions(
        min_dt,
        max_dt,
        pos,
        vel,
        ext,
        getter,
        tile_blocks,
        game,
        |collision| {
            if first.map(|first| collision.dt < first.dt).unwrap_or(true) {
                first = Some(collision);
            }
        }
    );
    first
}

/// Visit all the collisions that would occur for a physics object.
fn collisions<F: FnMut(Collision)>(
    min_dt: f32,
    max_dt: f32,
    pos: Vec3<f32>,
    vel: Vec3<f32>,
    ext: Extent3<f32>,
    getter: &Getter,
    tile_blocks: &PerChunk<ChunkBlocks>,
    game: &GameData,
    mut visit: F,
) {
    // for each axis, with its two complementary axes
    for axis in AXES {
        let other_axes = axis.other_axes();

        // pos/vel/ext along this axis and the other axes
        let axis_vel = vel[axis as usize];
        let axis_pos = pos[axis as usize];

        let other_axes_vel = other_axes.map(|axis2| vel[axis2 as usize]);
        let other_axes_pos = other_axes.map(|axis2| pos[axis2 as usize]);
        let other_axes_ext = other_axes.map(|axis2| ext[axis2 as usize]);
        
        // direction of movement along this axis
        // (skip loop iteration if not moving along this axis)
        let axis_vel_pole =
            match Pole::from_sign(Sign::of_f32(axis_vel)) {
                Some(pole) => pole,
                None => continue,
            };

        // change axis_pos to pos of physics object's face in direction of
        // movement
        let axis_pos = axis_pos
            + match axis_vel_pole {
                Pole::Neg => 0.0,
                Pole::Pos => ext[axis as usize],
            };

        // face of barrier rects may collide with along this axis
        let face_of_interest = Face::from_axis_pole(axis, -axis_vel_pole);

        // broadphase-visit barrier boxes
        broadphase_tiles(
            min_dt,
            max_dt,
            pos,
            vel,
            ext,
            |gtc| gtc_boxes(
                gtc,
                getter,
                tile_blocks,
                game,
                |bbox| {
                    // see if collides along this axis
                    if let Some(dt) = bbox
                        .face_rect(face_of_interest)
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
                        // visit if does
                        visit(Collision {
                            dt,
                            face: face_of_interest,
                        });
                    }
                }
            ),
        );
    }
}

/// Axis-aligned box barrier.
#[derive(Debug, Copy, Clone)]
struct BarrierBox {
    pos: Vec3<f32>,
    ext: Extent3<f32>,
}

impl BarrierBox {
    /// Express the given face of this barrier box as a barrier rect.
    fn face_rect(self, face: Face) -> BarrierRect {
        let (axis, pole) = face.to_axis_pole();
        let other_axes = axis.other_axes();

        let axis_pos =
            self.pos[axis as usize]
            + match pole {
                Pole::Neg => 0.0,
                Pole::Pos => self.ext[axis as usize],
            };
        let mut other_axes_pos = [0.0; 2];
        let mut other_axes_ext = [0.0; 2];
        for i in 0..2 {
            other_axes_pos[i] = self.pos[other_axes[i] as usize];
            other_axes_ext[i] = self.ext[other_axes[i] as usize];
        }

        BarrierRect {
            axis_pos,
            other_axes_pos,
            other_axes_ext,
        }
    }
}

/// Axis-aligned directional rectangle barrier.
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

/// Visit a superset of the gtcs a physics object may collide with.
fn broadphase_tiles<F: FnMut(Vec3<i64>)>(
    min_dt: f32,
    max_dt: f32,
    pos: Vec3<f32>,
    vel: Vec3<f32>,
    ext: Extent3<f32>,
    mut visit: F,
) {
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
    for z in min.z..=max.z {
        for y in min.y..=max.y {
            for x in min.x..=max.x {
                visit(Vec3 { x, y, z });
            }
        }
    }
}

/// Visit the barrier boxes at the given gtc.
fn gtc_boxes<F: FnMut(BarrierBox)>(
    gtc: Vec3<i64>,
    getter: &Getter,
    tile_blocks: &PerChunk<ChunkBlocks>,
    game: &GameData,
    mut visit: F,
) {
    let physics_logic = getter
        .gtc_get(gtc)
        .map(|tile| {
            let bid = tile.get(tile_blocks).get();
            game.blocks_physics_logic.get(bid)
        })
        .unwrap_or(&BlockPhysicsLogic::BasicCube);
    match physics_logic {
        &BlockPhysicsLogic::NoClip => (),
        &BlockPhysicsLogic::BasicCube => {
            visit(BarrierBox {
                pos: gtc.map(|n| n as f32),
                ext: 1.0.into(),
            });
        }
    }
}
