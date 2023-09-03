
#[macro_use]
extern crate tracing;

mod name;
mod fs_util;
mod http_util;
mod download;
mod glob;

use crate::name::AssetName;
use std::{
    path::PathBuf,
    env,
};
use tokio::fs;
use anyhow::Result;


const ENV: &'static str = "NOT_MINECRAFT_DATA_DIR";
const DEFAULT_DATA_DIR: &'static str = "notminecraft";
const ASSETS_SUBDIR: &'static str = "assets";
const TMP_SUBDIR: &'static str = "tmp";


/// Path of directory to store local data in.
#[derive(Debug, Clone)]
pub struct DataDir(pub PathBuf);

impl DataDir {
    pub fn new() -> Self {
        DataDir(env::var(ENV).map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(DEFAULT_DATA_DIR)))
    }

    pub fn subdir(&self, subdir: &str) -> PathBuf {
        self.0.join(subdir)
    }

    pub fn assets_subdir(&self) -> PathBuf {
        self.subdir(ASSETS_SUBDIR)
    }

    pub fn tmp_subdir(&self) -> PathBuf {
        self.subdir(TMP_SUBDIR)
    }

    /// Read asset with the given name.
    pub async fn get_asset(&self, name: &str) -> Option<Vec<u8>> {
        let name = AssetName::try_new(name).unwrap();
        let path = self.0.join(ASSETS_SUBDIR).join(name.to_path());
        fs::read(&path).await
            .map_err(|e| error!(%e, %name, "error reading asset"))
            .ok()
    }

    /// Read all assets which match the pattern. The pattern is like a normal
    /// asset name, except the last part can contain `*` characters which match
    /// on any sequence of file name characters.
    ///
    /// Returns `None` rather than `Some([])`. This is because the typical use
    /// case for this is an asset with multiple variants that can be randomly
    /// sampled between, wherein an empty set would cause errors.
    pub async fn match_assets(&self, glob: &str) -> Option<Vec<Vec<u8>>> {
        glob::match_assets(self, glob).await
    }

    /// Check whether the assets dir exists locally. Doesn't actually verify
    /// integrity. The user should be free to hack it, and the user may simply
    /// delete it fully if they want to be prompted for regeneration.
    pub async fn assets_present(&self) -> Result<bool> {
        let path = self.0.join(ASSETS_SUBDIR);
        Ok(fs_util::exists(&path).await?)
    }

    /// Download all assets into the local assets dir.
    pub async fn download_assets(&self) -> Result<()> {
        download::download_assets(self).await?;
        Ok(())
    }
}
