
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
        }
    }
}

#[derive(Debug, Clone)]
pub struct MovementController {
    // tiles per second.
    pub walk_speed: f32,
    // tiles per second.
    pub fly_speed: f32,
    // radians per mouse movement unit (possibly pixels?)
    pub mouse_sensitivity: f32,

    pub cam_pos: Vec3<f32>,
    pub cam_pitch: f32,
    pub cam_yaw: f32,
    pub cam_fov: f32,
}

impl Default for MovementController {
    fn default() -> Self {
        let move_speed = 8.0;

        MovementController {
            walk_speed: move_speed,
            fly_speed: move_speed,
            mouse_sensitivity: 1.0 / 1600.0,

            cam_pos: [0.0, 80.0, 0.0].into(),
            cam_pitch: f32::to_radians(-30.0),
            cam_yaw: 0.0,
            cam_fov: f32::to_radians(90.0),
        }
    }
}

impl MovementController {
    pub fn view_proj(&self, size: Extent2<f32>) -> ViewProj {
        let cam_dir =
            Quaternion::rotation_x(self.cam_pitch)
            * Quaternion::rotation_y(self.cam_yaw);
        
        ViewProj::perspective(
            self.cam_pos,
            cam_dir,
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
        if ctx.focus_level != FocusLevel::MouseCaptured { return }

        // walk's <x,y> is the world grid's <x,z> 
        let mut walk = Vec2::from(0.0);

        // determine walking direction relative to cam yaw
        if ctx.pressed_keys_semantic.contains(&bindings.move_forward) {
            walk.y += 1.0;
        }
        if ctx.pressed_keys_semantic.contains(&bindings.move_backward) {
            walk.y -= 1.0;
        }
        if ctx.pressed_keys_semantic.contains(&bindings.move_right) {
            walk.x += 1.0;
        }
        if ctx.pressed_keys_semantic.contains(&bindings.move_left) {
            walk.x -= 1.0;
        }

        // give it the correct magnitude
        if walk != Vec2::new(0.0, 0.0) {
            walk.normalize();
        }
        walk *= self.walk_speed;

        // rotate it in accordance with cam yaw
        walk.rotate_z(self.cam_yaw);

        // apply to position
        self.cam_pos += Vec3::new(walk.x, 0.0, walk.y) * elapsed;

        // determine flying direction
        let mut fly = 0.0;
        if ctx.pressed_keys_semantic.contains(&bindings.move_up) {
            fly += 1.0;
        }
        if ctx.pressed_keys_semantic.contains(&bindings.move_down) {
            fly -= 1.0;
        }

        // give it the correct magnitude
        fly *= self.fly_speed;

        // apply to position
        self.cam_pos += Vec3::new(0.0, fly, 0.0) * elapsed;
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
