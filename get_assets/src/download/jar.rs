
use crate::{
    DataDir,
    http_util::get_success,
    fs_util::atomic_write,
};
use std::io::{
    Cursor,
    Read,
};
use reqwest::Client;
use zip::read::ZipArchive;
use anyhow::{
    anyhow,
    Result,
};


const JAR_URL: &'static str =
    "https://piston-data.mojang.com/v1/objects/76d35cb452e739bd4780e835d17faf0785d755f9/client.jar";
const JAR_ENTRY_BLACKLIST: &'static [&'static str] =
    &[
        ".class",
        ".DSA",
        ".SF",
        ".MF",
        "null",
    ];

pub async fn download_jar_assets(
    base: &DataDir,
    http_client: &mut Client,
) -> Result<()> {
    info!("downloading jar");
    let jar = get_success(http_client, JAR_URL).await?;
    let mut jar = ZipArchive::new(Cursor::new(jar))?;

    for i in 0..jar.len() {
        let mut entry = jar.by_index(i)?;
        let entry_path = entry.enclosed_name()
            .ok_or_else(|| anyhow!(
                "zip file contains illegal name {:?}",
                entry.name(),
            ))?;
        if entry_path.ends_with("/") {
            continue;
        }
        let skip = entry_path
            .file_name()
            .and_then(|oss| oss.to_str())
            .map(|file_name| JAR_ENTRY_BLACKLIST
                .iter()
                .any(|suffix| file_name.ends_with(suffix)))
            .unwrap_or(true);
        if skip {
            continue;
        }
        trace!("jar-extracting {:?}", entry_path);
        let target_path = base.assets_subdir().join(entry_path);
        let mut content = Vec::new();
        entry.read_to_end(&mut content)?;
        atomic_write(base, target_path, content.as_ref()).await?;
    }

    Ok(())
}
