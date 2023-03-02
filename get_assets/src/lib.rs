
#[macro_use]
extern crate tracing;

mod name;
mod fs_util;
mod http_util;
mod download;

use crate::name::AssetName;
use std::path::PathBuf;
use anyhow::Result;
use tokio::fs;


const DEFAULT_DATA_DIR: &'static str = "notminecraft";
const ASSETS_SUBDIR: &'static str = "assets";
const TMP_SUBDIR: &'static str = "tmp";


#[derive(Debug, Clone)]
pub struct DataDir(pub PathBuf);

impl DataDir {
    pub fn new() -> Self {
        DataDir(PathBuf::from(DEFAULT_DATA_DIR))
    }

    pub fn assets_subdir(&self) -> PathBuf {
        self.0.join(ASSETS_SUBDIR)
    }

    pub fn tmp_subdir(&self) -> PathBuf {
        self.0.join(TMP_SUBDIR)
    }

    pub async fn get_asset(&self, name: &str) -> Option<Vec<u8>> {
        let name = AssetName::try_new(name).unwrap();
        let path = self.0.join(ASSETS_SUBDIR).join(name.to_path());
        fs::read(&path).await
            .map_err(|e| error!(%e, %name, "error reading asset"))
            .ok()
    }

    pub async fn assets_present(&self) -> Result<bool> {
        let path = self.0.join(ASSETS_SUBDIR);
        Ok(fs_util::exists(&path).await?)
    }

    pub async fn download_assets(&self) -> Result<()> {
        download::download_assets(self).await?;
        Ok(())
    }
}
