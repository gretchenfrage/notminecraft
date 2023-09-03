//! Save "file" handling (actually a rocksdb database).

mod entry;

use get_assets::DataDir;
use std::sync::Arc;
use anyhow::*;

const SAVES_SUBDIR: &'static str = "saves";



/*
/// Handle to an open save file.
pub struct SaveFile {

}

impl SaveFile {
    /// Open existing save file, or create one if none exists, within data dir.
    pub fn open(name: &str, schema: &Arc<SaveSchema>, data_dir: DataDir) -> Result<Self> {

    }
}
*/