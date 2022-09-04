
use crate::{
    jar_assets::JarReader,
    /*
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
    },*/
    ui2::*,
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
        FrameItem,
    },
    modifier::Modifier2,
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

    font: FontId,

    rng: Pcg64Mcg,
    //splash_text: UiText,
    //splash_size: Cosine,

    //ui: MainMenu,
    bg_image: GpuImage,

    button_images: Tile9Images,

    version_text: TextGuiBlock,
    copyright_text: TextGuiBlock,
    singleplayer_button_text: TextGuiBlock,
    multiplayer_button_text: TextGuiBlock,
    mods_button_text: TextGuiBlock,
    options_button_text: TextGuiBlock,

    debug_dot: Option<Vec2<f32>>,
}
/*
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

        let font = renderer.load_font_437(jar.read("font/default.png").await?)?;
        
        /*
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
        */

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

        // version_text: TextGuiBlock,
        // copyright_text: TextGuiBlock,
        // singleplayer_button_text: TextGuiBlock,
        // multiplayer_button_text: TextGuiBlock,
        // mods_button_text: TextGuiBlock,
        // options_button_text: TextGuiBlock,
        let version_text = TextGuiBlock::new(
            vec![TextGuiBlockSpan {
                text: "Not Minecraft Beta 1.0.2".into(),
                font,
                color: hex_color(0x505050FF),
            }],
            16.0,
            HAlign::Left,
            VAlign::Top,
            true,
        );
        let copyright_text = TextGuiBlock::new(
            vec![TextGuiBlockSpan {
                text: "Everything in the universe is in the public domain.".into(),
                font,
                color: Rgba::white(),
            }],
            16.0,
            HAlign::Right,
            VAlign::Bottom,
            true,
        );

        let singleplayer_button_text = TextGuiBlock::new(
            vec![TextGuiBlockSpan {
                text: lang["menu.singleplayer"].clone(),
                font,
                color: hex_color(0xE0E0E0FF),
            }],
            16.0,
            HAlign::Center,
            VAlign::Center,
            false,
        );
        let multiplayer_button_text = TextGuiBlock::new(
            vec![TextGuiBlockSpan {
                text: lang["menu.multiplayer"].clone(),
                font,
                color: hex_color(0xE0E0E0FF),
            }],
            16.0,
            HAlign::Center,
            VAlign::Center,
            false,
        );
        let mods_button_text = TextGuiBlock::new(
            vec![TextGuiBlockSpan {
                text: lang["menu.mods"].clone(),
                font,
                color: hex_color(0xE0E0E0FF),
            }],
            16.0,
            HAlign::Center,
            VAlign::Center,
            false,
        );
        let options_button_text = TextGuiBlock::new(
            vec![TextGuiBlockSpan {
                text: lang["menu.options"].clone(),
                font,
                color: hex_color(0xE0E0E0FF),
            }],
            16.0,
            HAlign::Center,
            VAlign::Center,
            false,
        );

/*
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
        }*/

        let bg_image = renderer.load_image(jar.read("gui/background.png").await?)?;
        /*
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
        );*/

        Ok(Game {
            size,
            scale,
            renderer,
            jar,

            font,

            rng,

            bg_image,

            button_images,

            version_text,
            copyright_text,
            singleplayer_button_text,
            multiplayer_button_text,
            mods_button_text,
            options_button_text,

            //splash_text,
            //splash_size: Cosine::new(1.0 / 2.0),

            //ui,

            debug_dot: None,
        })
    }
    /*
    fn gui<'a>(&'a self) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
        tile_image_gui_block(
            &self.bg_image,
            [64.0; 2],
        )
    }
    */
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
        //let mut canvas = frame.canvas();

        //self.ui.draw(canvas.reborrow());

        struct DrawGuiVisitorTarget<'r, 'a, 'b> {
            renderer: &'r Renderer,
            frame_content: &'b mut FrameContent<'a>,
        }

        impl<'r, 'a, 'b> GuiVisitorTarget<'a> for DrawGuiVisitorTarget<'r, 'a, 'b> {
            fn push_modifier(&mut self, stack_len: usize, modifier: Modifier2) {
                self.frame_content.0.push((stack_len, FrameItem::PushModifier2(modifier)));
            }

            fn visit_node<I: GuiNode<'a>>(&mut self, stack_len: usize, mut node: I) {
                node.draw(self.renderer, Canvas2 {
                    target: self.frame_content,
                    stack_len,
                });
            }
        }



        {
            // TODO kinda tricky to factor out because the compiler needs to
            // know that it won't borrow from renderer. but is actually a good
            // refactoring opportunity revealed by that.
            let gui = 
                layer_gui_block((
                    modifier_gui_block(
                        Rgba::new(0.25, 0.25, 0.25, 1.0),
                            tile_image_gui_block(
                            &self.bg_image,
                            [64.0; 2],
                        ),
                    ),
                    
                    h_center_gui_block(
                        0.5,
                        v_center_gui_block(
                            0.0,
                            h_stable_unscaled_dim_size_gui_block(
                                400.0,
                                v_stack_gui_block(
                                    25.0 / 2.0,
                                    (
                                        v_stable_unscaled_dim_size_gui_block(
                                            40.0,
                                            layer_gui_block((
                                                tile_9_gui_block(
                                                    &self.button_images,
                                                    Extent2::new(200.0, 20.0) * 2.0,
                                                    2.0 / 20.0,
                                                    3.0 / 20.0,
                                                    2.0 / 200.0,
                                                    2.0 / 200.0,
                                                ),
                                                &mut self.singleplayer_button_text,
                                            )),
                                        ),
                                        v_stable_unscaled_dim_size_gui_block(
                                            40.0,
                                            layer_gui_block((
                                                tile_9_gui_block(
                                                    &self.button_images,
                                                    Extent2::new(200.0, 20.0) * 2.0,
                                                    2.0 / 20.0,
                                                    3.0 / 20.0,
                                                    2.0 / 200.0,
                                                    2.0 / 200.0,
                                                ),
                                                &mut self.multiplayer_button_text,
                                            )),
                                        ),
                                        v_stable_unscaled_dim_size_gui_block(
                                            40.0,
                                            layer_gui_block((
                                                tile_9_gui_block(
                                                    &self.button_images,
                                                    Extent2::new(200.0, 20.0) * 2.0,
                                                    2.0 / 20.0,
                                                    3.0 / 20.0,
                                                    2.0 / 200.0,
                                                    2.0 / 200.0,
                                                ),
                                                &mut self.mods_button_text,
                                            )),
                                        ),
                                        v_stable_unscaled_dim_size_gui_block(
                                            40.0,
                                            layer_gui_block((
                                                tile_9_gui_block(
                                                    &self.button_images,
                                                    Extent2::new(200.0, 20.0) * 2.0,
                                                    2.0 / 20.0,
                                                    3.0 / 20.0,
                                                    2.0 / 200.0,
                                                    2.0 / 200.0,
                                                ),
                                                &mut self.options_button_text,
                                            )),
                                        ),
                                    ),
                                ),
                            ),
                        ),
                    ),
                    h_margin_gui_block(
                        4.0,
                        4.0,
                        v_margin_gui_block(
                            4.0,
                            4.0,
                            layer_gui_block((
                                &mut self.version_text,
                                &mut self.copyright_text,
                            )),
                        ),
                    ),
                ))
                ;
            let ((), (), sized_gui) = gui.size(self.size.w, self.size.h, self.scale);
            sized_gui.visit_nodes(GuiVisitor::new(&mut DrawGuiVisitorTarget {
                renderer: &self.renderer,
                frame_content: &mut frame,
            }));
        }

        /*
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
        */

        self.renderer.draw_frame(&frame)
    }

    pub async fn set_size(&mut self, size: Extent2<u32>) -> Result<()> {
        info!(?size, "setting size");

        self.renderer.resize(size);
        self.size = size.map(|n| n as f32);

        //self.ui.set_width(&self.renderer, self.size.w);
        //self.ui.set_height(&self.renderer, self.size.h);
        
        Ok(())
    }

    pub async fn set_scale(&mut self, scale: f32) -> Result<()> {
        info!(?scale, "setting scale");

        self.scale = scale;

        //self.ui.set_scale(&self.renderer, self.scale);
        
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
}
