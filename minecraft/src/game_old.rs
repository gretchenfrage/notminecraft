
use crate::{
    jar_assets::JarReader,
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
    modifier::{
        Modifier2,
        Transform2,
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

    font: FontId,

    rng: Pcg64Mcg,

    //ui: MainMenu,
    gui_state: GuiState,

    debug_dot: Option<Vec2<f32>>,
}

struct GuiState {
    bg_image: GpuImage,

    title_block: McTitleGuiBlock,

    button_images: Tile9Images,
    button_highlighted_images: Tile9Images,

    version_text: TextGuiBlock,
    copyright_text: TextGuiBlock,

    singleplayer_button_text: TextGuiBlock,
    multiplayer_button_text: TextGuiBlock,
    mods_button_text: TextGuiBlock,
    options_button_text: TextGuiBlock,
    
    singleplayer_button_cursor_is_over_tracker: CursorIsOverTrackerGuiBlock,
    multiplayer_button_cursor_is_over_tracker: CursorIsOverTrackerGuiBlock,
    mods_button_cursor_is_over_tracker: CursorIsOverTrackerGuiBlock,
    options_button_cursor_is_over_tracker: CursorIsOverTrackerGuiBlock,

    splash_text: McSplashTextGuiBlock,
}

impl GuiState {
    fn gui<'a>(&'a mut self) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
        layer_gui_block((
            modifier_gui_block(
                Rgba::new(0.25, 0.25, 0.25, 1.0),
                    tile_image_gui_block(
                    &self.bg_image,
                    [64.0; 2],
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
            v_center_gui_block(
                0.0,
                v_stack_gui_block(
                    0.0,
                    (
                        h_center_gui_block(
                            0.5,
                            h_stable_unscaled_dim_size_gui_block(
                                500.0,
                                v_stable_unscaled_dim_size_gui_block(
                                    200.0,
                                    &self.title_block,
                                ),
                            ),
                        ),
                        h_center_gui_block(
                            0.5,
                            h_stable_unscaled_dim_size_gui_block(
                                400.0,
                                v_stack_gui_block(
                                    25.0 / 2.0,
                                    (
                                        v_stable_unscaled_dim_size_gui_block(
                                            40.0,
                                            layer_gui_block((
                                                tile_9_gui_block(
                                                    match self.singleplayer_button_cursor_is_over_tracker.cursor_is_over {
                                                        false => &self.button_images,
                                                        true => &self.button_highlighted_images,
                                                    },
                                                    Extent2::new(200.0, 20.0) * 2.0,
                                                    2.0 / 20.0,
                                                    3.0 / 20.0,
                                                    2.0 / 200.0,
                                                    2.0 / 200.0,
                                                ),
                                                &mut self.singleplayer_button_text,
                                                &mut self.singleplayer_button_cursor_is_over_tracker,
                                            )),
                                        ),
                                        v_stable_unscaled_dim_size_gui_block(
                                            40.0,
                                            layer_gui_block((
                                                tile_9_gui_block(
                                                    match self.multiplayer_button_cursor_is_over_tracker.cursor_is_over {
                                                        false => &self.button_images,
                                                        true => &self.button_highlighted_images,
                                                    },
                                                    Extent2::new(200.0, 20.0) * 2.0,
                                                    2.0 / 20.0,
                                                    3.0 / 20.0,
                                                    2.0 / 200.0,
                                                    2.0 / 200.0,
                                                ),
                                                &mut self.multiplayer_button_text,
                                                &mut self.multiplayer_button_cursor_is_over_tracker,
                                            )),
                                        ),
                                        v_stable_unscaled_dim_size_gui_block(
                                            40.0,
                                            layer_gui_block((
                                                tile_9_gui_block(
                                                    match self.mods_button_cursor_is_over_tracker.cursor_is_over {
                                                        false => &self.button_images,
                                                        true => &self.button_highlighted_images,
                                                    },
                                                    Extent2::new(200.0, 20.0) * 2.0,
                                                    2.0 / 20.0,
                                                    3.0 / 20.0,
                                                    2.0 / 200.0,
                                                    2.0 / 200.0,
                                                ),
                                                &mut self.mods_button_text,
                                                &mut self.mods_button_cursor_is_over_tracker,
                                            )),
                                        ),
                                        v_stable_unscaled_dim_size_gui_block(
                                            40.0,
                                            layer_gui_block((
                                                tile_9_gui_block(
                                                    match self.options_button_cursor_is_over_tracker.cursor_is_over {
                                                        false => &self.button_images,
                                                        true => &self.button_highlighted_images,
                                                    },
                                                    Extent2::new(200.0, 20.0) * 2.0,
                                                    2.0 / 20.0,
                                                    3.0 / 20.0,
                                                    2.0 / 200.0,
                                                    2.0 / 200.0,
                                                ),
                                                &mut self.options_button_text,
                                                &mut self.options_button_cursor_is_over_tracker,
                                            )),
                                        ),
                                    ),
                                ),
                            ),
                        ),
                    ),
                ),
            ),
            &self.splash_text,
        ))
    }
}

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

        let raw_title_pixel_texture = jar.read_image_part("terrain.png", [16, 0], [16, 16]).await?;

        let button_raw_image = jar.read_image("gui/gui.png").await?;
        let button_images = LoadTile9ImagesConfig {
            raw_image: &button_raw_image,
            px_start: [0, 66].into(),
            px_extent: [200, 20].into(),
            px_top: 2,
            px_bottom: 3,
            px_left: 2,
            px_right: 2,
        }.load(&renderer);

        let button_highlighted_images = LoadTile9ImagesConfig {
            raw_image: &button_raw_image,
            px_start: [0, 86].into(),
            px_extent: [200, 20].into(),
            px_top: 2,
            px_bottom: 3,
            px_left: 2,
            px_right: 2,
        }.load(&renderer);

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

        let bg_image = renderer.load_image(jar.read("gui/background.png").await?)?;
       
        let title_block = McTitleGuiBlock::new(
            &renderer,
            &mut rng,
            raw_title_pixel_texture,
        );

        let splash_text = McSplashTextGuiBlock::new(
            &renderer,
            font,
        );

        Ok(Game {
            size,
            scale,
            renderer,
            jar,

            font,

            rng,

            gui_state: GuiState {
                bg_image,

                title_block,

                button_images,
                button_highlighted_images,

                version_text,
                copyright_text,
                singleplayer_button_text,
                multiplayer_button_text,
                mods_button_text,
                options_button_text,
                
                singleplayer_button_cursor_is_over_tracker: CursorIsOverTrackerGuiBlock::new(),
                multiplayer_button_cursor_is_over_tracker: CursorIsOverTrackerGuiBlock::new(),
                mods_button_cursor_is_over_tracker: CursorIsOverTrackerGuiBlock::new(),
                options_button_cursor_is_over_tracker: CursorIsOverTrackerGuiBlock::new(),

                splash_text,
            },

            //splash_text,
            //splash_size,

            //ui,


            debug_dot: None,
        })
    }

    pub async fn draw<'a>(&mut self, elapsed: f32) -> Result<()> {
        trace!(%elapsed, "updating");

        self.gui_state.title_block.update(elapsed);
        self.gui_state.splash_text.update(elapsed);

        trace!("drawing");

        let mut frame = FrameContent::new();

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
            let gui = self.gui_state.gui();
            let ((), (), sized_gui) = gui.size(self.size.w, self.size.h, self.scale);
            sized_gui.visit_nodes(GuiVisitor::new(&mut DrawGuiVisitorTarget {
                renderer: &self.renderer,
                frame_content: &mut frame,
            }));
        }

        self.renderer.draw_frame(&frame)
    }

    pub async fn set_size(&mut self, size: Extent2<u32>) -> Result<()> {
        info!(?size, "setting size");

        self.renderer.resize(size);
        self.size = size.map(|n| n as f32);

        
        Ok(())
    }

    pub async fn set_scale(&mut self, scale: f32) -> Result<()> {
        info!(?scale, "setting scale");

        self.scale = scale;

        Ok(())
    }

    pub fn cursor_moved(&mut self, pos: Vec2<f32>) -> Result<()> {
        let event = InputEvent::CursorMoved(pos);

        struct HandleInputEventGuiVisitorTarget<'r> {
            renderer: &'r Renderer,
            cumul_reverse_transform_stack: Vec<Option<Transform2>>,
            event: InputEvent,
        }

        impl<'r> HandleInputEventGuiVisitorTarget<'r> {
            pub fn new(renderer: &'r Renderer, event: InputEvent) -> Self {
                HandleInputEventGuiVisitorTarget {
                    renderer,
                    cumul_reverse_transform_stack: vec![Some(Transform2::identity())],
                    event,
                }
            }

            fn trim_stack(&mut self, stack_len: usize) {
                while self.cumul_reverse_transform_stack.len() > stack_len + 1 {
                    self.cumul_reverse_transform_stack.pop().unwrap();
                }
            }

            fn cumul_reverse_transform(&self) -> Option<Transform2> {
                self.cumul_reverse_transform_stack[self.cumul_reverse_transform_stack.len() - 1]
            }
        }

        impl<'r, 'a> GuiVisitorTarget<'a> for HandleInputEventGuiVisitorTarget<'r> {
            fn push_modifier(&mut self, stack_len: usize, modifier: Modifier2) {
                self.trim_stack(stack_len);

                let cumul_reverse_transform = self
                    .cumul_reverse_transform()
                    .and_then(|cumul_reverse_transform| {
                        if let Modifier2::Transform(transform) = modifier {
                            if let Some(reverse_transform) = transform.reverse() {
                                Some(reverse_transform.then(&cumul_reverse_transform))
                            } else {
                                None
                            }
                        } else {
                            Some(cumul_reverse_transform)
                        }
                    });

                self.cumul_reverse_transform_stack.push(cumul_reverse_transform);
            }

            fn visit_node<I: GuiNode<'a>>(&mut self, stack_len: usize, mut node: I) {
                self.trim_stack(stack_len);

                let reverse_transformed_event = self.event.clone()
                    .filter_map_pos(|pos| self
                        .cumul_reverse_transform()
                        .map(|cumul_reverse_transform| cumul_reverse_transform.apply(pos)));

                if let Some(reverse_transformed_event) = reverse_transformed_event {
                    node.handle_input_event(&self.renderer, reverse_transformed_event);
                }
            }
        }

        {
            let gui = self.gui_state.gui();
            let ((), (), sized_gui) = gui.size(self.size.w, self.size.h, self.scale);
            sized_gui.visit_nodes(GuiVisitor::new(&mut HandleInputEventGuiVisitorTarget::new(
                &self.renderer,
                event,
            )));
        }

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