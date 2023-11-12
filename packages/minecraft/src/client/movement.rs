
use crate::gui::prelude::*;
use vek::*;


/// Determine direction trying to walk from WASD keys and direction facing
/// (yaw). Output vector with magnitude either 0 or 1.
pub fn walking_xz(ctx: &GuiGlobalContext, yaw: f32) -> Vec2<f32> {
    let mut walking_xz = Vec2::from(0.0);
    if ctx.focus_level == FocusLevel::MouseCaptured {
        if ctx.pressed_keys_semantic.contains(&VirtualKeyCode::W) {
            walking_xz.y += 1.0;
        }
        if ctx.pressed_keys_semantic.contains(&VirtualKeyCode::S) {
            walking_xz.y -= 1.0;
        }
        if ctx.pressed_keys_semantic.contains(&VirtualKeyCode::D) {
            walking_xz.x += 1.0;
        }
        if ctx.pressed_keys_semantic.contains(&VirtualKeyCode::A) {
            walking_xz.x -= 1.0;
        }
    }
    walking_xz.rotate_z(yaw);
    if walking_xz != Vec2::from(0.0) {
        walking_xz.normalize();
    }
    walking_xz
}

/// Given target walking direction, accelerate as appropriate.
pub fn walking_accel(
    mut walking_xz: Vec2<f32>,
    vel: &mut Vec3<f32>,
    elapsed: f32,
) {
    const WALK_SPEED: f32 = 4.0;
    const WALK_ACCEL: f32 = 50.0;
    const WALK_DECEL: f32 = 30.0;

    // walking
    walking_xz *= WALK_SPEED;

    // accelerate self.vel xz towards the target value of walking_xz
    let accel_rate = if walking_xz != Vec2::from(0.0) {
        WALK_ACCEL
    } else {
        WALK_DECEL
    };

    let mut vel_xz = Vec2::new(vel.x, vel.z);
    let vel_xz_deviation = walking_xz - vel_xz;
    let vel_xz_deviation_magnitude = vel_xz_deviation.magnitude();
    let max_delta_vel_xz_magnitude = accel_rate * elapsed;
    if max_delta_vel_xz_magnitude > vel_xz_deviation_magnitude {
        vel_xz = walking_xz;
    } else {
        vel_xz += vel_xz_deviation / vel_xz_deviation_magnitude * max_delta_vel_xz_magnitude;
    }
    vel.x = vel_xz.x;
    vel.z = vel_xz.y;
}
