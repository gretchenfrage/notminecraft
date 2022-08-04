
use crate::jar_assets::JarReader;
use graphics::{
    Renderer,
    frame_content::{
        FrameContent,
        Canvas2,
        GpuImage,
        FontId,
        TextBlock,
        TextSpan,
        HorizontalAlign,
        VerticalAlign,
        LayedOutTextBlock,
        GpuImageArray,
        Mesh,
        Vertex,
        Triangle,
    },
};
use vek::*;
use anyhow::*;


pub fn hex_color(hex: u32) -> Rgba<f32> {
    Rgba {
        r: ((hex & 0xFF000000) >> 24) as f32 / 255.0,
        g: ((hex & 0x00FF0000) >> 16) as f32 / 255.0,
        b: ((hex & 0x0000FF00) >> 8) as f32 / 255.0,
        a: (hex & 0x000000FF) as f32 / 255.0,
    }
}


#[derive(Debug, Copy, Clone, PartialEq)]
pub struct UiSize {
    pub size: Extent2<f32>,
    pub scale: f32,
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

    version_text: LayedOutTextBlock,
    copyright_text: LayedOutTextBlock,
    title_pixel_positions: Vec<Vec3<f32>>,
}

fn lay_out_version_text(
    renderer: &Renderer,
    font: FontId,
    size: UiSize,
) -> LayedOutTextBlock
{
    renderer
        .lay_out_text(&TextBlock {
            spans: &[
                TextSpan {
                    text: "Not Minecraft Beta 1.0.2",
                    font_id: font,
                    font_size: 16.0 * size.scale,
                    color: hex_color(0x505050FF),
                },
            ],
            horizontal_align: HorizontalAlign::Left { width: Some(size.size.w) },
            vertical_align: VerticalAlign::Top,
        })
}

fn lay_out_copyright_text(
    renderer: &Renderer,
    font: FontId,
    size: UiSize,
) -> LayedOutTextBlock
{
    renderer
        .lay_out_text(&TextBlock {
            spans: &[
                TextSpan {
                    text: "Everything in the universe is in the public domain.",
                    font_id: font,
                    font_size: 16.0 * size.scale,
                    color: Rgba::white(),
                },
            ],
            horizontal_align: HorizontalAlign::Right { width: size.size.w },
            vertical_align: VerticalAlign::Bottom { height: size.size.h },
        })
}

fn draw_text_with_shadow(
    mut canvas: Canvas2,
    text: &LayedOutTextBlock,
)
{
    canvas.reborrow()
        .translate([2.0, 2.0])
        .color([0.25, 0.25, 0.25, 1.0])
        .draw_text(&text);
    canvas.reborrow()
        .draw_text(&text);
}

const TITLE_PIXELS: &'static [&'static str] = &[
    "█   █ █ █   █ ███ ███ ███ ███ ███ ███",
    "██ ██ █ ██  █ █   █   █ █ █ █ █    █ ",
    "█ █ █ █ █ █ █ ██  █   ██  ███ ██   █ ",
    "█   █ █ █  ██ █   █   █ █ █ █ █    █ ",
    "█   █ █ █   █ ███ ███ █ █ █ █ █    █ ",
];

impl Game {
    pub async fn new(mut renderer: Renderer, size: UiSize) -> Result<Self> {
        info!("loading");
        let jar = JarReader::new().await?;

        let menu_background = renderer.load_image(jar.read("gui/background.png").await?)?;
        let font = renderer.load_font_437(jar.read("font/default.png").await?)?;

        const FACES_PER_BLOCK: usize = 6;
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
            pos_x_face_pos,
            pos_y_face_pos,
            pos_z_face_pos,
            neg_x_face_pos,
            neg_y_face_pos,
            neg_z_face_pos,
        ]
            .into_iter()
            .flat_map(|face_pos| face_pos.zip(face_tex))
            .map(|(pos, tex)| Vertex {
                pos,
                tex,
                color: Rgba::white(),
                tex_index: 0,
            })
            .collect::<Vec<_>>();

        let title_pixel_triangles = (0..FACES_PER_BLOCK)
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

        let version_text = lay_out_version_text(&renderer, font, size);
        let copyright_text = lay_out_copyright_text(&renderer, font, size);

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
                z: 0.0,
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

            version_text,
            copyright_text,
            title_pixel_positions,
        })
    }

    pub async fn draw<'a>(&mut self) -> Result<()> {
        trace!("drawing");
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
        draw_text_with_shadow(
            canvas.reborrow()
                .translate([4.0, 4.0]),
            &self.version_text,
        );
        draw_text_with_shadow(
            canvas.reborrow()
                .translate(self.size.size)
                .translate([-2.0, 0.0]),
            &self.copyright_text,
        );

        let mut title_canvas = canvas.reborrow()
            .begin_3d_perspective(
                self.size.size,
                [0.0, 0.0, -20.0],
                Quaternion::identity(),
                f32::to_radians(75.0),
            )
            .translate([
                -(TITLE_PIXELS[0].chars().count() as f32) / 2.0,
                -(TITLE_PIXELS.len() as f32) / 2.0,
                0.0,
            ]);
        for &pos in &self.title_pixel_positions {
            title_canvas.reborrow()
                .translate(pos)
                .draw_mesh(&self.title_pixel, &self.title_pixel_texture);
        }

        debug!(?frame);
        self.renderer.draw_frame(&frame)
    }

    pub async fn set_size(&mut self, size: Extent2<u32>) -> Result<()> {
        info!(?size, "setting size");

        self.renderer.resize(size);
        self.size.size = size.map(|n| n as f32);

        self.version_text = lay_out_version_text(&self.renderer, self.font, self.size);
        self.copyright_text = lay_out_copyright_text(&self.renderer, self.font, self.size);

        Ok(())
    }

    pub async fn set_scale(&mut self, scale: f32) -> Result<()> {
        info!(?scale, "setting scale");

        self.size.scale = scale;

        self.version_text = lay_out_version_text(&self.renderer, self.font, self.size);
        self.copyright_text = lay_out_copyright_text(&self.renderer, self.font, self.size);

        Ok(())
    }
}
