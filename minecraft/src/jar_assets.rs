//! Extracting assets from minecraft.jar.

use crate::font_437::Font437;
use std::{
    env,
    collections::HashMap,
};
use anyhow::*;
use async_zip::read::fs::ZipFileReader;
use tokio::io::AsyncReadExt;
use vek::*;
use image::DynamicImage;
use ab_glyph::FontArc;


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

    pub async fn read_image_part(
        &self,
        path: impl AsRef<str>,
        start: impl Into<Vec2<u32>>,
        extent: impl Into<Extent2<u32>>,
    ) -> Result<DynamicImage> {
        let start = start.into();
        let extent = extent.into();

        let data = self.read(path).await?;
        let image = image::load_from_memory(&data)?;
        Ok(image.crop_imm(
            start.x,
            start.y,
            extent.w,
            extent.h,
        ))
    }

    pub async fn read_font_437(&self, path: impl AsRef<str>) -> Result<FontArc> {
        let data = self.read(path).await?;
        let font = Font437::new(data)?;
        Ok(FontArc::new(font))
    }

    pub async fn read_properties(&self, path: impl AsRef<str>) -> Result<HashMap<String, String>> {
        let data = self.read(path).await?;
        let string = String::from_utf8(data).map_err(|_| anyhow!("non UTF-8 data"))?;
        Ok(string
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
