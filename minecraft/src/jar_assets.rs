//! Extracting assets from minecraft.jar.

use std::env;
use anyhow::*;
use async_zip::read::fs::ZipFileReader;
use tokio::io::AsyncReadExt;


/// Reader for extracting assets from minecraft.jar.
pub struct JarReader(ZipFileReader);


impl JarReader {
    pub async fn new() -> Result<Self> {
        Ok(JarReader(ZipFileReader::new(env::var("MINECRAFT_JAR")?).await?))
    }

    pub async fn read(&self, path: impl AsRef<str>) -> Result<Vec<u8>> {
        let (index, _) = self.0
            .entry(path.as_ref())
            .ok_or_else(|| anyhow!("jar entry not found: {:?}", path.as_ref()))?;
        let mut buf = Vec::new();
        self.0
            .entry_reader(index).await?
            .read_to_end(&mut buf).await?;
        Ok(buf)
    }
}
