//! This is Not Minecraft Beta 1.0.2.
//!
//! Created by Phoenix Kahlo. Everything in the universe is in the public domain.

#[macro_use]
extern crate tracing;

pub mod sound;
pub mod asset;
pub mod gui;
//pub mod util;
//pub mod chunk_mesh;
//pub mod block_update_queue;
pub mod logging;
#[cfg(feature = "client")]
pub mod asset_download_prompt;
pub mod util_abort_handle;
pub mod util_per_thing;
pub mod util_array;
pub mod util_must_drain;
pub mod util_hex_color;
pub mod util_cos;
pub mod dyn_flex_channel;
pub mod game_data;
pub mod item;
pub mod physics;
pub mod game_binschema;
pub mod gui_state_main_menu;
pub mod gui_state_multiplayer_menu;
pub mod gui_state_about;
//pub mod save_file;
pub mod thread_pool;
pub mod settings;
//pub mod client;
pub mod server;
pub mod message;
pub mod sync_state_tile_blocks;






/*
use crate::{
    game_data::GameData,
    gui::GuiEventLoop,
    menu::main_menu::MainMenu,
    asset::{
        loader::AssetLoader,
        Assets,
    },
    thread_pool::ThreadPool,
    save_file::SaveFile,
    server::ServerHandle,
};
use get_assets::DataDir;
use std::{
    fs::File,
    sync::Arc,
    env::{
        self,
        args,
    },
    panic,
    io::{
        Write,
        stdout,
    },
};
use backtrace::Backtrace;
use tokio::{
    runtime::Runtime,
    io::{
        stdin,
        BufReader,
        AsyncBufReadExt,
    },
};
use tracing_subscriber::{
    fmt::{
        self,
        time::uptime,
    },
    prelude::*,
    Registry,
    EnvFilter,
};
use anyhow::{
    Result,
    ensure,
    bail,
};
use crossbeam_channel::bounded;


const DEFAULT_FILTER: &'static str =
    "warn,chunk_data=debug,get_assets=debug,graphics=debug,mesh_data=debug,minecraft=debug,opentype437=debug";

async fn asset_download_prompt(base: &DataDir) -> Result<()> {
    if base.assets_present().await? {
        return Ok(())
    }

    println!("assets directory not detected (at {})", base.assets_subdir().display());
    println!("auto-download from mojang's servers?");

    // acquire consent or early-exit
    {
        let stdout = stdout();
        let mut buf = String::new();
        let mut input = BufReader::new(stdin());
        loop {
            {
                let mut stdout = stdout.lock();
                stdout.write_all(b"[y/n] ").expect("stdout write fail");
                stdout.flush().expect("stdout flush fail");
            }

            let n = input.read_line(&mut buf).await?;
            ensure!(n != 0, "stdin closed");

            match buf.as_str() {
                "y\n" => break,
                "n\n" => bail!("could not acquire assets"),
                _ => {
                    println!("invalid input {:?}", buf);
                    buf.clear();
                }
            }
        }
    }

    base.download_assets().await?;

    Ok(())
}

fn main() {
    // initialize and install logging system
    let format = fmt::format()
        .compact()
        .with_timer(uptime())
        .with_line_number(true);
    let stdout_log = fmt::layer()
        .event_format(format);

    let log_file = File::create("log")
        .expect("unable to create log file");
    let log_file_log = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(Arc::new(log_file));

    let mut filter = DEFAULT_FILTER.to_owned();
    if let Ok(env_filter) = env::var(EnvFilter::DEFAULT_ENV) {
        filter.push(',');
        filter.push_str(&env_filter);
    }

    let subscriber = Registry::default()
        .with(EnvFilter::new(filter))
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

    // initialize things that'll be used even if it's server only
    let rt = Runtime::new().unwrap();
    let thread_pool = ThreadPool::new();
    let game = GameData::new();
    let game = Arc::new(game);
    let data_dir = DataDir::new();

    // parse args
    let args = args().collect::<Vec<_>>();
    match &args.iter().map(|s| s.as_str()).collect::<Vec<_>>()[..] {
        &[_] => {
            // run client
            // download assets, maybe
            let _ = rt
                .block_on(asset_download_prompt(&data_dir))
                .map_err(|e| error!(%e, "unable to acquire assets"));

            // initialize rest of game runtime, including actually loading assets
            let mut event_loop = GuiEventLoop::new(rt.handle(), thread_pool.clone());

            let assets = rt
                .block_on(async {
                    let mut loader = AssetLoader::new(&data_dir, &mut event_loop.renderer);
                    Assets::load(&mut loader).await
                });

            let gui_state = MainMenu::new(
                &event_loop.renderer,
                &assets
            );

            // enter window event loop
            event_loop.run(Box::new(gui_state), assets, data_dir, game);
        },
        &[_, "--server"] => {
            // run server
            info!("running server");
            let save = match SaveFile::open("server", &data_dir, &game) {
                Ok(save) => save,
                Err(e) => {
                    error!(%e, "error opening save file");
                    std::process::exit(1);
                }
            };
            let server = ServerHandle::start(save, &game, rt.handle(), &thread_pool);
            let _network_bind = server.open_to_network("0.0.0.0:35565");

            let (send_ctrlc, recv_ctrlc) = bounded(1);
            if let Err(e) = ctrlc::set_handler(move || { let _ = send_ctrlc.send(()); }) {
                error!(%e, "error setting ctrlc handler");
            }
            let _ = recv_ctrlc.recv();
            
            info!("shutting down");
            server.stop();
        }
        _ => {
            error!("invalid CLI args");
            std::process::exit(2);
        }
    };
}
*/