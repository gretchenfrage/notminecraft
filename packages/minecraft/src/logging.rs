//! Global logging system.

use std::{
    fs::File,
    sync::Arc,
    env,
    panic,
};
use backtrace::Backtrace;
use tracing_subscriber::{
    fmt::{
        self,
        time::uptime,
    },
    prelude::*,
    Registry,
    EnvFilter,
};


/// Default logging environment filter. Our crates are debug, everything else is warn.
const DEFAULT_FILTER: &'static str =
    "warn,chunk_data=debug,get_assets=debug,graphics=debug,mesh_data=debug,minecraft=debug,opentype437=debug";

/// Initializes a `tracing` logging backend which outputs to stdout and also a `log` file. Accepts
/// ecosystem-standard `RUST_LOG` env filters. Configures some other logging tweaks too.
pub fn init_logging() {
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
}
