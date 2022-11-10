
//#![feature(array_zip)]

/*
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
*/

/*
#[macro_use]
extern crate tracing;
*/

//mod game;
//pub mod resource_pack;
//pub mod localization;

#[macro_use]
extern crate tracing;


pub mod asset;
pub mod gui;
pub mod util;
pub mod main_menu;

use crate::{
    asset::jar_assets::JarAssets,
    gui::GuiEventLoop,
    main_menu::MainMenu,
};
use std::{
    fs::File,
    sync::Arc,
    env,
    panic,
};
use backtrace::Backtrace;
use tokio::runtime::Runtime;
use tracing_subscriber::{
    prelude::*,
    Registry,
    EnvFilter,
};


fn main() {
    // initialize and install logging system
    let stdout_log = tracing_subscriber::fmt::layer()
        //.with_filter(EnvFilter::from_default_env())
        .pretty();

    let log_file = File::create("log")
        .expect("unable to create log file");
    let log_file_log = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(Arc::new(log_file));

    let subscriber = Registry::default()
        .with(EnvFilter::from_default_env())
        .with(stdout_log)
        .with(log_file_log);
    //let subscriber = FmtSubscriber::builder()
        //.with_env_filter(EnvFilter::from_default_env())
        //.with_ansi(false).with_writer(std::fs::File::create("log.txt").unwrap())
    //    .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("unable to install log subscriber");
    info!("starting program");

    // make panic messages and backtrace go through logging system
    panic::set_hook(Box::new(|info| {
        error!("{}", info);
        if env::var("RUST_BACKTRACE").map(|val| val == "1").unwrap_or(false) {
            error!("{:?}", Backtrace::new());
        }
    }));
    trace!("installed custom panic hook");

    // initialize game runtime
    let mut event_loop = GuiEventLoop::new();

    let rt = Runtime::new().unwrap();
    let (
        resources,
        lang,
    ) = rt
        .block_on(JarAssets::read())
        .expect("failure to load jar assets")
        .load(&mut event_loop.renderer);

    let gui_state = MainMenu::new(
        &event_loop.renderer,
        &resources,
        &lang,
    );

    // enter window event loop
    event_loop.run(Box::new(gui_state), resources, lang);
}

/*
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
    let window = event_loop.create_window(Default::default() /*Game::window_config().await?*/).await?;
    let window = Arc::new(window);
    let size = window.inner_size();
    let size = Extent2::new(size.width, size.height);
    let renderer = Renderer::new(Arc::clone(&window)).await?;
    /*
    let mut game = Game::new(
        renderer,
        size,
        window.scale_factor() as f32,
    ).await?;
    */
    let frames_per_second = 60;
    let frame_delay = Duration::from_secs(1) / frames_per_second;

    let mut last_frame_instant = None;

    //let mut cursor_pos = None;

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
                        //game.set_size(Extent2::new(size.width, size.height)).await?;
                    }
                }
                _ => (),
            },
            Event::UserEvent(UserEvent::ScaleFactorChanged { scale_factor, .. }) => {
                //game.set_scale(scale_factor as f32).await?;
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
                /*
                trace!("drawing frame");
                let result = game.draw(elapsed).await;                
                if let Err(e) = result {
                    error!(error=%e, "draw_frame error");
                }
                */

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
*/