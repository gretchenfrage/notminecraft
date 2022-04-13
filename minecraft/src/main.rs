
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


mod jar_assets;
mod font_437;



/*
pub fn font_437_size(pixels_per_pixel: u32, scale_factor: f32) -> f32 {
    9.0 * pixels_per_pixel as f32 * scale_factor
}

pub fn draw_font_437(canvas: Canvas2d, font_id: FontId, pixels_per_pixel:)
*/
async fn window_main(event_loop: EventLoopHandle, mut events: EventReceiver) -> Result<()> {
    let frames_per_second = 60;
    let frame_delay = Duration::from_secs(1) / frames_per_second;
    
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

    let arcfont = jar_reader.read_font_437("font/default.png").await?;
    let font = renderer.load_font_preloaded(arcfont.clone());

    use ab_glyph::Font;
    let gid = arcfont.glyph_id('h');
    dbg!(arcfont.units_per_em());
    dbg!(arcfont.ascent_unscaled());
    dbg!(arcfont.descent_unscaled());
    dbg!(arcfont.line_gap_unscaled());
    dbg!(arcfont.h_advance_unscaled(gid));
    dbg!(arcfont.h_side_bearing_unscaled(gid));
    dbg!(arcfont.v_advance_unscaled(gid));
    dbg!(arcfont.v_side_bearing_unscaled(gid));
    dbg!(arcfont.kern_unscaled(gid, gid));

    let font2 = renderer.load_font(include_bytes!("../../graphics/src/assets/DejaVuSans.ttf"))?;

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
            horizontal_align: HorizontalAlign::Left { width: Some(renderer.size().w as f32) },
            vertical_align: VerticalAlign::Top,
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
            horizontal_align: HorizontalAlign::Right { width: renderer.size().w as f32 },
            //horizontal_align: HorizontalAlign::Left { width: None },
            vertical_align: VerticalAlign::Bottom { height: renderer.size().h as f32 },
            //vertical_align: VerticalAlign::Top,
        });

    const LOGO_SIZE: f32 = 470.0;
    const LOGO_TOP_GAP: f32 = 66.0;
    let logo_size = LOGO_SIZE * window.scale_factor() as f32;
    let logo_top_gap = LOGO_TOP_GAP * window.scale_factor() as f32;
    let logo = renderer.load_image(jar_reader.read("gui/logo.png").await?)?;


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
                        .draw_image(&logo)
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
