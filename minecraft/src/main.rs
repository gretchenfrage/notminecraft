
#[macro_use]
extern crate tracing;


pub mod sound;
pub mod asset;
pub mod gui;
pub mod util;
pub mod main_menu;
pub mod chunk_mesh;
pub mod game_data;
pub mod singleplayer;
pub mod item;


use crate::{
    game_data::GameData,
    gui::GuiEventLoop,
    main_menu::MainMenu,
    asset::{
        loader::AssetLoader,
        Assets,
    },
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

    // download assets, maybe
    let rt = Runtime::new().unwrap();
    let base = DataDir::new();
    let _ = rt
        .block_on(asset_download_prompt(&base))
        .map_err(|e| error!(%e, "unable to acquire assets"));

    // initialize rest of game runtime, including actually loading assets
    let mut event_loop = GuiEventLoop::new();

    let assets = rt
        .block_on(async {
            let mut loader = AssetLoader::new(&base, &mut event_loop.renderer);
            Assets::load(&mut loader).await
        });

    let game = GameData::new();
    let game = Arc::new(game);

    let gui_state = MainMenu::new(
        &event_loop.renderer,
        &assets
    );

    // enter window event loop
    event_loop.run(Box::new(gui_state), assets, game);
}
