
use crate::sound::{
    SoundClip,
    SoundEffect,
};
use graphics::{
    Renderer,
    frame_content::{
        GpuImageArray,
        FontId,
    },
};
use get_assets::DataDir;
use std::{
    borrow::Borrow,
    collections::HashMap,
    cell::RefCell,
    ops::Index,
    iter::repeat,
};
use image::{
    DynamicImage,
    imageops::{
        self,
        FilterType,
    },
};
use vek::*;


const MISSING_PNG: &'static [u8] = include_bytes!("missing.png");
const MISSING_OGG: &'static [u8] = include_bytes!("missing.ogg");
const MISSING_PROPERTY: &'static str = "[MISSING]";


#[derive(Debug)]
pub struct AssetLoader<'a> {
    base: &'a DataDir,
    renderer: RefCell<&'a mut Renderer>,
}

fn load_missing_image(renderer: &mut Renderer) -> GpuImageArray {
    let image = image::load_from_memory(MISSING_PNG)
        .expect("missing.png bytes failed to parse");
    renderer
        .load_image_array_raw(
            [image.width(), image.height()].into(),
            [image],
        )
}

fn load_missing_image_array(renderer: &mut Renderer, len: usize) -> GpuImageArray {
    let image = image::load_from_memory(MISSING_PNG)
        .expect("missing.png bytes failed to parse");
    renderer
        .load_image_array_raw(
            [image.width(), image.height()].into(),
            repeat(&image).take(len),
        )
}

impl<'a> AssetLoader<'a> {
    pub fn new(base: &'a DataDir, renderer: &'a mut Renderer) -> Self {
        AssetLoader {
            base,
            renderer: RefCell::new(renderer),
        }
    }

    async fn load_raw_image(&self, name: &str) -> Option<DynamicImage> {
        self.base
            .get_asset(name).await
            .and_then(|bytes| image::load_from_memory(&bytes)
                .map_err(|e| error!(
                    %e, %name,
                    "image bytes failed to parse",
                ))
                .ok())
    }

    pub async fn load_image_array(&self, names: &[&str]) -> GpuImageArray {
        let mut images = Vec::new();
        for name in names {
            let image = self.load_raw_image(name).await
                .unwrap_or_else(|| image::load_from_memory(MISSING_PNG)
                    .expect("missing.png bytes failed to parse"));
            images.push(image);
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
        self.renderer.borrow_mut().load_image_array_raw(size, images)
    }

    pub async fn load_font_437(&self, name: &str) -> FontId {
        let image = self.load_raw_image(name).await
            .unwrap_or_else(|| image::load_from_memory(MISSING_PNG)
                .expect("missing.png bytes failed to parse"));
        // TODO: support higher resolution 437-style fonts and also have a real default font
        let image = imageops::resize(&image, 128, 128, FilterType::Nearest).into();
        self.renderer.borrow_mut().load_font_437_raw(&image).unwrap()
    }

    pub async fn load_image_atlas<'b, E>(
        &'b self,
        name: &str,
        sprites: E,
    ) -> ImageAtlas<'a, 'b>
    where
        E: Into<Extent2<u32>>,
    {
        ImageAtlas {
            image: self.load_raw_image(name).await,
            sprites: sprites.into(),
            renderer: &self.renderer,
        }
    }

    pub async fn load_image_clipper<'b, E>(
        &'b self,
        name: &str,
        norm_size: E,
    ) -> ImageClipper<'a, 'b>
    where
        E: Into<Extent2<u32>>,
    {
        ImageClipper {
            image: self.load_raw_image(name).await,
            norm_size: norm_size.into(),
            renderer: &self.renderer,
        }
    }

    pub async fn load_sound_effect(&self, glob: &str) -> SoundEffect {
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

    pub async fn load_properties(&self, name: &str) -> Properties {
        Properties(self.base
            .get_asset(name).await
            .and_then(|bytes| String::from_utf8(bytes)
                .map_err(|e| error!(%e, %name, "non-utf8 properties file"))
                .ok())
            .unwrap_or_default()
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

#[derive(Debug)]
pub struct ImageAtlas<'a, 'b> {
    image: Option<DynamicImage>,
    sprites: Extent2<u32>,
    renderer: &'b RefCell<&'a mut Renderer>,
}

impl<'a, 'b> ImageAtlas<'a, 'b> {
    fn image_size(&self) -> Extent2<u32> {
        Extent2 {
            w: self.image.as_ref().unwrap().width(),
            h: self.image.as_ref().unwrap().height(),
        }
    }

    pub fn load_sprite<V: Into<Vec2<u32>>>(&self, sprite: V) -> GpuImageArray {
        let sprite = sprite.into();
        assert!(sprite.x < self.sprites.w, "sprite coordinates out of range");
        assert!(sprite.y < self.sprites.h, "sprite coordinates out of range");
        if let Some(ref image) = self.image {
            let start = Vec2::from(self.image_size()) * sprite / self.sprites;
            let ext = self.image_size() / self.sprites;
            let image = image.crop_imm(start.x, start.y, ext.w, ext.h);
            self.renderer.borrow_mut()
                .load_image_array_raw(
                    [image.width(), image.height()].into(),
                    [image],
                )
        } else {
            load_missing_image(&mut *self.renderer.borrow_mut())
        }
    }

    pub fn load_sprite_array<I>(&self, sprites: I) -> GpuImageArray
    where
        I: IntoIterator,
        <I as IntoIterator>::Item: Into<Vec2<u32>>,
    {
        if let Some(ref image) = self.image {
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
                images.push(image.crop_imm(start.x, start.y, ext.w, ext.h));
            }
            self.renderer.borrow_mut().load_image_array_raw(ext, images)
        } else {
            let len = sprites.into_iter().count();
            load_missing_image_array(&mut *self.renderer.borrow_mut(), len)
        }
    }
}

#[derive(Debug)]
pub struct ImageClipper<'a, 'b> {
    image: Option<DynamicImage>,
    norm_size: Extent2<u32>,
    renderer: &'b RefCell<&'a mut Renderer>,
}

impl<'a, 'b> ImageClipper<'a, 'b> {
    fn image_size(&self) -> Extent2<u32> {
        Extent2 {
            w: self.image.as_ref().unwrap().width(),
            h: self.image.as_ref().unwrap().height(),
        }
    }

    pub fn load_clip<V, E>(&self, norm_start: V, norm_ext: E) -> GpuImageArray
    where
        V: Into<Vec2<u32>>,
        E: Into<Extent2<u32>>,
    {
        if let Some(ref image) = self.image {
            let norm_start = norm_start.into();
            let norm_ext = norm_ext.into();
            let start = norm_start * self.image_size() / self.norm_size;
            let ext = norm_ext * self.image_size() / self.norm_size;
            let image = image.crop_imm(start.x, start.y, ext.w, ext.h);
            self.renderer.borrow_mut()
                .load_image_array_raw(
                    [image.width(), image.height()].into(),
                    [image],
                )
        } else {
            load_missing_image(&mut *self.renderer.borrow_mut())
        }
    }
}

#[derive(Debug, Clone)]
pub struct Properties(HashMap<String, String>);

impl<K: Borrow<str>> Index<K> for Properties {
    type Output = str;

    fn index(&self, k: K) -> &str {
        self.0.get(k.borrow()).map(String::as_str).unwrap_or(MISSING_PROPERTY)
    }
}
