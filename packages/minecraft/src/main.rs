
#[macro_use]
extern crate tracing;

use minecraft::{
    logging::init_logging,
    thread_pool::ThreadPool,
    game_data::GameData,
    server::{
        channel::*,
        network::NetworkServer,
        save_db::SaveDb,
        runner::run,
    },
};
#[cfg(feature = "client")]
use minecraft::{
    asset_download_prompt::asset_download_prompt,
    gui::GuiEventLoop,
    asset::{
        loader::AssetLoader,
        Assets,
    },
    menu::main_menu::MainMenu,
};
use get_assets::DataDir;
use std::{
    sync::Arc,
    thread,
};
use tokio::runtime::Runtime;


// main method when compiled in server only mode
#[cfg(not(feature = "client"))]
fn main() {
    init_logging();
    info!("starting server");
    run_server(DataDir::new(), "server", "0.0.0.0:35565");
}

// main method when compiled with client
#[cfg(feature = "client")]
fn main() {
    init_logging();
    if false {
        // TODO parse CLI
        info!("starting server");
        run_server(DataDir::new(), "server", "0.0.0.0:35565");
    } else {
        info!("starting windowed client");
        let game = Arc::new(GameData::new());
        let rt = Runtime::new().expect("error creating tokio runtime");
        let data_dir = DataDir::new();
        maybe_download_assets(&rt, &data_dir);
        let thread_pool = ThreadPool::new();
        let mut event_loop = GuiEventLoop::new(rt.handle(), thread_pool.clone());
        let assets = rt
            .block_on(Assets::load(&mut AssetLoader::new(&data_dir, &mut event_loop.renderer)));
        let gui_state = MainMenu::new(&event_loop.renderer, &assets);
        event_loop.run(Box::new(gui_state), assets, data_dir, game);
    }
}

#[cfg(feature = "client")]
fn maybe_download_assets(rt: &Runtime, data_dir: &DataDir) {
    let result = rt.block_on(asset_download_prompt(&data_dir));
    if let Err(e) = result {
        error!(%e, "unable to download assets");
    }
}

// run server until it stops, or panic
fn run_server(data_dir: DataDir, save_file_name: &str, bind_to: &str) {
    let game = Arc::new(GameData::new());
    let save_db = SaveDb::open(save_file_name, &data_dir, &game).expect("error opening save file");
    let rt = Runtime::new().expect("error creating tokio runtime");
    let thread_pool = ThreadPool::new();
    let (server_send, server_recv) = channel();
    stop_on_kill(server_send.clone());
    let network_server = NetworkServer::new(server_send.clone());
    network_server.handle().bind(bind_to.to_owned(), rt.handle(), &game);
    run(server_send, server_recv, thread_pool, network_server, save_db, game);
}

// hook up sigkill to graceful server shutdown
fn stop_on_kill(server_send: ServerSender) {
    let result = ctrlc::set_handler(move || {
        let server_send = server_send.clone();
        // spawn another thread to actually do this to avoid deadlock
        // because interrupt signals are funnie
        thread::spawn(move || {
            info!("stopping server (process received kill signal)");
            server_send.send_stop();
        });
    });
    if let Err(e) = result {
        warn!(%e, "error setting kill signal handler");
    }
}
