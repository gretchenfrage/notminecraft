
use crate::gui::prelude::*;
use mesh_data::*;
use graphics::prelude::*;
use std::f32::consts::*;
use rand::prelude::*;
use rand_chacha::ChaCha20Rng;
use image::{
    DynamicImage,
    RgbaImage,
};
use vek::*;


#[derive(Debug)]
pub struct StarsMesh {
    white_pixel: GpuImageArray,
    stars: Mesh,
}

impl StarsMesh {
    pub fn new(ctx: &GuiGlobalContext) -> Self {
        let mut image = RgbaImage::new(1, 1);
        image[(0, 0)] = [0xff; 4].into();
        let white_pixel = ctx.renderer.borrow().load_image_array_raw(
            [1, 1].into(),
            [DynamicImage::from(image)],
        );

        let mut rng = ChaCha20Rng::from_seed([0; 32]);
        let mut stars = MeshData::new();
        let mut star = MeshData::new();
        for _ in 0..1500 {
            let u1: f32 = rng.gen_range(0.0..1.0);
            let u2: f32 = rng.gen_range(0.0..1.0);
            let u3: f32 = rng.gen_range(0.0..1.0);

            let w = (1.0 - u1).sqrt() * (2.0 * PI * u2).sin();
            let x = (1.0 - u1).sqrt() * (2.0 * PI * u2).cos();
            let y = u1.sqrt() * (2.0 * PI * u3).sin();
            let z = u1.sqrt() * (2.0 * PI * u3).cos();

            let star_quat = Quaternion { w, x, y, z };
            let star_size = rng.gen_range(0.5..2.0);
            let star_light = rng.gen_range(1.0f32..10.0).powf(2.0) / 100.0;

            star.add_quad(&Quad {
                pos_start: [-star_size / 2.0, -star_size / 2.0, 300.0].into(),
                pos_ext_1: [0.0, star_size, 0.0].into(),
                pos_ext_2: [star_size, 0.0, 0.0].into(),
                tex_start: 0.0.into(),
                tex_extent: 1.0.into(),
                vert_colors: [[1.0, 1.0, 1.0, star_light].into(); 4],
                tex_index: 0,
            });

            for v in &mut star.vertices {
                v.pos = star_quat * v.pos;
            }

            stars.extend(star.vertices.iter().copied(), star.indices.iter().copied());
            star.clear();
        }
        let stars = stars.upload(&*ctx.renderer.borrow());

        StarsMesh {
            white_pixel,
            stars,
        }
    }

    pub fn draw<'a>(&'a self, canvas: &mut Canvas3<'a, '_>) {
        canvas.reborrow()
            .draw_mesh(&self.stars, &self.white_pixel);
    }
}
