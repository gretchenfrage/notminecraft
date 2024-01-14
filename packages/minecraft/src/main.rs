
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
    gui_state_main_menu::MainMenu,
};
use get_assets::DataDir;
use std::{
    sync::Arc,
    thread,
    env::args,
};
use tokio::runtime::Runtime;


const CLI_INTRO: &'static str = r#"This is Not Minecraft Beta 1.0.2.

Created by Phoenix Kahlo.
Everything in the universe is in the public domain."#;

#[cfg(feature = "client")]
const CLI_HELP: &'static str = r#"
Examples:

    [this command]
    Run the client.

    [this command] --server
    Run the server

    [this command] --server --save=server --bind=127.0.0.1:35565
    Run the server with explicit options.

    (Note: Change 127.0.0.1 to 0.0.0.0 to allow connections from other computers).

Env var examples:
    RUST_LOG=minecraft=trace
    Changes logging levels"#;

#[cfg(not(feature = "client"))]
const CLI_HELP_SERVER_ONLY: &'static str = r#"
This binary has been compiled in server-only mode.
Examples:

    [this command]
    Run the server

    [this command] --save=server --bind=127.0.0.1:35565
    Run the server with explicit options.

    (Note: Change 127.0.0.1 to 0.0.0.0 to allow connections from other computers).

Env var examples:
    RUST_LOG=minecraft=trace
    Changes logging levels"#;


// main method when compiled with client
#[cfg(feature = "client")]
fn main() {
    println!("{}", CLI_INTRO);
    init_logging();

    let args = args().collect::<Vec<_>>();
    if args.get(1).map(String::as_str) == Some("--help") {
        println!("{}", CLI_HELP);
    } else if args.get(1).map(String::as_str) == Some("--server") {
        run_server_from_cli(&args);
    } else {
        info!("starting client");
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

// main method when compiled in server only mode
#[cfg(not(feature = "client"))]
fn main() {
    println!("{}", CLI_INTRO);
    init_logging();
    let args = args().collect::<Vec<_>>();
    if args.get(1).map(String::as_str) == Some("--help") {
        println!("{}", CLI_HELP_SERVER_ONLY);
    } else {
        run_server_from_cli(&args);
    }
}

#[cfg(feature = "client")]
fn maybe_download_assets(rt: &Runtime, data_dir: &DataDir) {
    let result = rt.block_on(asset_download_prompt(&data_dir));
    if let Err(e) = result {
        error!(%e, "unable to download assets");
    }
}

// parse CLI args and run server from that
fn run_server_from_cli(args: &Vec<String>) {
    info!("starting server");
    let save_file_name = args.iter()
        .filter_map(|arg| arg.strip_prefix("--save="))
        .next()
        .unwrap_or("server");
    let bind_to = args.iter()
        .filter_map(|arg| arg.strip_prefix("--bind="))
        .next()
        .unwrap_or("127.0.0.1:35565");
    run_server(DataDir::new(), save_file_name, bind_to);
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
