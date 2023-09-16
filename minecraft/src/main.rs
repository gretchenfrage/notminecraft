
#[macro_use]
extern crate tracing;


pub mod sound;
pub mod asset;
pub mod gui;
pub mod util;
pub mod chunk_mesh;
pub mod block_update_queue;
pub mod game_data;
pub mod item;
pub mod physics;
pub mod client_server;
pub mod game_binschema;
pub mod menu;
pub mod save_file;
pub mod thread_pool;


use crate::{
    game_data::GameData,
    gui::GuiEventLoop,
    menu::main_menu::MainMenu,
    asset::{
        loader::AssetLoader,
        Assets,
    },
    thread_pool::ThreadPool,
};
use get_assets::DataDir;
use std::{
    fs::File,
    sync::Arc,
    env,
    panic,
    io::{
        Write,
        stdout,
    },
    env::args,
};
use backtrace::Backtrace;
use tokio::{
    runtime::{
        Runtime,
        Handle,
    },
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

    // maybe run headless
    let args = args().collect::<Vec<_>>();
    let mut client_only = false;
    match &args.iter().map(|s| s.as_str()).collect::<Vec<_>>()[..] {
        &[_] => (),
        &[_, "--server"] => {
            info!("running server");
            let result = client_server::server::run_networked_server(rt.handle(), &thread_pool, &data_dir, &game);
            match result {
                Ok(()) => {
                    info!("server shutting down");
                    return;
                }
                Err(e) => {
                    error!(%e, "server shutting down");
                    std::process::exit(1);
                },
            };
        }
        &[_, "--client-only"] => {
            client_only = true;
        }
        _ => {
            error!("invalid CLI args");
            std::process::exit(2);
        }
    };

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

    if !client_only {
        // start server in a background thread
        let rt_handle = Handle::clone(&rt.handle());
        let game_2 = Arc::clone(&game);
        let data_dir = data_dir.clone();
        std::thread::spawn(move || {
            let result = client_server::server::run_networked_server(&rt_handle, &thread_pool, &data_dir, &game_2);
            match result {
                Ok(()) => info!("server shutting down"),
                Err(e) => error!(%e, "server shutting down"),
            };
        });
    }
    
    let gui_state = MainMenu::new(
        &event_loop.renderer,
        &assets
    );

    // enter window event loop
    event_loop.run(Box::new(gui_state), assets, data_dir, game);
}
