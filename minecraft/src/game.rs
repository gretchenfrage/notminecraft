
use crate::{
    jar_assets::JarReader,
/*    ui::{
        UiSize,
        UiModify,
        Margins,
        UiPosInputEvent,
        text::{
            UiText,
            UiTextConfig,
            UiTextBlock,
            UiTextBlockConfig,
        },
        tile_9::Tile9PxRanges,
        menu_button::{
            UiMenuButton,
            UiMenuButtonConfig,
        },
        /*
        v_stack::{
            UiVStack,
            UiVStackConfig,
        },
        h_center::{
            UiHCenter,
            UiHCenterConfig,
        },
        v_center::{
            UiVCenter,
            UiVCenterConfig,
        },*/
    },*/
    ui::{
        ui_block_items_struct,
        UiBlock,
        UiBlockSetWidth,
        UiBlockSetHeight,
        center_block::{
            UiHCenterBlock,
            UiVCenterBlock,
        },
        layer_block::UiLayerBlock,
        stable_unscaled_size_block::{
            UiStableUnscaledWidthBlock,
            UiStableUnscaledHeightBlock,
        },
        stack_block::UiVStackBlock,
        text_block::{UiTextBlock, UiTextBlockConfig},
        tile_9_block::{
            UiTile9Block,
            UiTile9BlockConfig,
            Tile9Images,
            LoadTile9ImagesConfig,
        },
        tile_block::{
            UiTileBlock,
            UiTileBlockConfig,
        },
        margin_block::{
            UiHMarginBlock,
            UiHMarginBlockConfig,
            UiVMarginBlock,
            UiVMarginBlockConfig,
        },
        mc::{
            title::UiMcTitleBlock,
        },
    },/*type Button =
    UiStableUnscaledHeightBlock<
        UiLayerBlock<(
            UiTile9Block,
            UiTextBlock,
        )>
    >;
type Buttons =
    UiHCenterBlock<
        UiStableUnscaledWidthBlock<
            UiVStackBlock<(
                Button,
                Button,
                Button,
                Button,
            )>
        >
    >;*/
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
use image::DynamicImage;


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
    size: Extent2<f32>,
    scale: f32,
    renderer: Renderer,
    jar: JarReader,

    //menu_background: GpuImage,
    font: FontId,
    //title_pixel: Mesh,
    //title_pixel_texture: GpuImageArray,
    
    //title_cam_distance: f32,
    //title_cam_height: f32,
    //title_angle: f32,
    //title_cam_fov: f32,

    rng: Pcg64Mcg,
    //version_text: UiTextBlock,
    //copyright_text: UiTextBlock,
    //title_pixel_positions: Vec<Vec3<f32>>,
    //splash_text: UiText,
    //splash_size: Cosine,

    ui: MainMenu,

    debug_dot: Option<Vec2<f32>>,
}
/*
type MainMenu = UiLayerBlock<MainMenuItems>;

struct MainMenuItems {
    background: UiTileBlock,
    version_text: UiVMarginBlock<UiHMarginBlock<UiTextBlock>>,
    copyright_text: UiVMarginBlock<UiHMarginBlock<UiTextBlock>>,
    buttons: Buttons,
}

ui_block_items_struct!(
    settable_width=true,
    settable_height=true,
    MainMenuItems {
        background: UiTileBlock,
        version_text: UiVMarginBlock<UiHMarginBlock<UiTextBlock>>,
        copyright_text: UiVMarginBlock<UiHMarginBlock<UiTextBlock>>,
        buttons: Buttons,
    }    
);

type Buttons =
    UiHCenterBlock<
        UiStableUnscaledWidthBlock<
            UiVStackBlock<ButtonsItems>
        >
    >;

struct ButtonsItems {
    singleplayer_button: Button,
    multiplayer_button: Button,
    mods_button: Button,
    options_button: Button,
}

ui_block_items_struct!(
    settable_width=true,
    settable_height=false,
    ButtonsItems {
        singleplayer_button: Button,
        multiplayer_button: Button,
        mods_button: Button,
        options_button: Button,
    }
);*/

type MainMenu =
    UiLayerBlock<(
        UiTileBlock, // background
        UiHMarginBlock< // corner text
            UiVMarginBlock<
                UiLayerBlock<(
                    UiTextBlock, // version
                    UiTextBlock, // uncopyright
                )>,
            >,
        >,
        UiVCenterBlock< // center column
            UiVStackBlock<(
                UiHCenterBlock< // title block
                    UiStableUnscaledWidthBlock<
                        UiStableUnscaledHeightBlock<
                            UiMcTitleBlock,
                        >,
                    >,
                >,
                UiHCenterBlock< // buttons
                    UiStableUnscaledWidthBlock<
                        UiVStackBlock<(
                            Button, // singleplayer
                            Button, // multiplayer
                            Button, // mods
                            Button, // options
                        )>,
                    >,
                >,
            )>,
        >,
    )>;


type Button =
    UiStableUnscaledHeightBlock<
        UiLayerBlock<(
            UiTile9Block,
            UiTextBlock,
        )>
    >;


/*
const TITLE_PIXELS: &'static [&'static str] = &[
    "█   █ █ █   █ ███ ███ ███ ███ ███ ███",
    "██ ██ █ ██  █ █   █   █ █ █ █ █    █ ",
    "█ █ █ █ █ █ █ ██  █   ██  ███ ██   █ ",
    "█   █ █ █  ██ █   █   █ █ █ █ █    █ ",
    "█   █ █ █   █ ███ ███ █ █ █ █ █    █ ",
];
*/
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

    pub async fn new(
        mut renderer: Renderer,
        size: Extent2<u32>,
        scale: f32,
    ) -> Result<Self> {
        info!("loading");

        let size = size.map(|n| n as f32);

        let mut rng = Pcg64Mcg::new(0xcafef00dd15ea5e5);

        let jar = JarReader::new().await?;

        let lang = jar.read_properties("lang/en_US.lang").await?;

        //let menu_background = renderer.load_image(jar.read("gui/background.png").await?)?;
        let font = renderer.load_font_437(jar.read("font/default.png").await?)?;
        /*
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
        /*
        struct McMenuButtonFactory {
            font: FontId,
            font_size: f32,
            text_color: Rgba<f32>,
            texture: DynamicImage,
            texture_scale: f32,
            tile_9_px_ranges: Tile9PxRanges,
            tile_9_px_ranges_highlight: Tile9PxRanges,
            unscaled_height: f32,
        }

        impl McMenuButtonFactory {
            async fn new(font: FontId, jar: &JarReader) -> Result<Self> {
                Ok(McMenuButtonFactory {
                    font,
                    font_size: 16.0,
                    text_color: hex_color(0xE0E0E0FF),
                    texture: jar.read_image("gui/gui.png").await?,
                    texture_scale: 2.0,
                    tile_9_px_ranges: Tile9PxRanges {
                        start: [0, 66].into(),
                        extent: [200, 20].into(),
                        top: 2,
                        bottom: 3,
                        left: 2,
                        right: 2,
                    },
                    tile_9_px_ranges_highlight: Tile9PxRanges {
                        start: [0, 86].into(),
                        extent: [200, 20].into(),
                        top: 2,
                        bottom: 3,
                        left: 2,
                        right: 2,
                    },
                    unscaled_height: 40.0,
                })
            }

            fn create(
                &self,
                renderer: &Renderer,
                text: String,
                width: f32,
                scale: f32,
            ) -> UiMenuButton {
                UiMenuButton::new(
                    renderer,
                    UiMenuButtonConfig {
                        text,
                        font: self.font,
                        font_size: self.font_size,
                        text_color: self.text_color,
                        texture: self.texture.clone(),
                        texture_scale: self.texture_scale,
                        tile_9_px_ranges: self.tile_9_px_ranges,
                        tile_9_px_ranges_highlight: self.tile_9_px_ranges_highlight,
                        unscaled_height: self.unscaled_height,
                    },
                    width,
                    scale,
                )
            }
        }

        let menu_button_factory = McMenuButtonFactory::new(font, &jar).await?;
        
        let buttons = UiVCenter::new(
            UiVCenterConfig {
                create_inner: |scale| UiHCenter::new(
                    UiHCenterConfig {
                        create_inner: |width, scale| UiVStack::new(
                            UiVStackConfig {
                                create_items: |width, scale| ButtonsItems {
                                    singleplayer_button: menu_button_factory
                                        .create(
                                            &renderer,
                                            lang["menu.singleplayer"].clone(),
                                            width,
                                            scale,
                                        ),
                                    multiplayer_button: menu_button_factory
                                        .create(
                                            &renderer,
                                            lang["menu.multiplayer"].clone(),
                                            width,
                                            scale,
                                        ),
                                    mods_button: menu_button_factory
                                        .create(
                                            &renderer,
                                            lang["menu.mods"].clone(),
                                            width,
                                            scale,
                                        ),
                                    options_button: menu_button_factory
                                        .create(
                                            &renderer,
                                            lang["menu.options"].clone(),
                                            width,
                                            scale,
                                        ),
                                },
                                unscaled_gap: 8.0,
                                num_items: 4,
                                get_item_height: |items: &ButtonsItems, i| match i {
                                    0 => items.singleplayer_button.size().size.h,
                                    1 => items.multiplayer_button.size().size.h,
                                    2 => items.mods_button.size().size.h,
                                    3 => items.options_button.size().size.h,
                                    _ => unreachable!(),
                                },
                            },
                            width,
                            scale,
                        ),
                        unscaled_inner_width: 400.0,
                    },
                    size.size.w,
                    size.scale,
                ),
                get_inner_height: |inner: &UiHCenter<UiVStack<ButtonsItems>>| inner.inner.size().size.h,
                fraction_down: 0.6,
            },
            size.size.h,
            size.scale,
        );
    */*/

    /*type Buttons =
    UiHCenterBlock<
        UiStableUnscaledWidthBlock<
            UiVStackBlock<(
                Button,
                Button,
                Button,
                Button,
            )>
        >
    >;

type Button =
    UiStableUnscaledHeightBlock<
        UiLayerBlock<(
            UiTile9Block,
            UiTextBlock,
        )>
    >;*/
        let raw_title_pixel_texture = jar.read_image_part("terrain.png", [16, 0], [16, 16]).await?;

        let button_raw_image = jar.read_image("gui/gui.png").await?;
        let button_images = LoadTile9ImagesConfig {
            raw_image: button_raw_image,
            px_start: [0, 66].into(),
            px_extent: [200, 20].into(),
            px_top: 2,
            px_bottom: 3,
            px_left: 2,
            px_right: 2,
        }.load(&renderer);

        fn create_button(
            width: f32,
            scale: f32,
            text: String,
            renderer: &Renderer,
            images: Tile9Images,
            font: FontId,
        ) -> Button {
            UiStableUnscaledHeightBlock::new(
                40.0,
                |size, scale| UiLayerBlock::new(
                    |size, scale| (
                        UiTile9Block::new(
                            UiTile9BlockConfig {
                                images,
                                size_unscaled_untiled: Extent2::new(200.0, 20.0) * 2.0,
                                frac_top: 2.0 / 20.0,
                                frac_bottom: 3.0 / 20.0,
                                frac_left: 2.0 / 200.0,
                                frac_right: 2.0 / 200.0,
                            },
                            size,
                            scale,
                        ),
                        UiTextBlock::new(
                            &renderer,
                            UiTextBlockConfig {
                                text,
                                font,
                                font_size: 16.0,
                                color: hex_color(0xE0E0E0FF),
                                h_align: HAlign::Center,
                                v_align: VAlign::Center,
                                wrap: false,
                            },
                            size,
                            scale,
                        ),
                    ),
                    size,
                    scale,
                ),
                width,
                scale,
            )
        }

        let bg_image = renderer.load_image(jar.read("gui/background.png").await?)?;
        /*
        type MainMenu =
    UiLayerBlock<(
        UiTileBlock, // background
        UiVMarginBlock< // corner text
            UiHMarginBlock<
                UiLayerBlock<(
                    UiTextBlock, // version
                    UiTextBlock, // uncopyright
                )>,
            >,
        >,
        UiVStackBlock<( // center column
            UiHCenterBlock< // title block
                UiStableUnscaledWidthBlock<
                    UiStableUnscaledHeightBlock<
                        UiMcTitleBlock,
                    >,
                >,
            >,
            UiHCenterBlock< // buttons
                UiStableUnscaledWidthBlock<
                    UiVStackBlock<(
                        Button, // singleplayer
                        Button, // multiplayer
                        Button, // mods
                        Button, // options
                    )>,
                >,
            >,
        )>,
    )>;


type Button =
    UiStableUnscaledHeightBlock<
        UiLayerBlock<(
            UiTile9Block,
            UiTextBlock,
        )>
    >;*/
        let ui = UiLayerBlock::new(
            |size, scale| (
                UiTileBlock::new(
                    UiTileBlockConfig {
                        image: bg_image,
                        size_unscaled_untiled: [64.0; 2].into(),
                        color: [0.25, 0.25, 0.25, 1.0].into(),
                    },
                    size,
                    scale,
                ),
                UiHMarginBlock::new(
                    UiHMarginBlockConfig {
                        margin_left: 4.0,
                        margin_right: 4.0,
                    },
                    |size, scale| UiVMarginBlock::new(
                        UiVMarginBlockConfig {
                            margin_top: 4.0,
                            margin_bottom: 4.0,
                        },
                        |size, scale| UiLayerBlock::new(
                            |size, scale| (
                                UiTextBlock::new(
                                    &renderer,
                                    UiTextBlockConfig {
                                        text: "Not Minecraft Beta 1.0.2".into(),
                                        font,
                                        font_size: 16.0,
                                        color: hex_color(0x505050FF),
                                        h_align: HAlign::Left,
                                        v_align: VAlign::Top,
                                        wrap: true,
                                    },
                                    size,
                                    scale,
                                ),
                                UiTextBlock::new(
                                    &renderer,
                                    UiTextBlockConfig {
                                        text: "Everything in the universe is in the public domain.".into(),
                                        font,
                                        font_size: 16.0,
                                        color: Rgba::white(),
                                        h_align: HAlign::Right,
                                        v_align: VAlign::Bottom,
                                        wrap: true,
                                    },
                                    size,
                                    scale,
                                ),
                            ),
                            size,
                            scale,
                        ),
                        size,
                        scale,
                    ),
                    size,
                    scale,
                ),
                UiVCenterBlock::new(
                    |scale| UiVStackBlock::new(
                        25.0,
                        |width, scale| (
                            UiHCenterBlock::new(
                                |scale| UiStableUnscaledWidthBlock::new(
                                    600.0,
                                    |size, scale| UiStableUnscaledHeightBlock::new(
                                        300.0,
                                        |size, scale| UiMcTitleBlock::new(
                                            &renderer,
                                            &mut rng,
                                            raw_title_pixel_texture,
                                            size,
                                            scale,
                                        ),
                                        size.w,
                                        scale,
                                    ),
                                    width,
                                    scale,
                                ),
                                size,
                                scale,
                            ),
                            UiHCenterBlock::new(
                                |scale| UiStableUnscaledWidthBlock::new(
                                    400.0,
                                    |size, scale| UiVStackBlock::new(
                                        8.0,
                                        |width, scale| (
                                            create_button(
                                                width,
                                                scale,
                                                lang["menu.singleplayer"].clone(),
                                                &renderer,
                                                button_images.clone(),
                                                font,
                                            ),
                                            create_button(
                                                width,
                                                scale,
                                                lang["menu.multiplayer"].clone(),
                                                &renderer,
                                                button_images.clone(),
                                                font,
                                            ),
                                            create_button(
                                                width,
                                                scale,
                                                lang["menu.mods"].clone(),
                                                &renderer,
                                                button_images.clone(),
                                                font,
                                            ),
                                            create_button(
                                                width,
                                                scale,
                                                lang["menu.options"].clone(),
                                                &renderer,
                                                button_images.clone(),
                                                font,
                                            ),
                                        ),
                                        size.w,
                                        scale,
                                    ),
                                    width,
                                    scale,
                                ),
                                size,
                                scale,
                            ),
                        ),
                        size.w,
                        scale,
                    ),
                    size,
                    scale,
                ),
            ),
            size,
            scale,
        );/*
        let ui = UiLayerBlock::new(
            |size, scale| MainMenuItems {
                background: UiTileBlock::new(
                    UiTileBlockConfig {
                        image: bg_image,
                        size_unscaled_untiled: [64.0; 2].into(),
                        color: [0.25, 0.25, 0.25, 1.0].into(),
                    },
                    size,
                    scale,
                ),
                version_text: UiVMarginBlock::new(
                    UiVMarginBlockConfig {
                        margin_top: 4.0,
                        margin_bottom: 4.0,
                    },
                    |size, scale| UiHMarginBlock::new(
                        UiHMarginBlockConfig {
                            margin_left: 4.0,
                            margin_right: 4.0,
                        },
                        |size, scale| UiTextBlock::new(
                            &renderer,
                            UiTextBlockConfig {
                                text: "Not Minecraft Beta 1.0.2".into(),
                                font,
                                font_size: 16.0,
                                color: hex_color(0x505050FF),
                                h_align: HAlign::Left,
                                v_align: VAlign::Top,
                                wrap: true,
                            },
                            size,
                            scale,
                        ),
                        size,
                        scale,
                    ),
                    size,
                    scale,
                ),
                copyright_text: UiVMarginBlock::new(
                    UiVMarginBlockConfig {
                        margin_top: 4.0,
                        margin_bottom: 4.0,
                    },
                    |size, scale| UiHMarginBlock::new(
                        UiHMarginBlockConfig {
                            margin_left: 4.0,
                            margin_right: 4.0,
                        },
                        |size, scale| UiTextBlock::new(
                            &renderer,
                            UiTextBlockConfig {
                                text: "Everything in the universe is in the public domain.".into(),
                                font,
                                font_size: 16.0,
                                color: Rgba::white(),
                                h_align: HAlign::Right,
                                v_align: VAlign::Bottom,
                                wrap: true,
                            },
                            size,
                            scale,
                        ),
                        size,
                        scale,
                    ),
                    size,
                    scale,
                ),
                buttons: UiHCenterBlock::new(
                    |scale| UiStableUnscaledWidthBlock::new(
                        400.0,
                        |size, scale| UiVStackBlock::new(
                            8.0,
                            |width, scale| ButtonsItems {
                                singleplayer_button: create_button(
                                    width,
                                    scale,
                                    lang["menu.singleplayer"].clone(),
                                    &renderer,
                                    button_images.clone(),
                                    font,
                                ),
                                multiplayer_button: create_button(
                                    width,
                                    scale,
                                    lang["menu.multiplayer"].clone(),
                                    &renderer,
                                    button_images.clone(),
                                    font,
                                ),
                                mods_button: create_button(
                                    width,
                                    scale,
                                    lang["menu.mods"].clone(),
                                    &renderer,
                                    button_images.clone(),
                                    font,
                                ),
                                options_button: create_button(
                                    width,
                                    scale,
                                    lang["menu.options"].clone(),
                                    &renderer,
                                    button_images.clone(),
                                    font,
                                ),
                            },
                            size.w,
                            scale,
                        ),
                        123456789.0, // TODO
                        scale,
                    ),
                    size,
                    scale,
                ),
            },
            size,
            scale,
        );
    */
        Ok(Game {
            size,
            scale,
            renderer,
            jar,

            //menu_background,
            font,
            //title_pixel,
            //title_pixel_texture,

            //title_cam_distance: -45.0,
            //title_cam_height: -10.0,
            //title_angle: 0.48869,
            //title_cam_fov: 1.38753,

            rng,
            //version_text,
            //copyright_text,
            //title_pixel_positions,
            //splash_text,
            //splash_size: Cosine::new(1.0 / 2.0),

            ui,

            debug_dot: None,
        })
    }

    pub async fn draw<'a>(&mut self, elapsed: f32) -> Result<()> {
        trace!(%elapsed, "updating");
        /*
        self.splash_size.add_to_input(elapsed);
        for pos in &mut self.title_pixel_positions {
            pos.z = f32::min(0.0, pos.z + 75.0 * elapsed);
        }
        */
        trace!("drawing");

        let mut frame = FrameContent::new();
        let mut canvas = frame.canvas();

        self.ui.draw(canvas.reborrow());

        /*
        canvas.reborrow()
            .color([0.25, 0.25, 0.25, 1.0])
            .draw_image_uv(
                &self.menu_background,
                self.size.size,
                [0.0, 0.0],
                self.size.size / (64.0 * self.size.scale),
            );

        self.buttons
            .draw(
                canvas.reborrow(),
                |inner, canvas| inner
                    .draw(
                        canvas,
                        |inner, canvas| inner
                            .draw(
                                canvas,
                                |items, i, canvas| match i {
                                    0 => items.singleplayer_button.draw(canvas),
                                    1 => items.multiplayer_button.draw(canvas),
                                    2 => items.mods_button.draw(canvas),
                                    3 => items.options_button.draw(canvas),
                                    _ => unreachable!(),
                                },
                            ),
                    )
            );

        {
            let mut canvas = canvas.reborrow()
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
                ]);
            for &pos in &self.title_pixel_positions {
                canvas.reborrow()
                    .translate(pos)
                    .draw_mesh(&self.title_pixel, &self.title_pixel_texture);
            }
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
        
        self.version_text.draw(canvas.reborrow());
        self.copyright_text.draw(canvas.reborrow());

        if let Some(pos) = self.debug_dot {
            let dot_size = Extent2 { w: 10.0, h: 10.0 };
            canvas.reborrow()
                .translate(pos)
                .translate(-dot_size / 2.0)
                .color(Rgba::red())
                .draw_solid(dot_size);
        }
        */
        self.renderer.draw_frame(&frame)
    }

    pub async fn set_size(&mut self, size: Extent2<u32>) -> Result<()> {
        info!(?size, "setting size");

        self.renderer.resize(size);
        self.size = size.map(|n| n as f32);

        self.ui.set_width(&self.renderer, self.size.w);
        self.ui.set_height(&self.renderer, self.size.h);
        /*
        self.version_text.set_size(&self.renderer, self.size.size);
        self.copyright_text.set_size(&self.renderer, self.size.size);
        
        self.buttons.set_height(self.size.size.h);
        self.buttons.inner.set_width(self.size.size.w);
        */
        Ok(())
    }

    pub async fn set_scale(&mut self, scale: f32) -> Result<()> {
        info!(?scale, "setting scale");

        self.scale = scale;

        self.ui.set_scale(&self.renderer, self.scale);
        /*
        self.version_text.set_scale(&self.renderer, self.size.scale);
        self.copyright_text.set_scale(&self.renderer, self.size.scale);
        self.splash_text.set_scale(&self.renderer, self.size.scale);

        self.buttons
            .set_scale(
                self.size.scale,
                |inner, scale| inner
                    .set_scale(
                        scale,
                        |inner, scale| inner
                            .set_scale(
                                scale,
                                |items, i, scale| match i {
                                    0 => items.singleplayer_button
                                        .set_scale(
                                            &self.renderer,
                                            scale,
                                        ),
                                    1 => items.multiplayer_button
                                        .set_scale(
                                            &self.renderer,
                                            scale,
                                        ),
                                    2 => items.mods_button
                                        .set_scale(
                                            &self.renderer,
                                            scale,
                                        ),
                                    3 => items.options_button
                                        .set_scale(
                                            &self.renderer,
                                            scale,
                                        ),
                                    _ => unreachable!(),
                                },
                                |items, i| match i {
                                    0 => items.singleplayer_button.size().size.h,
                                    1 => items.multiplayer_button.size().size.h,
                                    2 => items.mods_button.size().size.h,
                                    3 => items.options_button.size().size.h,
                                    _ => unreachable!(),
                                }
                            ),
                        |inner, width| inner
                            .set_width(
                                width,
                                |items, i, width| match i {
                                    0 => items.singleplayer_button
                                        .set_width(
                                            &self.renderer,
                                            width,
                                        ),
                                    1 => items.multiplayer_button
                                        .set_width(
                                            &self.renderer,
                                            width,
                                        ),
                                    2 => items.mods_button
                                        .set_width(
                                            &self.renderer,
                                            width,
                                        ),
                                    3 => items.options_button
                                        .set_width(
                                            &self.renderer,
                                            width,
                                        ),
                                    _ => unreachable!(),
                                },
                            ),
                    ),
                |inner| inner.inner.size().size.h
            );
        */
        Ok(())
    }

    pub async fn keyboard_input(&mut self, input: KeyboardInput) -> Result<()> {
        Ok(())
    }

    pub async fn mouse_wheel(&mut self, delta: MouseScrollDelta) -> Result<()> {
        let n = match delta {
            MouseScrollDelta::LineDelta(_, y) => y,
            MouseScrollDelta::PixelDelta(delta) => delta.y as f32 / (16.0 * self.scale),
        };
        self.set_scale(self.scale * (1.0 + n / 100.0)).await?;
        Ok(())
    }
/*
    pub async fn on_pos_input_event(&mut self, event: UiPosInputEvent) -> Result<()> {
        //debug!(?event);
        /*
        self.buttons
            .on_pos_input_event(
                event,
                |inner, event| inner
                    .on_pos_input_event(
                        event,
                        |inner, event| inner
                            .on_pos_input_event(
                                event,
                                |items, i, event| match i {
                                    0 => items.singleplayer_button.on_pos_input_event(event),
                                    1 => items.multiplayer_button.on_pos_input_event(event),
                                    2 => items.mods_button.on_pos_input_event(event),
                                    3 => items.options_button.on_pos_input_event(event),
                                    _ => unreachable!(),
                                }
                            )
                    )
            );*/
        Ok(())
    }*/
}
