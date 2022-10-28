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
    }