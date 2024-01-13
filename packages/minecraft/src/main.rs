
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
use get_assets::DataDir;
use std::{
    sync::Arc,
    thread,
};
use tokio::runtime::Runtime;


// main method
fn main() {
    init_logging();
    info!("starting server");
    run_server(DataDir::new(), "server", "0.0.0.0:35565");
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
