
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

const INDEX_ENTRY_BLACKLIST: &'static [&'static str] =
    &[
        "READ_ME_I_AM_VERY_IMPORTANT",
    ];


pub async fn download_index_assets(
    base: &DataDir,
    http_client: &mut Client,
) -> Result<()> {
    info!("downloading index ({})", ASSET_INDEX_URL);

    let index = get_success(http_client, ASSET_INDEX_URL).await?;
    let index = deser_index(index.as_ref())?;

    let base_url = Url::parse(RESOURCE_URL_BASE).unwrap();

    for (name, hash) in index {
        let file_name = name.file_name();

        let mut url = base_url.clone();
        {
            let mut url_path = url.path_segments_mut().unwrap();
            url_path.push(hash.prefix());
            url_path.push(hash.as_ref());
        }
        
        let skip = INDEX_ENTRY_BLACKLIST
            .iter()
            .any(|suffix| file_name.ends_with(suffix));
        if skip {
            trace!("skipping {} ({})", name, url);
            continue;
        }

        info!("downloading {} ({})", name, url);

        let content = get_success(http_client, url).await?;
        let path = base.assets_subdir().join(name.to_path());
        atomic_write(base, path, content.as_ref()).await?;
    }

    Ok(())
}
