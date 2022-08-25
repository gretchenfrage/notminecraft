
#![feature(array_zip)]


use crate::{
    game::Game,
    ui::{
        UiSize,
        UiPosInputEvent,
    },
};
use graphics::Renderer;
use std::{
    panic,
    sync::Arc,
    time::{
        Instant,
        Duration,
    },
    env,
};
use winit_main::{
    EventLoopHandle,
    EventReceiver,
    UserEvent,
    reexports::event::{
        Event,
        WindowEvent,
    },
};
use tracing_subscriber::{
    FmtSubscriber,
    EnvFilter,
};
use tokio::time::sleep;
use backtrace::Backtrace;
use anyhow::*;
use vek::*;

#[macro_use]
extern crate tracing;


mod game;
mod jar_assets;
mod ui;
mod ui2;


fn main() {
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        //.with_ansi(false).with_writer(std::fs::File::create("log.txt").unwrap())
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();
    info!("starting program");

    panic::set_hook(Box::new(|info| {
        error!("{}", info);
        if env::var("RUST_BACKTRACE").map(|val| val == "1").unwrap_or(false) {
            error!("{:?}", Backtrace::new());
        }
    }));
    trace!("installed custom panic hook");

    winit_main::run(|event_loop, events| async move {
        trace!("successfully bootstrapped winit + tokio");
        let result = window_main(event_loop, events).await;
        if let Err(e) = result {
            error!(error=%e, "exit with error");
        }
    });
}

async fn window_main(event_loop: EventLoopHandle, mut events: EventReceiver) -> Result<()> {
    let window = event_loop.create_window(Game::window_config().await?).await?;
    let window = Arc::new(window);
    let size = window.inner_size();
    let size = Extent2::new(size.width, size.height);
    let renderer = Renderer::new(Arc::clone(&window)).await?;
    let mut game = Game::new(
        renderer,
        UiSize {
            size: size.map(|n| n as f32),
            scale: window.scale_factor() as f32,
        },
    ).await?;

    let frames_per_second = 60;
    let frame_delay = Duration::from_secs(1) / frames_per_second;

    let mut last_frame_instant = None;

    let mut cursor_pos = None;

    loop {
        let event = events.recv().await;
        trace!(?event, "received event");
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => break,
                WindowEvent::Resized(size) => {
                    if size.width == 0 || size.height == 0 {
                        trace!("not resizing window because of 0 size"); // TODO factor in
                    } else {
                        trace!("resizing window");
                        game.set_size(Extent2::new(size.width, size.height)).await?;
                    }
                }
                WindowEvent::KeyboardInput { input, .. } => {
                    game.keyboard_input(input).await?;
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    game.mouse_wheel(delta).await?;
                }
                WindowEvent::CursorMoved { position, .. } => {
                    let pos = Vec2::new(position.x as f32, position.y as f32);
                    cursor_pos = Some(pos);
                    game.on_pos_input_event(UiPosInputEvent::CursorMoved(pos)).await?;
                }
                WindowEvent::MouseInput { button, state, .. } => {
                    if let Some(pos) = cursor_pos {
                        let event = UiPosInputEvent::MouseInput {
                            pos,
                            button,
                            state,
                        };
                        game.on_pos_input_event(event).await?;
                    } else {
                        debug!("MouseInput event with no previous CursorMoved events");
                    }
                }
                _ => (),
            },
            Event::UserEvent(UserEvent::ScaleFactorChanged { scale_factor, .. }) => {
                game.set_scale(scale_factor as f32).await?;
            }
            Event::MainEventsCleared => {
                // track time
                let now = Instant::now();
                let elapsed = last_frame_instant
                    .map(|last_frame_instant| now - last_frame_instant)
                    .unwrap_or(Duration::ZERO);
                last_frame_instant = Some(now);
                let elapsed = (elapsed.as_nanos() as f64 / 1000000000.0) as f32;

                // draw frame
                trace!("drawing frame");
                let result = game.draw(elapsed).await;                
                if let Err(e) = result {
                    error!(error=%e, "draw_frame error");
                }

                // unblock
                debug_assert!(matches!(
                    events.recv().await,
                    Event::UserEvent(UserEvent::Blocker(_)),
                ));

                // track time and wait
                let now = Instant::now();
                let next_frame_instant = last_frame_instant.unwrap() + frame_delay;
                let delay = next_frame_instant.checked_duration_since(now);
                if let Some(delay) = delay {
                    sleep(delay).await;
                }

                // request redraw
                window.request_redraw();
            },
            _ => (),
        };
    }

    Ok(())
}




/*
#[macro_use]
extern crate tracing;

use crate::{
    jar_assets::JarReader,
};
use anyhow::*;
use graphics::*;
use vek::*;
use std::{
    panic,
    sync::Arc,
    time::{
        Instant,
        Duration,
    },
    thread::sleep,
    env,
};
use winit_main::{
    EventLoopHandle,
    EventReceiver,
    reexports::{
        event::{
            Event,
            WindowEvent,
        },
        window::{
            WindowAttributes,
            Icon,
        },
        dpi::{
            Size,
            LogicalSize,
        },
    },
};
use tracing_subscriber::{
    FmtSubscriber,
    EnvFilter,
};
use backtrace::Backtrace;
use image::DynamicImage;


mod jar_assets;

struct Slice9 {
    corner_tl: GpuImage,
    corner_tr: GpuImage,
    corner_bl: GpuImage,
    corner_br: GpuImage,
    edge_t: GpuImage,
    edge_b: GpuImage,
    edge_l: GpuImage,
    edge_r: GpuImage,
    center: GpuImage,
}

impl Slice9 {
    pub fn new(
        renderer: &Renderer,
        image: DynamicImage,
        edge_t: u32,
        edge_b: u32,
        edge_l: u32,
        edge_r: u32,
    ) -> Self {
        Slice9 {
            corner_tl: renderer.load_image_raw(image.crop_imm(
                0, 0,
                edge_l, edge_t,
            )),
            corner_tr: renderer.load_image_raw(image.crop_imm(
                image.width() - edge_r, 0,
                edge_r, edge_t,
            )),
            corner_bl: renderer.load_image_raw(image.crop_imm(
                0, image.height() - edge_b,
                edge_b, edge_l,
            )),
            corner_br: renderer.load_image_raw(image.crop_imm(
                image.width() - edge_r, image.height() - edge_b,
                edge_b, edge_r,
            )),
            edge_t: renderer.load_image_raw(image.crop_imm(
                edge_l, 0,
                image.width() - edge_l - edge_r, edge_t,
            )),
            edge_b: renderer.load_image_raw(image.crop_imm(
                edge_l, image.height() - edge_b,
                image.width() - edge_l - edge_r, edge_b,
            )),
            edge_l: renderer.load_image_raw(image.crop_imm(
                0, edge_t,
                edge_l, image.height() - edge_t - edge_b,
            )),
            edge_r: renderer.load_image_raw(image.crop_imm(
                image.width() - edge_r, edge_t,
                edge_r, image.height() - edge_t - edge_b,
            )),
            center: renderer.load_image_raw(image.crop_imm(
                edge_l, edge_t,
                image.width() - edge_l - edge_r, image.height() - edge_t - edge_b,
            )),
        }
    }

    pub fn draw(
        &self,
        mut canvas: Canvas2d,
        size: Extent2<f32>,
    ) {
        let corner_tl_size = self.corner_tl.size().map(|n| n as f32);
        let corner_tr_size = self.corner_tr.size().map(|n| n as f32);
        let corner_bl_size = self.corner_bl.size().map(|n| n as f32);
        let corner_br_size = self.corner_br.size().map(|n| n as f32);
        let edge_t_size = self.edge_t.size().map(|n| n as f32);
        let edge_b_size = self.edge_b.size().map(|n| n as f32);
        let edge_l_size = self.edge_l.size().map(|n| n as f32);
        let edge_r_size = self.edge_r.size().map(|n| n as f32);
        let center_size = self.center.size().map(|n| n as f32);

        let csize = size - corner_tl_size - corner_br_size;
        let r_start = size.w - edge_r_size.w;
        let b_start = size.h - edge_b_size.h;

        canvas.reborrow()
            .with_scale(corner_tl_size)
            .draw_image(&self.corner_tl);
        canvas.reborrow()
            .with_translate([r_start, 0.0])
            .with_scale(corner_tr_size)
            .draw_image(&self.corner_tr);
        canvas.reborrow()
            .with_translate([0.0, b_start])
            .with_scale(corner_bl_size)
            .draw_image(&self.corner_bl);
        canvas.reborrow()
            .with_translate([r_start, b_start])
            .with_scale(corner_br_size)
            .draw_image(&self.corner_br);
        let tsize = Extent2::new(csize.w, edge_t_size.h);
        let bsize = Extent2::new(csize.w, edge_b_size.h);
        let lsize = Extent2::new(edge_l_size.w, csize.h);
        let rsize = Extent2::new(edge_r_size.w, csize.h);
        canvas.reborrow()
            .with_translate([corner_tl_size.w, 0.0])
            .with_scale(tsize)
            .draw_image_uv(
                &self.edge_t,
                [0.0, 0.0],
                tsize / edge_t_size,
            );
        canvas.reborrow()
            .with_translate([corner_tl_size.w, b_start])
            .with_scale(bsize)
            .draw_image_uv(
                &self.edge_b,
                [0.0, 0.0],
                bsize / edge_b_size,
            );
        canvas.reborrow()
            .with_translate([0.0, corner_tl_size.h])
            .with_scale(lsize)
            .draw_image_uv(
                &self.edge_l,
                [0.0, 0.0],
                lsize / edge_l_size,
            );
        canvas.reborrow()
            .with_translate([r_start, corner_tl_size.h])
            .with_scale(rsize)
            .draw_image_uv(
                &self.edge_r,
                [0.0, 0.0],
                rsize / edge_r_size,
            );
        canvas.reborrow()
            .with_translate(corner_tl_size)
            .with_scale(csize)
            .draw_image_uv(
                &self.center,
                [0.0, 0.0],
                csize / center_size,
            );
    }
}

async fn window_main(event_loop: EventLoopHandle, mut events: EventReceiver) -> Result<()> {
    let frames_per_second = 60;
    let frame_delay = Duration::from_secs(1) / frames_per_second;

    let start_instant = Instant::now();
    
    let jar_reader = JarReader::new().await?;
    let icon = jar_reader
        .read_image_part(
            "terrain.png",
            Vec2::new(3, 0) * 16,
            [16, 16],
        ).await?;
    let icon_width = icon.width();
    let icon_height = icon.height();

    let window = event_loop.create_window(WindowAttributes {
        inner_size: Some(Size::Logical(LogicalSize {
            width: 850.0,
            height: 480.0,
        })),
        title: "Minecraft".into(),
        window_icon: Some(Icon::from_rgba(
            icon.to_rgba8().into_raw(),
            icon_width,
            icon_height,
        )?),
        ..Default::default()
    }).await?;
    let window = Arc::new(window);
    let mut renderer = Renderer::new(Arc::clone(&window)).await?;
    let menu_bg = renderer.load_image(jar_reader.read("gui/background.png").await?)?;

    let font = renderer.load_font_437(jar_reader.read("font/default.png").await?)?;

    let button = Slice9::new(
        &renderer,
        jar_reader.read_image_part("gui/gui.png", [0, 66], [200, 20]).await?,
        3, 3, 3, 3,
    );
    let button_highlight = Slice9::new(
        &renderer,
        jar_reader.read_image_part("gui/gui.png", [0, 86], [200, 20]).await?,
        3, 3, 3, 3,
    );


    let version_text = renderer
        .lay_out_text(&TextBlock {
            spans: &[
                TextSpan {
                    text: "Not Minecraft Beta 1.0.2",
                    font_id: font,
                    font_size: 16.0 * window.scale_factor() as f32,
                    color: Rgba::new(0x50, 0x50, 0x50, 0xFF),
                },
            ],
            horizontal_align: HAlign::Left { width: Some(renderer.size().w as f32) },
            vertical_align: VAlign::Top,
        });
    let copyright_text = renderer
        .lay_out_text(&TextBlock {
            spans: &[
                TextSpan {
                    text: "Everything in the universe is in the public domain.",
                    font_id: font,
                    font_size: 16.0 * window.scale_factor() as f32,
                    color: Rgba::white(),
                },
            ],
            horizontal_align: HAlign::Right { width: renderer.size().w as f32 },
            //horizontal_align: HAlign::Left { width: None },
            vertical_align: VAlign::Bottom { height: renderer.size().h as f32 },
            //vertical_align: VAlign::Top,
        });

    const LOGO_SIZE: f32 = 470.0;
    const LOGO_TOP_GAP: f32 = 66.0;
    let logo_size = LOGO_SIZE * window.scale_factor() as f32;
    let logo_top_gap = LOGO_TOP_GAP * window.scale_factor() as f32;
    let logo = renderer.load_image(jar_reader.read("gui/logo.png").await?)?;

    const BUTTON_SIZE: Extent2<f32> = Extent2 { w: 400.0, h: 40.0 };
    let button_size = BUTTON_SIZE * window.scale_factor() as f32;

    fn lay_out_button_text(renderer: &Renderer, font_id: FontId, scale_factor: f32, text: &str) -> LayedOutTextBlock {
        let button_size = BUTTON_SIZE * scale_factor;
        renderer
            .lay_out_text(&TextBlock {
                spans: &[
                    TextSpan {
                        text,
                        font_id,
                        font_size: 16.0 * scale_factor,
                        color: Rgba::new(0xE0, 0xE0, 0xE0, 0xFF),
                    },
                ],
                horizontal_align: HAlign::Center { width: button_size.w },
                vertical_align: VAlign::Center { height: button_size.h },
            })
    }

    let lang = jar_reader.read_properties("lang/en_US.lang").await?;

    let buttons = vec![
        lay_out_button_text(&renderer, font, window.scale_factor() as f32, &lang["menu.singleplayer"]),
        lay_out_button_text(&renderer, font, window.scale_factor() as f32, &lang["menu.multiplayer"]),
        lay_out_button_text(&renderer, font, window.scale_factor() as f32, &lang["menu.mods"]),
        lay_out_button_text(&renderer, font, window.scale_factor() as f32, &lang["menu.options"]),
    ];

    let splashes = jar_reader
        .read_string("title/splashes.txt").await?
        .lines()
        .filter(|line| !line.is_empty())
        .filter(|line| !line.to_ascii_lowercase().contains("notch"))
        .map(|line| renderer
            .lay_out_text(&TextBlock {
                spans: &[
                    TextSpan {
                        text: line,
                        font_id: font,
                        font_size: 32.0 * window.scale_factor() as f32,
                        color: Rgba::new(0xFF, 0xFF, 0x00, 0xFF),
                    },
                ],
                horizontal_align: HAlign::Center { width: f32::INFINITY },
                vertical_align: VAlign::Center { height: f32::INFINITY },
            }))
        .collect::<Vec<_>>();

    let mut cursor_pos = Vec2::new(0.0, 0.0);

    loop {
        let event = events.recv().await;
        trace!(?event, "received event");
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => break,
                WindowEvent::Resized(size) => {
                    if size.width == 0 || size.height == 0 {
                        trace!("not resizing window because of 0 size");
                    } else {
                        trace!("resizing window");
                        renderer.resize(size);
                        // TODO resize graphics here
                    }
                },
                WindowEvent::CursorMoved {
                    device_id: _, // TODO: ... we could support multiple simultaneous cursors?
                    position,
                    ..
                } => {
                    cursor_pos = Vec2::new(position.x as f32, position.y as f32);
                },
                _ => (),
            },
            Event::MainEventsCleared => {
                let before_frame = Instant::now();

                // draw frame
                trace!("drawing frame");
                let canvas_size = renderer.size().map(|n| n as f32);
                //debug!(scale_factor=%window.scale_factor());
                //debug!(%canvas_size);
                let result = renderer.draw_frame(|mut canvas| {
                    canvas.reborrow()
                        .with_color([64, 64, 64, 0xFF])
                        .draw_image_uv(
                            &menu_bg,
                            [0.0, 0.0],
                            canvas_size / (64.0 * window.scale_factor() as f32),
                        );
                    /*
                    canvas
                        .with_scale(Vec2::new(1.0, 1.0) / canvas_size)
                        .with_translate(canvas_size)
                        .with_translate([-3.0, -1.0])
                        .with_color([64, 64, 64, 0xFF])
                        .draw_text(&text);
                    */
                    let mut c = canvas.reborrow()
                        .with_scale(Vec2::new(1.0, 1.0) / canvas_size)
                        .with_translate(canvas_size)
                        .with_translate([0.0, 2.0]);
                    c.reborrow()
                        .with_color([64, 64, 64, 0xFF])
                        .draw_text(&copyright_text);
                    c.reborrow()
                        .with_translate([-2.0, -2.0])
                        .draw_text(&copyright_text);

                    let mut c = canvas.reborrow()
                        .with_scale(Vec2::new(1.0, 1.0) / canvas_size)
                        .with_translate([4.0, 4.0]);
                    c.reborrow()
                        .with_translate([2.0, 2.0])
                        .with_color([64, 64, 64, 0xFF])
                        .draw_text(&version_text);
                    c.reborrow()
                        .draw_text(&version_text);

                    canvas.reborrow()
                        .with_translate([0.5, 0.0])
                        .with_scale(Vec2::new(1.0, 1.0) / canvas_size)
                        .with_translate([0.0, logo_top_gap])
                        .with_scale(logo_size)
                        .with_translate([-0.5, 0.0])
                        .draw_image(&logo);

                    let pixels_per_pixel = 2.0 * window.scale_factor() as f32;
                    
                    //let button_size = Extent2::new(100.0, 100.0);
                    let mut button_top_gap = 216.0 * window.scale_factor() as f32;
                    let intra_button_gap = 8.0 * window.scale_factor() as f32;
                    for btext in &buttons {
                        let min_pos = Vec2::new(
                            (canvas_size.w - button_size.w) / 2.0,
                            button_top_gap,
                        );
                        let max_pos = Vec2::new(
                            (canvas_size.w + button_size.w) / 2.0,
                            button_top_gap + button_size.h,
                        );
                        let cursor_over =
                            cursor_pos.x >= min_pos.x
                            && cursor_pos.x <= max_pos.x
                            && cursor_pos.y >= min_pos.y
                            && cursor_pos.y <= max_pos.y;

                        let c = canvas.reborrow()
                            .with_translate([0.5, 0.0])
                            .with_scale(Vec2::new(1.0, 1.0) / canvas_size)
                            .with_translate([-button_size.w / 2.0, button_top_gap])
                            .with_scale([pixels_per_pixel, pixels_per_pixel]);
                        let which_button =
                            if cursor_over { &button_highlight }
                            else { &button };
                        which_button.draw(c, button_size / pixels_per_pixel);
                        let mut c = canvas.reborrow()
                            .with_translate([0.5, 0.0])
                            .with_scale(Vec2::new(1.0, 1.0) / canvas_size)
                            .with_translate([0.0, button_top_gap + button_size.h / 2.0]);
                        c.reborrow()
                            .with_translate([2.0, 2.0])
                            .with_color([64, 64, 64, 0xFF])
                            .draw_text(&btext);
                        c.draw_text(&btext);
                        button_top_gap += button_size.h + intra_button_gap;
                    }
                    
                    use std::f32::consts::PI;
                    let t = Instant::now().duration_since(start_instant).as_micros();
                    let micro_period = 1000000 / 2;
                    let scale = (((t % micro_period) as f32 / micro_period as f32 * PI * 2.0).cos() / 2.0 + 0.5) * 0.05 + 0.95;
                    let mut c = canvas.reborrow()
                        .with_translate([0.5, 0.0])
                        .with_scale(Vec2::new(1.0, 1.0) / canvas_size)
                        .with_translate([-logo_size / 2.0, logo_top_gap])
                        .with_translate(logo_size * Vec2::new(0.8, 0.2))
                        .with_scale([scale, scale]);
                    let splash = &splashes[(t / micro_period) as usize % splashes.len()];
                    c.reborrow()
                        .with_translate([4.0, 4.0])
                        .with_color([64, 64, 64, 0xFF])
                        .draw_text(splash);
                    c.draw_text(splash)
                });
                if let Err(e) = result {
                    error!(error=%e, "draw_frame error");
                }

                // wait
                let after_frame = Instant::now();
                let delay = (before_frame + frame_delay)
                    .checked_duration_since(after_frame);
                if let Some(delay) = delay {
                    sleep(delay);
                }

                // request redraw
                window.request_redraw();
            },
            _ => (),
        };
    }

    Ok(())
}

fn main() {
    /*
    use ab_glyph::{Font, FontRef};
    let font = FontRef::try_from_slice(include_bytes!("../../graphics/src/assets/DejaVuSans.ttf")).unwrap();
    dbg!(font.units_per_em());
    dbg!(font.outline(font.glyph_id('a')));
    return;
    */

    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();
    info!("starting program");

    panic::set_hook(Box::new(|info| {
        error!("{}", info);
        if env::var("RUST_BACKTRACE").map(|val| val == "1").unwrap_or(false) {
            error!("{:?}", Backtrace::new());
        }
    }));
    trace!("installed custom panic hook");

    winit_main::run(|event_loop, events| async move {
        trace!("successfully bootstrapped winit + tokio");
        let result = window_main(event_loop, events).await;
        if let Err(e) = result {
            error!(error=%e, "exit with error");
        }
    });
}
*/