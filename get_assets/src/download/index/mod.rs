
mod hash;
mod deser;

use crate::{
    http_util::get_success,
    fs_util::atomic_write,
    DataDir,
};
use self::deser::deser_index;
use reqwest::Client;
use anyhow::Result;
use url::Url;


const ASSET_INDEX_URL: &'static str =
    "https://launchermeta.mojang.com/v1/packages/3d8e55480977e32acd9844e545177e69a52f594b/pre-1.6.json";

const RESOURCE_URL_BASE: &'static str =
    "https://resources.download.minecraft.net";


pub async fn download_index_assets(
    base: &DataDir,
    http_client: &mut Client,
) -> Result<()> {
    info!("downloading pre-1.6.json index");

    let index = get_success(http_client, ASSET_INDEX_URL).await?;
    let index = deser_index(index.as_ref())?;

    let base_url = Url::parse(RESOURCE_URL_BASE).unwrap();

    for (name, hash) in index {
        info!("downloading {}", name);

        let mut url = base_url.clone();
        {
            let mut url_path = url.path_segments_mut().unwrap();
            url_path.push(hash.prefix());
            url_path.push(hash.as_ref());
        }
        let content = get_success(http_client, url).await?;
        let path = base.assets_subdir().join(name.to_path());
        atomic_write(base, path, content.as_ref()).await?;
    }

    Ok(())
}
