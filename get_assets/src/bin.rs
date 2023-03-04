
use get_assets::DataDir;
use crate::ms_uptime::MsUptime;
use anyhow::Result;
use tracing_subscriber::{
    prelude::*,
    Registry,
    EnvFilter,
    fmt,
};


mod ms_uptime {
    use std::{
        fmt::Result,
        time::Instant,
    };
    use tracing_subscriber::fmt::{
        format::Writer,
        time::FormatTime,
    };

    #[derive(Debug, Clone)]
    pub struct MsUptime(Instant);

    impl MsUptime {
        pub fn new() -> Self {
            MsUptime(Instant::now())
        }
    }

    impl FormatTime for MsUptime {
        fn format_time(&self, w: &mut Writer) -> Result {
            let elapsed = self.0.elapsed();
            write!(w, "{:.3}s", elapsed.as_millis() as f32 / 1000.0)
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // initialize logging
    let format = fmt::format()
        .compact()
        .with_timer(MsUptime::new())
        .with_line_number(true);
    let stdout_log = fmt::layer()
        .event_format(format);
    let subscriber = Registry::default()
        .with(EnvFilter::from_default_env())
        .with(stdout_log);
    tracing::subscriber::set_global_default(subscriber)
        .expect("unable to install log subscriber");

    let base = DataDir::new();
    //base.download_assets().await?;
    println!("{:#?}", base.match_assets("sound/step/grass*.ogg").await.unwrap().len());
    Ok(())
}
