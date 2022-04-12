
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
        window::WindowAttributes,
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


async fn window_main(event_loop: EventLoopHandle, mut events: EventReceiver) -> Result<()> {
    let frames_per_second = 60;
    let frame_delay = Duration::from_secs(1) / frames_per_second;

    let window = event_loop.create_window(WindowAttributes {
        inner_size: Some(Size::Logical(LogicalSize {
            width: 850.0,
            height: 480.0,
        })),
        title: "Minecraft".into(),
        ..Default::default()
    }).await?;
    let window = Arc::new(window);
    let mut renderer = Renderer::new(Arc::clone(&window)).await?;
    let jar_reader = JarReader::new().await?;
    let menu_bg = renderer.load_image(jar_reader.read("gui/background.png").await?)?;

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
                debug!(scale_factor=%window.scale_factor());
                debug!(%canvas_size);
                let result = renderer.draw_frame(|mut canvas| {
                    canvas
                        .with_color([64, 64, 64, 0xFF])
                        .draw_image(
                            &menu_bg,
                            [0.0, 0.0],
                            canvas_size / (64.0 * window.scale_factor() as f32),
                        );
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
