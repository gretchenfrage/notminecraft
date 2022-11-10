//! Extracting assets from minecraft.jar.


use std::collections::HashMap;
use tokio::io::AsyncReadExt;
use async_zip::read::fs::ZipFileReader;
use image::DynamicImage;
use anyhow::*;
use vek::*;


/// Tokio-based reader for extracting assets from minecraft.jar. 
pub struct JarReader(ZipFileReader);

impl JarReader {
    /*
    /// Attempt to construct, getting the minecraft.jar path from the
    /// `MINECRAFT_JAR` environment variable.
    pub async fn new() -> Result<Self> {
        Ok(JarReader(ZipFileReader::new(env::var("MINECRAFT_JAR")?).await?))
    }

    pub async fn from_file(file: File) -> Result<Self> {
        Ok(JarReader(ZipFileReader::new(file).await?))
    }*/

    pub async fn new(path: &str) -> Result<Self> {
        Ok(JarReader(ZipFileReader::new(path.to_string()).await?))
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

    pub async fn read_string(&self, path: impl AsRef<str>) -> Result<String> {
        String::from_utf8(self.read(path).await?)
            .map_err(|_| anyhow!("non UTF-8 data"))
    }

    pub async fn read_image(
        &self,
        path: impl AsRef<str>,
    ) -> Result<DynamicImage>
    {
        let data = self.read(path).await?;
        Ok(image::load_from_memory(&data)?)
    }

    pub async fn read_image_part(
        &self,
        path: impl AsRef<str>,
        start: impl Into<Vec2<u32>>,
        extent: impl Into<Extent2<u32>>,
    ) -> Result<DynamicImage>
    {
        let start = start.into();
        let extent = extent.into();

        let image = self.read_image(path).await?;

        Ok(image.crop_imm(
            start.x,
            start.y,
            extent.w,
            extent.h,
        ))
    }

    pub async fn read_properties(&self, path: impl AsRef<str>) -> Result<HashMap<String, String>> {
        Ok(self
            .read_string(path).await?
            .lines()
            .filter_map(|line| line
                .find('=')
                .map(|i| (
                    line[0..i].to_owned(),
                    line[i + 1..].to_owned(),
                )))
            .collect())
    }
}
