
#[macro_use]
extern crate tracing;


pub mod asset;
pub mod gui;
pub mod util;
pub mod main_menu;
pub mod chunk_mesh;
pub mod game_data;
pub mod singleplayer;


use crate::{
    asset::jar_assets::JarAssets,
    game_data::GameData,
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
    tracing::subscriber::set_global_default(subscriber)
        .expect("unable to install log subscriber");
    info!("starting program");

    // make panic messages and backtrace go through logging system
    panic::set_hook(Box::new(|info| {
        error!("{}", info);
        if env::var("RUST_BACKTRACE").map(|val| val == "1").unwrap_or(true) {
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
    let game = GameData::new();
    let game = Arc::new(game);

    let gui_state = MainMenu::new(
        &event_loop.renderer,
        &resources,
        &lang,
    );

    // enter window event loop
    event_loop.run(Box::new(gui_state), resources, lang, game);
}
