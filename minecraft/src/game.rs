
use crate::{
    jar_assets::JarReader,
    ui::{
        UiSize,
        UiModify,
        Margins,
        text::{
            UiText,
            UiTextConfig,
            UiTextBlock,
            UiTextBlockConfig,
        },
    },
};
use graphics::{
    Renderer,
    frame_content::{
        FrameContent,
        Canvas2,
        GpuImage,
        FontId,
        TextBlock,
        TextSpan,
        HAlign,
        VAlign,
        LayedOutTextBlock,
        GpuImageArray,
        Mesh,
        Vertex,
        Triangle,
    },
};
use winit_main::reexports::{
    window::WindowAttributes,
    event::{
        KeyboardInput,
        VirtualKeyCode,
        MouseScrollDelta,
    },
    dpi::PhysicalSize,
};
use std::f32;
use rand::Rng;
use rand_pcg::Pcg64Mcg;
use vek::*;
use anyhow::*;


#[derive(Debug, Clone)]
pub struct Cosine {
    input: f32,
    period: f32,
}

impl Cosine {
    pub fn new(period: f32) -> Self {
        Cosine {
            input: 0.0,
            period,
        }
    }

    pub fn add_to_input(&mut self, n: f32) {
        self.input += n;
        self.input %= self.period;
    }

    pub fn get(&self) -> f32 {
        (self.input * f32::consts::PI / self.period).cos()
    }
}


pub fn hex_color(hex: u32) -> Rgba<f32> {
    Rgba {
        r: ((hex & 0xFF000000) >> 24) as f32 / 255.0,
        g: ((hex & 0x00FF0000) >> 16) as f32 / 255.0,
        b: ((hex & 0x0000FF00) >> 8) as f32 / 255.0,
        a: (hex & 0x000000FF) as f32 / 255.0,
    }
}


#[allow(dead_code)]
pub struct Game {
    size: UiSize,
    renderer: Renderer,
    jar: JarReader,

    menu_background: GpuImage,
    font: FontId,
    title_pixel: Mesh,
    title_pixel_texture: GpuImageArray,
    
    title_cam_distance: f32,
    title_cam_height: f32,
    title_angle: f32,
    title_cam_fov: f32,

    rng: Pcg64Mcg,
    version_text: UiTextBlock,
    copyright_text: UiTextBlock,
    title_pixel_positions: Vec<Vec3<f32>>,
    splash_text: UiText,
    splash_size: Cosine,
}

const TITLE_PIXELS: &'static [&'static str] = &[
    "█   █ █ █   █ ███ ███ ███ ███ ███ ███",
    "██ ██ █ ██  █ █   █   █ █ █ █ █    █ ",
    "█ █ █ █ █ █ █ ██  █   ██  ███ ██   █ ",
    "█   █ █ █  ██ █   █   █ █ █ █ █    █ ",
    "█   █ █ █   █ ███ ███ █ █ █ █ █    █ ",
];

impl Game {
    pub async fn window_config() -> Result<WindowAttributes> {
        Ok(WindowAttributes {
            title: "Not Minecraft".into(),
            inner_size: Some(PhysicalSize {
                
                width: 1250 / 2 * 4 / 3,
                height: 725 / 2 * 4 / 3,
                
                /*
                width: 849,
                height: 529,*/
            }.into()),
            ..Default::default()
        })
    }

    pub async fn new(mut renderer: Renderer, size: UiSize) -> Result<Self> {
        info!("loading");
        let mut rng = Pcg64Mcg::new(0xcafef00dd15ea5e5);

        let jar = JarReader::new().await?;

        let menu_background = renderer.load_image(jar.read("gui/background.png").await?)?;
        let font = renderer.load_font_437(jar.read("font/default.png").await?)?;

        const FACES_PER_TITLE_PIXEL: usize = 5;
        const VERTS_PER_FACE: usize = 4;

        let pos_x_face_pos: [Vec3<f32>; VERTS_PER_FACE] = [
            [1.0, 1.0, 0.0].into(),
            [1.0, 1.0, 1.0].into(),
            [1.0, 0.0, 1.0].into(),
            [1.0, 0.0, 0.0].into(),
        ];
        let pos_y_face_pos: [Vec3<f32>; VERTS_PER_FACE] = [
            [0.0, 1.0, 1.0].into(),
            [1.0, 1.0, 1.0].into(),
            [1.0, 1.0, 0.0].into(),
            [0.0, 1.0, 0.0].into(),
        ];
        let pos_z_face_pos: [Vec3<f32>; VERTS_PER_FACE] = [
            [1.0, 1.0, 1.0].into(),
            [0.0, 1.0, 1.0].into(),
            [0.0, 0.0, 1.0].into(),
            [1.0, 0.0, 1.0].into(),
        ];
        let neg_x_face_pos: [Vec3<f32>; VERTS_PER_FACE] = [
            [0.0, 1.0, 1.0].into(),
            [0.0, 1.0, 0.0].into(),
            [0.0, 0.0, 0.0].into(),
            [0.0, 0.0, 1.0].into(),
        ];
        let neg_y_face_pos: [Vec3<f32>; VERTS_PER_FACE] = [
            [0.0, 0.0, 0.0].into(),
            [1.0, 0.0, 0.0].into(),
            [1.0, 0.0, 1.0].into(),
            [0.0, 0.0, 1.0].into(),
        ];
        let neg_z_face_pos: [Vec3<f32>; VERTS_PER_FACE] = [
            [0.0, 1.0, 0.0].into(),
            [1.0, 1.0, 0.0].into(),
            [1.0, 0.0, 0.0].into(),
            [0.0, 0.0, 0.0].into(),
        ];

        let face_tex: [Vec2<f32>; 4] = [
            [0.0, 0.0].into(),
            [1.0, 0.0].into(),
            [1.0, 1.0].into(),
            [0.0, 1.0].into(),
        ];

        let face_triangles: [Triangle; 2] = [
            Triangle([0, 1, 2]),
            Triangle([0, 2, 3]),
        ];

        let title_pixel_vertices = [
            (pos_x_face_pos, 0.5),
            (pos_y_face_pos, 0.5),
            (neg_x_face_pos, 0.5),
            (neg_y_face_pos, 0.5),
            (neg_z_face_pos, 1.0),
        ]
            .into_iter()
            .flat_map(|(face_pos, brightness)| face_pos.zip(face_tex)
                .map(|(pos, tex)| Vertex {
                    pos,
                    tex,
                    color: Rgba::new(brightness, brightness, brightness, 1.0),
                    tex_index: 0,
                }))
            .collect::<Vec<_>>();

        let title_pixel_triangles = (0..FACES_PER_TITLE_PIXEL)
            .flat_map(|face_i| face_triangles
                .map(|tri| tri
                    .map(|vert_i| face_i * VERTS_PER_FACE + vert_i)))
            .collect::<Vec<_>>();

        let title_pixel = Mesh {
            vertices: renderer.create_gpu_vec_init(&title_pixel_vertices),
            triangles: renderer.create_gpu_vec_init(&title_pixel_triangles),
        };
        let title_pixel_texture = renderer
            .load_image_array_raw(
                Extent2::new(16, 16),
                [
                    jar.read_image_part("terrain.png", [16, 0], [16, 16]).await?,
                ],
            );

        let main_menu_text_margins = Margins {
            top: 4.0,
            bottom: 4.0,
            left: 4.0,
            right: 4.0,
        };
        let version_text = UiTextBlock::new(
            &renderer,
            UiTextBlockConfig {
                text_config: UiTextConfig {
                    text: "Not Minecraft Beta 1.0.2".into(),
                    font,
                    font_size: 16.0,
                    color: hex_color(0x505050FF),
                    h_align: HAlign::Left,
                    v_align: VAlign::Top,
                },
                margins: main_menu_text_margins,
                wrap: true,
            },
            size,
        );
        let copyright_text = UiTextBlock::new(
            &renderer,
            UiTextBlockConfig {
                text_config: UiTextConfig {
                    text: "Everything in the universe is in the public domain.".into(),
                    font,
                    font_size: 16.0,
                    color: Rgba::white(),
                    h_align: HAlign::Right,
                    v_align: VAlign::Bottom,
                },
                margins: main_menu_text_margins,
                wrap: true,
            },
            size,
        );
        let splash_text = UiText::new(
            &renderer,
            UiTextConfig {
                text: "Splash text!".into(),
                font,
                font_size: 32.0,
                color: Rgba::yellow(),
                h_align: HAlign::Center,
                v_align: VAlign::Center,
            },
            None,
            size.scale,
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
                z: rng.gen_range(-75.0..=-40.0),
            })
            .collect();

        Ok(Game {
            size,
            renderer,
            jar,

            menu_background,
            font,
            title_pixel,
            title_pixel_texture,

            title_cam_distance: -45.0,
            title_cam_height: -10.0,
            title_angle: 0.48869,
            title_cam_fov: 1.38753,

            rng,
            version_text,
            copyright_text,
            title_pixel_positions,
            splash_text,
            splash_size: Cosine::new(1.0 / 2.0),
        })
    }

    pub async fn draw<'a>(&mut self, elapsed: f32) -> Result<()> {
        trace!(%elapsed, "drawing");

        self.splash_size.add_to_input(elapsed);
        for pos in &mut self.title_pixel_positions {
            pos.z = f32::min(0.0, pos.z + 75.0 * elapsed);
        }

        let mut frame = FrameContent::new();
        let mut canvas = frame.canvas();
        canvas.reborrow()
            .color([0.25, 0.25, 0.25, 1.0])
            .draw_image_uv(
                &self.menu_background,
                self.size.size,
                [0.0, 0.0],
                self.size.size / (64.0 * self.size.scale),
            );
        self.version_text.draw(canvas.reborrow());
        self.copyright_text.draw(canvas.reborrow());
        
        let mut title_canvas = canvas.reborrow()
            .begin_3d_perspective(
                self.size.size,
                [0.0, self.title_cam_height, self.title_cam_distance],
                Quaternion::identity(),
                self.title_cam_fov, // TODO horizontal field of view
            )
            .rotate(Quaternion::rotation_x(self.title_angle))
            .translate([
                -(TITLE_PIXELS[0].chars().count() as f32) / 2.0,
                -(TITLE_PIXELS.len() as f32) / 2.0,
                0.0,
            ])
            ;
        for &pos in &self.title_pixel_positions {
            title_canvas.reborrow()
                .translate(pos)
                .draw_mesh(&self.title_pixel, &self.title_pixel_texture);
        }
        {
            let mut canvas = canvas.reborrow()
                .translate(Vec2 {
                    x: self.size.size.w / 4.0 * 3.0,
                    y: self.size.size.h / 16.0 * 5.0,
                })
                .scale([
                    self.splash_size.get().abs() / 16.0 + 1.0; 2
                ])
                .rotate(f32::to_radians(22.5));
            self.splash_text.draw(canvas.reborrow());
        }
        self.renderer.draw_frame(&frame)
    }

    pub async fn set_size(&mut self, size: Extent2<u32>) -> Result<()> {
        info!(?size, "setting size");

        self.renderer.resize(size);
        self.size.size = size.map(|n| n as f32);

        self.version_text.set_size(&self.renderer, self.size.size);
        self.copyright_text.set_size(&self.renderer, self.size.size);

        Ok(())
    }

    pub async fn set_scale(&mut self, scale: f32) -> Result<()> {
        info!(?scale, "setting scale");

        self.size.scale = scale;

        self.version_text.set_scale(&self.renderer, self.size.scale);
        self.copyright_text.set_scale(&self.renderer, self.size.scale);
        self.splash_text.set_scale(&self.renderer, self.size.scale);

        Ok(())
    }

    pub async fn keyboard_input(&mut self, input: KeyboardInput) -> Result<()> {
        Ok(())
    }

    pub async fn mouse_wheel(&mut self, delta: MouseScrollDelta) -> Result<()> {
        let n = match delta {
            MouseScrollDelta::LineDelta(_, y) => y,
            MouseScrollDelta::PixelDelta(delta) => delta.y as f32 / (16.0 * self.size.scale),
        };
        self.set_scale(self.size.scale + n / 100.0).await?;
        Ok(())
    }
}
