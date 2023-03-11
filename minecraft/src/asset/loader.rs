
use crate::{
    sound::{
        SoundClip,
        SoundEffect,
    },
    gui::blocks::{
        Tile9CropConfig,
        tile_9_crop,
        Tile9Parts,
    },
};
use graphics::{
    Renderer,
    frame_content::{
        GpuImage,
        GpuImageArray,
        FontId,
    },
};
use get_assets::DataDir;
use std::borrow::Cow;
use image::{
    DynamicImage,
    imageops::{
        self,
        FilterType,
    },
};
use anyhow::Result;
use vek::*;


const MISSING_PNG: &'static [u8] = include_bytes!("missing.png");
const MISSING_OGG: &'static [u8] = include_bytes!("missing.ogg");


#[derive(Debug)]
pub struct AssetLoader<'a> {
    pub base: &'a DataDir,
    pub renderer: &'a mut Renderer,
}

impl<'a> AssetLoader<'a> {
    async fn load_raw_image(&mut self, name: &str) -> DynamicImage {
        self.base
            .get_asset(name).await
            .and_then(|bytes| image::load_from_memory(&bytes)
                .map_err(|e| error!(
                    %e, %name,
                    "image bytes failed to parse",
                ))
                .ok())
            .unwrap_or_else(|| image::load_from_memory(MISSING_PNG)
                .expect("missing.png bytes failed to parse"))
    }

    pub async fn load_image(&mut self, name: &str) -> GpuImage {
        let image = self.load_raw_image(name).await;
        self.renderer.load_image_raw(image)
    }

    pub async fn load_image_array(&mut self, names: &[&str]) -> GpuImageArray {
        let mut images = Vec::new();
        for name in names {
            images.push(self.load_raw_image(name).await);
        }
        let size =
            Extent2 {
                w: images.iter()
                    .map(|image| image.width())
                    .max()
                    .unwrap_or(1),
                h: images.iter()
                    .map(|image| image.height())
                    .max()
                    .unwrap_or(1),
            };
        self.renderer.load_image_array_raw(size, images)
    }

    pub async fn load_font_437(&mut self, name: &str) -> FontId {
        let image = self.load_raw_image(name).await;
        let image = imageops::resize(&image, 128, 128, FilterType::Nearest);
        self.renderer.load_font_437_raw(&image).unwrap()
    }

    pub async fn load_image_atlas<E>(
        &mut self,
        name: &str,
        sprites: E,
    ) -> ImageAtlas
    where
        E: Into<Extent2<u32>>,
    {
        ImageAtlas {
            image: self.load_raw_image(name).await,
            sprites: sprites.into(),
            renderer: &mut self.renderer,
        }
    }

    pub async fn load_image_clipper<E>(
        &mut self,
        name: &str,
        norm_size: E,
    ) -> ImageClipper
    where
        E: Into<Extent2<u32>>,
    {
        ImageClipper {
            image: self.load_raw_image(name).await,
            norm_size: norm_size.into(),
            renderer: &mut self.renderer,
        }
    }

    pub async fn load_sound_effect(&mut self, glob: &str) -> SoundEffect {
        self.base.match_assets(glob).await
            .map(|variants| variants
                .into_iter()
                .filter_map(|bytes| SoundClip::new(bytes)
                    .map_err(|e| error!(
                        %e, %glob,
                        "audio bytes failed to parse",
                    ))
                    .ok())
                .collect::<Vec<_>>())
            .filter(|vec| !vec.is_empty())
            .map(SoundEffect::new)
            .unwrap_or_else(|| SoundClip::new(MISSING_OGG.into())
                .expect("missing.ogg bytes failed to parse")
                .into())
    }
}

#[derive(Debug)]
pub struct ImageAtlas<'a> {
    image: DynamicImage,
    sprites: Extent2<u32>,
    renderer: &'a mut Renderer,
}

impl<'a> ImageAtlas<'a> {
    fn image_size(&self) -> Extent2<u32> {
        Extent2 {
            w: self.image.width(),
            h: self.image.height(),
        }
    }

    pub fn load_sprite<V: Into<Vec2<u32>>>(&mut self, sprite: V) -> GpuImage {
        let sprite = sprite.into();
        assert!(sprite.x < self.sprites.w, "sprite coordinates out of range");
        assert!(sprite.y < self.sprites.h, "sprite coordinates out of range");
        let start = Vec2::from(self.image_size()) * sprite / self.sprites;
        let ext = self.image_size() / self.sprites;
        let image = self.image.crop_imm(start.x, start.y, ext.w, ext.h);
        self.renderer.load_image_raw(image)
    }

    pub fn load_sprite_array<I>(&mut self, sprites: I) -> GpuImageArray
    where
        I: IntoIterator,
        <I as IntoIterator>::Item: Into<Vec2<u32>>,
    {
        let mut images = Vec::new();
        let ext = self.image_size() / self.sprites;
        for sprite in sprites {
            let sprite = sprite.into();
            assert!(
                sprite.x < self.sprites.w,
                "sprite coordinates out of range",
            );
            assert!(
                sprite.y < self.sprites.h,
                "sprite coordinates out of range",
            );
            let start = Vec2::from(self.image_size()) * sprite / self.sprites;
            images.push(self.image.crop_imm(start.x, start.y, ext.w, ext.h));
        }
        self.renderer.load_image_array_raw(ext, images)
    }
}

#[derive(Debug)]
pub struct ImageClipper<'a> {
    image: DynamicImage,
    norm_size: Extent2<u32>,
    renderer: &'a mut Renderer,
}

impl<'a> ImageClipper<'a> {
    fn image_size(&self) -> Extent2<u32> {
        Extent2 {
            w: self.image.width(),
            h: self.image.height(),
        }
    }

    pub fn load_clip<V, E>(&mut self, norm_start: V, norm_ext: E) -> GpuImage
    where
        V: Into<Vec2<u32>>,
        E: Into<Extent2<u32>>,
    {
        let norm_start = norm_start.into();
        let norm_ext = norm_ext.into();
        let start = norm_start * self.image_size() / self.norm_size;
        let ext = norm_ext * self.image_size() / self.norm_size;
        let image = self.image.crop_imm(start.x, start.y, ext.w, ext.h);
        self.renderer.load_image_raw(image)
    }
}
