
use crate::{
    util::quad_mesh::{
        QuadMesh,
        Quad,
    },
    gui::{
        GuiBlock,
        GuiNode,
        GuiGlobalContext,
        GuiSpatialContext,
        DimChildSets,
    }
};
use graphics::{
    Renderer,
    modifier::Transform3,
    frame_content::{
        Canvas2,
    },
};
use std::f32::consts::PI;
use rand::Rng;
use vek::*;


#[derive(Debug)]
pub struct GuiTitleBlock {
    pixel_mesh: QuadMesh,
    pixels: Vec<Vec3<f32>>,
}

impl GuiTitleBlock {
    pub fn new<R: Rng>(renderer: &Renderer, rng: &mut R) -> Self {
        let quads = [
            // front (-z)
            ([0, 0, 0], [0, 1, 0], [1, 0, 0], [1.0; 3]),
            
            // left (-x)
            ([0, 0, 1], [0, 1, 0], [0, 0, -1], [0.75; 3]),
            // right (+x)
            ([1, 0, 0], [0, 1, 0], [0, 0, 1], [0.75; 3]),
            // top (+y)
            ([0, 1, 0], [0, 0, 1], [1, 0, 0], [0.75; 3]),
            // bottom (-y)
            ([0, 0, 1], [0, 0, -1], [1, 0, 0], [0.75; 3]),
            
        ]
            .map(|(pos_start, pos_ext_1, pos_ext_2, [r, g, b])| Quad {
                pos_start: Vec3::from(pos_start).map(|n: i32| n as f32),
                pos_ext_1: Extent3::from(pos_ext_1).map(|n: i32| n as f32),
                pos_ext_2: Extent3::from(pos_ext_2).map(|n: i32| n as f32),
                tex_start: 0.0.into(),
                tex_extent: 1.0.into(),
                vert_colors: [Rgba::new(r, g, b, 1.0); 4],
                tex_index: 0,
            });
        let pixel_mesh = QuadMesh::create_init(renderer, &quads);

        let mut pixels = Vec::new();

        let image = image::load_from_memory(include_bytes!("title.png"))
            .unwrap()
            .into_luma8();
        for x in 0..image.width() {
            for y in 0..image.height() {
                if image[(x, y)].0[0] != 0 {
                    pixels.push(Vec3 {
                        x: x as f32 - image.width() as f32 / 2.0,
                        y: image.height() as f32 - (y + 1) as f32,
                        z: rng.gen_range(-75.0..=-40.0),
                    });
                }
            }
        }

        GuiTitleBlock {
            pixel_mesh,
            pixels,
        }
    }

    pub fn update(&mut self, elapsed: f32) {
        for pixel in &mut self.pixels {
            pixel.z = f32::min(0.0, pixel.z + 110.0 * elapsed);
        }
    }
}

//const WIDTH: f32 = 447.0;
//const HEIGHT: f32 = 64.0;

impl<'a> GuiBlock<'a, DimChildSets, DimChildSets> for &'a GuiTitleBlock {
    type Sized = GuiTitleNode<'a>;

    fn size(
        self,
        _: &GuiGlobalContext<'a>,
        (): (),
        (): (),
        scale: f32,
    ) -> (f32, f32, Self::Sized)
    {
        let sized = GuiTitleNode {
            inner: self,
            scale,
        };
        (447.0 * scale, 64.0 * scale, sized)
    }
}


#[derive(Debug)]
pub struct GuiTitleNode<'a> {
    inner: &'a GuiTitleBlock,
    scale: f32,
}

impl<'a> GuiNode<'a> for GuiTitleNode<'a> {
    fn blocks_cursor(&self, _: GuiSpatialContext<'a>) -> bool { false }

    fn draw(self, ctx: GuiSpatialContext<'a>, canvas: &mut Canvas2<'a, '_>) {
        //canvas.reborrow()
        //    .color([1.0, 0.0, 0.0, 1.0])
        //    .draw_solid([447.0 * self.scale, 64.0 * self.scale]);
        let mut canvas = canvas
            .reborrow()
            .scale(self.scale)
            .translate([0.0, 32.0])
            .begin_3d_perspective(
                [447.0, 64.0],
                [0.0, 0.0, -40.0],
                Quaternion::identity(),
                PI * 0.0835
            )
            .scale([1.0, 1.15, 1.0])
            .modify(Transform3(Mat4::new(
                1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.0175, 0.0, 1.0,
            )));
        for &pixel in &self.inner.pixels {
            canvas.reborrow()
                .translate(pixel)
                .draw_mesh(
                    &self.inner.pixel_mesh,
                    &ctx.resources().menu_title_pixel,
                );
        }
    }
}
