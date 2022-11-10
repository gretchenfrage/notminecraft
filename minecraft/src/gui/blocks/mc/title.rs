
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
    frame_content::{
        Canvas2,
    },
};
use vek::*;


#[derive(Debug)]
pub struct GuiTitleBlock {
    pixel_mesh: QuadMesh,
    pixels: Vec<Vec3<f32>>,
}

impl GuiTitleBlock {
    pub fn new(renderer: &Renderer) -> Self {
        let quads = [
            // front (-z)
            ([0, 0, 0], [0, 1, 0], [1, 0, 0], 1.0),
            
            // left (-x)
            ([0, 0, 1], [0, 1, 0], [0, 0, -1], 0.75),
            // right (+x)
            ([1, 0, 0], [0, 1, 0], [0, 0, 1], 0.75),
            // top (+y)
            ([0, 1, 0], [0, 0, 1], [1, 0, 0], 0.75),
            // bottom (-y)
            ([0, 0, 1], [0, 0, -1], [1, 0, 0], 0.75),
            
        ]
            .map(|(pos_start, pos_ext_1, pos_ext_2, col)| Quad {
                pos_start: Vec3::from(pos_start).map(|n: i32| n as f32),
                pos_ext_1: Extent3::from(pos_ext_1).map(|n: i32| n as f32),
                pos_ext_2: Extent3::from(pos_ext_2).map(|n: i32| n as f32),
                tex_start: 0.0.into(),
                tex_extent: 1.0.into(),
                vert_colors: [Rgba::new(col, col, col, 1.0); 4],
                tex_index: 0,
            });
        let pixel_mesh = QuadMesh::create_init(renderer, &quads);
        dbg!(&pixel_mesh);

        let mut pixels = Vec::new();

        let image = image::load_from_memory(include_bytes!("title.png"))
            .unwrap()
            .into_luma8();
        'outer: for x in 0..image.width() {
            for y in 0..image.height() {
                if image[(x, y)].0[0] != 0 {
                    pixels.push(Vec3 {
                        x: x as f32 - image.width() as f32 / 2.0,
                        y: image.height() as f32 / 2.0 - (y + 1) as f32,
                        z: 0.0,
                    });
                    //break 'outer;
                }
            }
        }

        GuiTitleBlock {
            pixel_mesh,
            pixels,
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
        canvas.reborrow()
            .color(Rgba::red())
            .draw_solid([447.0 * self.scale, 64.0 * self.scale])
            ;
        let mut canvas = canvas
            .reborrow()
            .translate([223.5 * self.scale, 32.0 * self.scale])
            .scale(self.scale)
            .begin_3d(
                
                Mat4::new(
                      356.6837,  0.0, 0.0, 0.0,
                    0.0,  -386.4658,   66.59978, -98.00946,
                           0.0,        0.0,     1.0,       0.5,
                    0.0, 0.45333248,        1.0, 32.122105,
                )
                
                /*
                Mat4::new(
                           1.0,        0.0,       0.0,   0.0,
                    -786.36847, -170.07373, -9465.644, 242.0,
                           0.0,        0.0,     0.001,   0.5,
                           0.0,        0.4, 1.0867925, 195.0,
                )*/
            );
            /*
            .begin_3d(/*Mat4::new(
                /*
                11.243, 4.2, 34.486, 0.0,
                0.0,    0.0,  -12.6, 1.0,
                0.0,    0.0,    0.1, 0.5,
                0.0,    0.0,    0.0, 1.0,
                */
                /*
                11.3226, 2.7871, 0.6132,  0.0,
                    0.0,  -12.6,    3.0, -0.5,
                    0.0,    0.0, 0.0001,  0.5,
                    0.0,    0.0,    0.0,  1.0,
                    */

                563.44, 19.38, 102.99, 227.18,
                0.0,  -604.93,  77.84, -91.32,
                0.0,      0.0, 0.0001,    0.5,
                0.0,     0.86,    1.0,  47.98,
            )*/Mat4 {
    cols: Vec4 {
        x: Vec4 {
            x: 356.6837,
            y: -2.5395966,
            z: 0.0,
            w: 0.07936245,
        },
        y: Vec4 {
            x: 32.186558,
            y: -386.4658,
            z: 0.0,
            w: 0.45333248,
        },
        z: Vec4 {
            x: -16.522638,
            y: 66.59978,
            z: -0.001,
            w: 1.0,
        },
        w: Vec4 {
            x: 81.26757,
            y: -98.00946,
            z: 0.5,
            w: 32.122105,
        },
    },
});*/
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


/*
use super::*;
use graphics::{
    frame_content::{Mesh, GpuImageArray, Vertex, Triangle},
};
use std::sync::Arc;
use rand::Rng;
use image::DynamicImage;

const TITLE_PIXELS: &'static [&'static str] = &[
    "█   █ █ █   █ ███ ███ ███ ███ ███ ███",
    "██ ██ █ ██  █ █   █   █ █ █ █ █    █ ",
    "█ █ █ █ █ █ █ ██  █   ██  ███ ██   █ ",
    "█   █ █ █  ██ █   █   █ █ █ █ █    █ ",
    "█   █ █ █   █ ███ ███ █ █ █ █ █    █ ",
];

const FACES_PER_TITLE_PIXEL: usize = 5;
const VERTS_PER_FACE: usize = 4;

const POS_X_FACE_POS: [Vec3<f32>; VERTS_PER_FACE] = [
    Vec3 { x: 1.0, y: 1.0, z: 0.0 },
    Vec3 { x: 1.0, y: 1.0, z: 1.0 },
    Vec3 { x: 1.0, y: 0.0, z: 1.0 },
    Vec3 { x: 1.0, y: 0.0, z: 0.0 },
];
const POS_Y_FACE_POS: [Vec3<f32>; VERTS_PER_FACE] = [
    Vec3 { x: 0.0, y: 1.0, z: 1.0 },
    Vec3 { x: 1.0, y: 1.0, z: 1.0 },
    Vec3 { x: 1.0, y: 1.0, z: 0.0 },
    Vec3 { x: 0.0, y: 1.0, z: 0.0 },
];  
const POS_Z_FACE_POS: [Vec3<f32>; VERTS_PER_FACE] = [
    Vec3 { x: 1.0, y: 1.0, z: 1.0 },
    Vec3 { x: 0.0, y: 1.0, z: 1.0 },
    Vec3 { x: 0.0, y: 0.0, z: 1.0 },
    Vec3 { x: 1.0, y: 0.0, z: 1.0 },
];
const NEG_X_FACE_POS: [Vec3<f32>; VERTS_PER_FACE] = [
    Vec3 { x: 0.0, y: 1.0, z: 1.0 },
    Vec3 { x: 0.0, y: 1.0, z: 0.0 },
    Vec3 { x: 0.0, y: 0.0, z: 0.0 },
    Vec3 { x: 0.0, y: 0.0, z: 1.0 },
];
const NEG_Y_FACE_POS: [Vec3<f32>; VERTS_PER_FACE] = [
    Vec3 { x: 0.0, y: 0.0, z: 0.0 },
    Vec3 { x: 1.0, y: 0.0, z: 0.0 },
    Vec3 { x: 1.0, y: 0.0, z: 1.0 },
    Vec3 { x: 0.0, y: 0.0, z: 1.0 },
];
const NEG_Z_FACE_POS: [Vec3<f32>; VERTS_PER_FACE] = [
    Vec3 { x: 0.0, y: 1.0, z: 0.0 },
    Vec3 { x: 1.0, y: 1.0, z: 0.0 },
    Vec3 { x: 1.0, y: 0.0, z: 0.0 },
    Vec3 { x: 0.0, y: 0.0, z: 0.0 },
];

const FACE_TEX: [Vec2<f32>; 4] = [
    Vec2 { x: 0.0, y: 0.0 },
    Vec2 { x: 1.0, y: 0.0 },
    Vec2 { x: 1.0, y: 1.0 },
    Vec2 { x: 0.0, y: 1.0 },
];

const FACE_TRIANGLES: [Triangle; 2] = [
    Triangle([0, 1, 2]),
    Triangle([0, 2, 3]),
];


#[derive(Debug, Clone)]
pub struct McTitleGuiBlock {
    title_cam_height: f32,
    title_cam_distance: f32,
    title_cam_fov: f32,
    title_angle: f32,
    title_translate: Vec3<f32>,
    
    title_pixel_mesh: Arc<Mesh>,
    title_pixel_texture: GpuImageArray,
    title_pixel_positions: Vec<Vec3<f32>>,
}

impl McTitleGuiBlock {
    pub fn new<R: Rng>(
        renderer: &Renderer,
        rng: &mut R,
        raw_title_pixel_texture: DynamicImage,
    ) -> Self
    {
        let title_pixel_vertices: Vec<Vertex> = [
            (POS_X_FACE_POS, 0.5),
            (POS_Y_FACE_POS, 0.5),
            (NEG_X_FACE_POS, 0.5),
            (NEG_Y_FACE_POS, 0.5),
            (NEG_Z_FACE_POS, 1.0),
        ]
            .into_iter()
            .flat_map(|(face_pos, brightness)| face_pos.zip(FACE_TEX)
                .map(|(pos, tex)| Vertex {
                    pos,
                    tex,
                    color: Rgba::new(brightness, brightness, brightness, 1.0),
                    tex_index: 0,
                }))
            .collect();
        let title_pixel_triangles: Vec<Triangle> = (0..FACES_PER_TITLE_PIXEL)
            .flat_map(|face_i| FACE_TRIANGLES
                .map(|tri| tri
                    .map(|vert_i| face_i * VERTS_PER_FACE + vert_i)))
            .collect();
        let title_pixel_mesh = Arc::new(Mesh {
            vertices: renderer.create_gpu_vec_init(&title_pixel_vertices),
            triangles: renderer.create_gpu_vec_init(&title_pixel_triangles),
        });

        let title_pixel_texture = renderer
            .load_image_array_raw(
                Extent2::new(16, 16), // TODO should be, like, dynamic
                [raw_title_pixel_texture],
            );

        let title_pixel_positions = TITLE_PIXELS
            .iter()
            .enumerate()
            .flat_map(|(r, &column)| column
                .chars()
                .enumerate()
                .filter(|&(_, c)| c == '█')
                .map(move |(c, _)| (r, c)))
            .map(|(r, c)| Vec3 {
                x: c as f32,
                y: (TITLE_PIXELS.len() - r - 1) as f32,
                //z: 0.0, // TODO
                z: rng.gen_range(-75.0..=-40.0),
            })
            .collect();

        McTitleGuiBlock {
            title_cam_distance: -45.0 / 2.5,
            title_cam_height: -10.0 / 6.0 * 0.0,
            title_cam_fov: 1.38753,
            title_angle: 0.48869 / 2.0,
            title_translate: [
                -(TITLE_PIXELS[0].chars().count() as f32) / 2.0,
                -(TITLE_PIXELS.len() as f32) / 2.0,
                0.0,
            ].into(),

            title_pixel_mesh,
            title_pixel_texture,
            title_pixel_positions,
        }
    }

    pub fn update(&mut self, elapsed: f32) {
        for pos in &mut self.title_pixel_positions {
            pos.z = f32::min(0.0, pos.z + 75.0 * elapsed);
        }
    }
}

impl<'a> GuiBlock<'a, DimParentSets, DimParentSets> for &'a McTitleGuiBlock {
    type Sized = McTitleSizedGuiBlock<'a>;

    fn size(self, w: f32, h: f32, scale: f32) -> ((), (), Self::Sized) {
        let sized = McTitleSizedGuiBlock {
            block: self,
            size: Extent2 { w, h },
            scale,
        };
        ((), (), sized)
    }
}

pub struct McTitleSizedGuiBlock<'a> {
    block: &'a McTitleGuiBlock,
    size: Extent2<f32>,
    scale: f32,
}

impl<'a> GuiNode<'a> for McTitleSizedGuiBlock<'a> {
    fn draw(self, _: &Renderer, mut canvas: Canvas2<'a, '_>) {
        /*canvas.reborrow()
            .color([1.0, 0.0, 0.0, 0.1])
            .draw_solid(self.size);*/
        let mut canvas = canvas.reborrow()
            .begin_3d_perspective(
                self.size,
                [0.0, self.block.title_cam_height, self.block.title_cam_distance],
                Quaternion::identity(),
                self.block.title_cam_fov, // TODO horizontal field of view
            )
            .rotate(Quaternion::rotation_x(self.block.title_angle))
            .translate(self.block.title_translate);
        for &pos in &self.block.title_pixel_positions {
            canvas.reborrow()
                .translate(pos)
                .draw_mesh(&self.block.title_pixel_mesh, &self.block.title_pixel_texture);
        }
    }
}*/