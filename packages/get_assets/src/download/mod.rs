
mod jar;
mod index;

use crate::DataDir;
use reqwest::Client;
use anyhow::Result;


pub async fn download_assets(base: &DataDir) -> Result<()> {
    let mut http_client = Client::new();
    jar::download_jar_assets(base, &mut http_client).await?;
    index::download_index_assets(base, &mut http_client).await?;
    Ok(())
}
