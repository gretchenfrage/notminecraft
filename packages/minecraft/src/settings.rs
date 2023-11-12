
use std::{
    path::Path,
    fs::File,
    io::{
        BufReader,
        BufWriter,
    },
};
use serde::{Serialize, Deserialize};
use anyhow::*;


pub const SETTINGS_FILE_NAME: &'static str = "settings.json";


/// Game settings. A client-side global resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub fog: bool,
    pub day_night: bool,
    pub load_dist_outline: bool,
    pub chunk_outline: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            fog: true,
            day_night: true,
            load_dist_outline: false,
            chunk_outline: false,
        }
    }
}

impl Settings {
    pub fn read(path: impl AsRef<Path>) -> Self {
        Self::try_read(path).unwrap_or_default()
    }

    pub fn try_read(path: impl AsRef<Path>) -> Result<Self> {
        Ok(serde_json::from_reader(BufReader::new(File::open(path)?))?)
    }

    pub fn write(&self, path: impl AsRef<Path>) -> Result<()> {
        serde_json::to_writer_pretty(BufWriter::new(File::create(path)?), self)?;
        Ok(())
    }
}
