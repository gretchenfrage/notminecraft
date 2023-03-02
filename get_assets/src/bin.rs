
use get_assets::DataDir;
use anyhow::Result;
use tracing_subscriber::{
    prelude::*,
    Registry,
    EnvFilter,
};


#[tokio::main]
async fn main() -> Result<()> {
    let stdout_log = tracing_subscriber::fmt::layer()
        .pretty();
    let subscriber = Registry::default()
        .with(EnvFilter::from_default_env())
        .with(stdout_log);
    tracing::subscriber::set_global_default(subscriber)
        .expect("unable to install log subscriber");

    DataDir::new().download_assets().await?;
    Ok(())
}
