//! Command line interface for prompting user to download assets.

use get_assets::DataDir;
use std::io::{
    Write,
    stdout,
};
use tokio::io::{
    stdin,
    BufReader,
    AsyncBufReadExt,
};
use anyhow::{
    Result,
    ensure,
    bail,
};


/// Command line interface for prompting user to download assets.
pub async fn asset_download_prompt(base: &DataDir) -> Result<()> {
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
