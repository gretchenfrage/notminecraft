
use crate::{
    client::{
        PLAYER_HEIGHT,
        meshing::mob_mesher::MobMesher,
    },
    asset::Assets,
    gui::prelude::*,
};
use graphics::prelude::*;
use std::f32::consts::*;
use vek::*;


#[derive(Debug)]
pub struct CharMesh {
    head: Mesh,
    torso: Mesh,
    leg: Mesh,
    arm: Mesh,
}

impl CharMesh {
    pub fn new(ctx: &GuiGlobalContext) -> Self {
        let char_mesher = MobMesher {
            gpu_vec_ctx: ctx.renderer.borrow(),
            tex_size: [64, 32].into(),
        };
        CharMesh {
            head: char_mesher.make_part([0, 0], [8, 8, 8], [0.5, 0.5, 0.5]),
            torso: char_mesher.make_part([16, 16], [8, 12, 4], [0.5, 0.0, 0.5]),
            leg: char_mesher.make_part([0, 16], [4, 12, 4], [0.5, 10.0 / 12.0, 0.5]),
            arm: char_mesher.make_part([40, 16], [4, 12, 4], [0.5, 10.0 / 12.0, 0.5]),
        }
    }

    pub fn draw<'a>(
        &'a self,
        canvas: &mut Canvas3<'a, '_>,
        assets: &'a Assets,
        head_pitch: f32,
        pointing: bool,
    ) {
        let mut canvas = canvas.reborrow()
            .scale(PLAYER_HEIGHT / 32.0);
        canvas.reborrow()
            .translate([0.0, 12.0, 0.0])
            .draw_mesh(&self.torso, &assets.mob_char);
        canvas.reborrow()
            .translate([0.0, 28.0, 0.0])
            .rotate(Quaternion::rotation_x(-head_pitch))
            .draw_mesh(&self.head, &assets.mob_char);
        canvas.reborrow()
            .translate([-2.0, 10.0, 0.0])
            .draw_mesh(&self.leg, &assets.mob_char);
        canvas.reborrow()
            .translate([2.0, 10.0, 0.0])
            .draw_mesh(&self.leg, &assets.mob_char);
        canvas.reborrow()
            .translate([-6.0, 22.0, 0.0])
            .draw_mesh(&self.arm, &assets.mob_char);
        let mut arm_pitch = 0.0;
        if pointing {
            arm_pitch -= head_pitch;
            arm_pitch -= PI / 2.0;
        }
        canvas.reborrow()
            .translate([6.0, 22.0, 0.0])
            .rotate(Quaternion::rotation_x(arm_pitch))
            .draw_mesh(&self.arm, &assets.mob_char);
    }
}


#[derive(Debug)]
pub struct CharMeshGuiBlock<'a> {
    pub char_mesh: &'a CharMesh,
    pub head_pitch: f32,
    pub pointing: bool,
}

impl<'a> GuiNode<'a> for SimpleGuiBlock<CharMeshGuiBlock<'a>> {
    never_blocks_cursor_impl!();

    fn draw(self, ctx: GuiSpatialContext<'a>, canvas: &mut Canvas2<'a, '_>) {
        let mut canvas = canvas.reborrow()
            .scale(self.size)
            .begin_3d(
                ViewProj::orthographic(
                    [0.0, PLAYER_HEIGHT / 2.0, -5.0],
                    Quaternion::identity(),
                    3.2,
                    self.size
                ),
                Fog::None,
            )
            .rotate(Quaternion::rotation_y(PI));
        self.inner.char_mesh.draw(&mut canvas, ctx.assets(), self.inner.head_pitch, self.inner.pointing);
    }
}
