
use crate::{
    gui::prelude::*,
    physics::prelude::*,
    client::{
        CAMERA_HEIGHT,
        MAX_BOB_SHIFT_V,
        MAX_BOB_SHIFT_H,
        MAX_BOB_ROLL_DEGS,
        meshing::char_mesh::CharMesh,
        MaybePendingChunkMesh,
        cam_dir,
    },
    message::CharState,
    util::sparse_vec::SparseVec,
};
use graphics::{
    prelude::*,
    frame_content::{
        DrawObj2,
        DrawSky,
    }
};
use chunk_data::*;
use std::f32::consts::*;
use vek::*;


/// GUI block that draws the 3D game world from the player's perspective.
#[derive(Debug)]
pub struct WorldGuiBlock<'a> {
    // TODO: probably make this just like a substruct within client
    pub pos: Vec3<f32>,
    pub pitch: f32,
    pub yaw: f32,
    pub pointing: bool,
    pub load_dist: u8,

    pub char_mesh: &'a CharMesh,
    pub char_name_layed_out: &'a LayedOutTextBlock,

    pub my_client_key: Option<usize>,
    pub client_char_state: &'a SparseVec<CharState>,
    pub client_char_name_layed_out: &'a SparseVec<LayedOutTextBlock>,

    pub day_night_time: f32,
    pub stars: &'a Mesh,
    pub white_pixel: &'a GpuImageArray,

    pub bob_animation: f32,
    pub third_person: bool,

    pub chunks: &'a LoadedChunks,
    pub tile_blocks: &'a PerChunk<ChunkBlocks>,
    pub tile_meshes: &'a mut PerChunk<MaybePendingChunkMesh>,
}

impl<'a> GuiNode<'a> for SimpleGuiBlock<WorldGuiBlock<'a>> {
    simple_blocks_cursor_impl!();

    fn draw(self, ctx: GuiSpatialContext<'a>, canvas: &mut Canvas2<'a, '_>) {
        let SimpleGuiBlock { inner, size, scale: _ } = self;

        // apply any pending chunk tile mesh patches
        for (cc, ci) in inner.chunks.iter() {
            if let &mut MaybePendingChunkMesh::ChunkMesh(ref mut chunk_mesh) = inner.tile_meshes.get_mut(cc, ci) {
                chunk_mesh.patch(&*ctx.global.renderer.borrow());
            }
        }

        // bob animation
        let mut bob_roll = 0.0;
        let mut bob_translate = Vec3::from(0.0);
        if !inner.third_person {
            let bob_animation_sine = f32::sin(inner.bob_animation * 2.0 * PI);
            bob_roll = bob_animation_sine * f32::to_radians(MAX_BOB_ROLL_DEGS);
            let bob_shift = Vec2 {
                x: bob_animation_sine * MAX_BOB_SHIFT_H,
                y: -(bob_animation_sine * bob_animation_sine) * MAX_BOB_SHIFT_V,
            };
            bob_translate = Vec3 {
                x: f32::cos(inner.yaw) * bob_shift.x,
                y: bob_shift.y,
                z: f32::sin(inner.yaw) * bob_shift.x,
            };
        }

        // determine view proj
        let cam_dir = cam_dir(inner.pitch, inner.yaw);
        let mut cam_pos = inner.pos + Vec3::new(0.0, CAMERA_HEIGHT, 0.0) + bob_translate;
        if inner.third_person {
            cam_pos -= cam_dir * 5.0;
        }
        let view_proj = ViewProj::perspective(
            // position
            cam_pos,
            // direction
            Quaternion::rotation_x(inner.pitch)
            * Quaternion::rotation_z(bob_roll)
            * Quaternion::rotation_y(inner.yaw),
            // field of view
            f32::to_radians(120.0),
            // size
            size,
        );

        // determine fog
        let fog = match ctx.settings().fog {
            true => Fog::Earth {
                start: 100.0,
                end: 150.0,
                day_night_time: inner.day_night_time,
            },
            false => Fog::None,
        };

        // draw sky
        canvas.reborrow()
            .scale(self.size)
            .draw(DrawObj2::Sky(DrawSky {
                view_proj,
                day_night_time: inner.day_night_time,
            }));

        // draw stars
        // intensity of it being day as opposed to night
        let day = (f32::sin(inner.day_night_time * PI * 2.0) + 0.6).clamp(0.0, 1.0);
        canvas.reborrow()
            .scale(self.size)
            .begin_3d(view_proj, Fog::None)
            .translate(cam_pos)
            .rotate(Quaternion::rotation_x(-inner.day_night_time * PI * 2.0))
            .color([1.0, 1.0, 1.0, 1.0 - day])
            .draw_mesh(inner.stars, inner.white_pixel);

        // draw sun and moon
        {
            let mut canvas = canvas.reborrow()
                .scale(self.size)
                .begin_3d(view_proj, Fog::None)
                .translate(cam_pos)
                .rotate(Quaternion::rotation_x(-inner.day_night_time * PI * 2.0));
            let sun_moon_transl = Vec3::new(-0.5, -0.5, 1.6);
            //let sun_oversat = 0.22 + day * 1.3;
            let sun_oversat = (day + 1.0).powf(2.0) - 0.8;
            canvas.reborrow()
                .translate(sun_moon_transl)
                .color([sun_oversat, sun_oversat, sun_oversat, 1.0])
                .draw_image(&ctx.assets().sun, 0, 0.0, 1.0);
            canvas.reborrow()
                .rotate(Quaternion::rotation_x(PI))
                .translate(sun_moon_transl)
                .draw_image(&ctx.assets().moon, 0, 0.0, 1.0);
        }
        

        // begin 3D perspective
        let mut canvas = canvas.reborrow()
            .scale(self.size)
            .begin_3d(view_proj, fog);

        // chunks
        for (cc, ci) in inner.chunks.iter() {
            // frustum culling
            let pos = (cc * CHUNK_EXTENT).map(|n| n as f32);
            let ext = CHUNK_EXTENT.map(|n| n as f32).into();
            if !view_proj.is_volume_visible(pos, ext) {
                continue;
            }

            // blocks
            if let &MaybePendingChunkMesh::ChunkMesh(ref chunk_mesh) = (&*inner.tile_meshes).get(cc, ci) {
                canvas.reborrow()
                    .translate(pos)
                    .draw_mesh(chunk_mesh.mesh(), &ctx.assets().blocks);
            }

            // debug outline
            if ctx.settings().chunk_outline {
                draw_debug_box(&mut canvas, pos, ext);
            }
        }

        // my character
        if inner.third_person {
            let mut canvas = canvas.reborrow()
                .translate(inner.pos)
                .rotate(Quaternion::rotation_y(-inner.yaw));
            inner.char_mesh.draw(&mut canvas, ctx.assets(), inner.pitch, inner.pointing);
            canvas.reborrow()
                .translate([0.0, 2.0, 0.0])
                .scale(0.25 / 16.0)
                .scale([1.0, -1.0, 1.0])
                .rotate(Quaternion::rotation_y(PI))
                .color([1.0, 1.0, 1.0, 0.5])
                .draw_text(&inner.char_name_layed_out);
        }

        // other characters
        if let Some(my_client_key) = inner.my_client_key {
            for (client_key, client_char_state) in inner.client_char_state.iter() {
                if client_key == my_client_key {
                    continue;
                }

                // TODO: deduplicate this part with above
                let mut canvas = canvas.reborrow()
                    .translate(client_char_state.pos)
                    .rotate(Quaternion::rotation_y(-client_char_state.yaw));
                inner.char_mesh.draw(&mut canvas, ctx.assets(), client_char_state.pitch, client_char_state.pointing);
                canvas.reborrow()
                    .translate([0.0, 2.0, 0.0])
                    .scale(0.25 / 16.0)
                    .scale([1.0, -1.0, 1.0])
                    .rotate(Quaternion::rotation_y(PI))
                    .draw_text(&inner.client_char_name_layed_out[client_key]);
            }
        }

        // outline for block being looked at
        let getter = inner.chunks.getter();
        if let Some(looking_at) = compute_looking_at(
            // pos
            cam_pos,
            // dir
            cam_dir,
            // reach
            50.0,
            &getter,
            &inner.tile_blocks,
            ctx.game(),
        ) {
            const GAP: f32 = 0.002;

            let mut canvas = canvas.reborrow()
                .translate(looking_at.tile.gtc().map(|n| n as f32))
                .color([0.0, 0.0, 0.0, 0.65]);

            for face in FACES {
                for edge in face.to_edges() {
                    let [start, end] = edge.to_corners()
                        .map(|corner| corner.to_poles()
                            .map(|pole| match pole {
                                Pole::Neg => 0.0 + GAP,
                                Pole::Pos => 1.0 - GAP,
                            })
                            + face.to_vec().map(|n| n as f32) * 2.0 * GAP);
                    canvas.reborrow()
                        .draw_line(start, end);
                }
            }
        }

        // debug box for load dist
        if ctx.settings().load_dist_outline {
            let load_dist = inner.load_dist as f32;
            let chunk_ext = CHUNK_EXTENT.map(|n| n as f32);

            let mut load_cc_start = (inner.pos / chunk_ext).map(|n| n.floor()) - load_dist;
            load_cc_start.y = 0.0;
            let mut load_cc_ext = Vec3::from(1.0 + load_dist * 2.0);
            load_cc_ext.y = 2.0;

            draw_debug_box(
                &mut canvas,
                load_cc_start * chunk_ext,
                load_cc_ext * chunk_ext,
            );
        }
    }
}

fn draw_debug_box(canvas: &mut Canvas3, pos: impl Into<Vec3<f32>>, ext: impl Into<Vec3<f32>>) {
    let ext = ext.into();
    let mut canvas = canvas.reborrow()
        .translate(pos.into())
        .color(Rgba::red());
    for edge in EDGES {
        let [start, end] = edge.to_corners()
            .map(|corner| corner.to_poles()
                .map(|pole| match pole {
                    Pole::Neg => 0.0,
                    Pole::Pos => 1.0,
                }) * ext);
        canvas.reborrow()
            .draw_line(start, end);
    }
}
