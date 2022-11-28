
use crate::gui::{
    VirtualKeyCode,
    GuiGlobalContext,
    FocusLevel,
};
use graphics::view_proj::ViewProj;
use std::f32::consts::PI;
use vek::*;


#[derive(Debug, Clone)]
pub struct KeyBindings {
    pub move_forward: VirtualKeyCode,
    pub move_backward: VirtualKeyCode,
    pub move_left: VirtualKeyCode,
    pub move_right: VirtualKeyCode,
    pub move_up: VirtualKeyCode,
    pub move_down: VirtualKeyCode,
    pub move_faster: VirtualKeyCode,
}

impl Default for KeyBindings {
    fn default() -> Self {
        KeyBindings {
            move_forward: VirtualKeyCode::W,
            move_backward: VirtualKeyCode::S,
            move_left: VirtualKeyCode::A,
            move_right: VirtualKeyCode::D,
            move_up: VirtualKeyCode::Space,
            move_down: VirtualKeyCode::LShift,
            move_faster: VirtualKeyCode::LControl,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MovementController {
    pub fly_h_speed: f32,
    pub fly_v_speed: f32,

    pub fly_h_speed_sprinting: f32,
    pub fly_v_speed_sprinting: f32,

    pub acceleration_h: f32,
    pub acceleration_v: f32,

    pub mouse_sensitivity: f32,

    pub vel_h: Vec2<f32>,
    pub vel_v: f32,

    pub cam_pos: Vec3<f32>,
    pub cam_pitch: f32,
    pub cam_yaw: f32,
    pub cam_fov: f32,
}

impl Default for MovementController {
    fn default() -> Self {
        MovementController {
            fly_h_speed: 10.92,
            fly_v_speed: 8.0,

            fly_h_speed_sprinting: 21.6,
            fly_v_speed_sprinting: 16.0,

            acceleration_h: 50.0,
            acceleration_v: 50.0,

            vel_h: 0.0.into(),
            vel_v: 0.0,

            mouse_sensitivity: 1.0 / 1600.0,

            cam_pos: [0.0, 80.0, 0.0].into(),
            cam_pitch: f32::to_radians(-30.0),
            cam_yaw: 0.0,
            cam_fov: f32::to_radians(90.0),
        }
    }
}

fn apply_friction_vec2(
    vel: &mut Vec2<f32>,
    target_vel: Vec2<f32>,
    acceleration_time: f32,
) {
    let mut delta = target_vel - *vel;
    let delta_mag = delta.magnitude();
    if delta_mag > acceleration_time {
        delta /= delta_mag;
        delta *= acceleration_time;
    }
    *vel += delta;
}

fn apply_friction_f32(
    vel: &mut f32,
    target_vel: f32,
    acceleration_time: f32,
) {
    let mut delta = target_vel - *vel;
    let delta_mag = delta.abs();
    if delta_mag > acceleration_time {
        delta /= delta_mag;
        delta *= acceleration_time;
    }
    *vel += delta;
}

impl MovementController {
    pub fn cam_rot(&self) -> Quaternion<f32> {
        Quaternion::rotation_x(self.cam_pitch)
            * Quaternion::rotation_y(self.cam_yaw)
    }

    pub fn cam_dir(&self) -> Vec3<f32> {
        let dir =
            Quaternion::rotation_y(-self.cam_yaw)
            * Quaternion::rotation_x(-self.cam_pitch)
            * Vec3::new(0.0, 0.0, 1.0);
        //debug!(magnitude=%dir.magnitude());
        dir
        //quat_mult_lh(self.cam_rot(), Vec3::new(0.0, 0.0, 1.0))
    }

    pub fn view_proj(&self, size: Extent2<f32>) -> ViewProj {      
        ViewProj::perspective(
            self.cam_pos,
            self.cam_rot(),
            self.cam_fov,
            size.w / size.h,
        )
    }

    pub fn update(
        &mut self,
        ctx: &GuiGlobalContext,
        bindings: &KeyBindings,
        elapsed: f32,
    ) {
        // move_h's <x,y> is the world grid's <x,z> 
        let mut move_h = Vec2::from(0.0);
        let mut move_v = 0.0;

        if ctx.focus_level == FocusLevel::MouseCaptured {
            let sprinting =
                ctx.pressed_keys_semantic.contains(&bindings.move_faster);

            // determine horizontal movement direction relative to cam yaw
            if ctx.pressed_keys_semantic.contains(&bindings.move_forward) {
                move_h.y += 1.0;
            }
            if ctx.pressed_keys_semantic.contains(&bindings.move_backward) {
                move_h.y -= 1.0;
            }
            if ctx.pressed_keys_semantic.contains(&bindings.move_right) {
                move_h.x += 1.0;
            }
            if ctx.pressed_keys_semantic.contains(&bindings.move_left) {
                move_h.x -= 1.0;
            }

            // give it the correct magnitude
            if move_h != Vec2::new(0.0, 0.0) {
                move_h.normalize();
            }
            move_h *= match sprinting {
                false => self.fly_h_speed,
                true => self.fly_h_speed_sprinting,
            };

            // rotate it in accordance with cam yaw
            move_h.rotate_z(self.cam_yaw);


            // determine vertical movement direction
            if ctx.pressed_keys_semantic.contains(&bindings.move_up) {
                move_v += 1.0;
            }
            if ctx.pressed_keys_semantic.contains(&bindings.move_down) {
                move_v -= 1.0;
            }

            // give it the correct magnitude
            move_v *= match sprinting {
                false => self.fly_v_speed,
                true => self.fly_v_speed_sprinting,
            };
        }

        // apply to velocity
        apply_friction_vec2(
            &mut self.vel_h,
            move_h,
            self.acceleration_h * elapsed,
        );

        // apply to position
        self.cam_pos += Vec3::new(self.vel_h.x, 0.0, self.vel_h.y) * elapsed;

        // apply to velocity
        apply_friction_f32(
            &mut self.vel_v,
            move_v,
            self.acceleration_v * elapsed,
        );

        // apply to position
        self.cam_pos.y += self.vel_v * elapsed;
    }

    pub fn on_captured_mouse_move(
        &mut self,
        amount: Vec2<f32>,
    ) {
        self.cam_pitch += -amount.y * self.mouse_sensitivity;
        self.cam_pitch = f32::max(-PI / 2.0, self.cam_pitch);
        self.cam_pitch = f32::min(PI / 2.0, self.cam_pitch);

        self.cam_yaw += -amount.x * self.mouse_sensitivity;
        self.cam_yaw %= PI * 2.0;
    }
}
