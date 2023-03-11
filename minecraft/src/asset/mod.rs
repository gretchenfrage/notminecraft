
mod loader;

use self::loader::AssetLoader;
use crate::{
    sound::SoundEffect,
    gui::blocks::Tile9Parts,
};
use graphics::frame_content::{
    FontId,
    GpuImage,
    GpuImageArray,
};
use vek::*;


#[derive(Debug, Clone)]
pub struct Assets {
    pub font: FontId,

    pub menu_title_pixel: GpuImageArray,
    pub menu_button: Tile9Parts<GpuImage>,
    pub menu_button_highlight: Tile9Parts<GpuImage>,
    pub menu_bg: GpuImage,

    pub hud_crosshair: GpuImage,
    pub hud_hotbar: GpuImage,
    pub hud_hotbar_selected: GpuImage,

    pub blocks: GpuImageArray,

    pub click_sound: SoundEffect,

    pub menu_splash_text: String,

    pub menu_version: String,
    pub menu_uncopyright: String,

    pub menu_singleplayer: String,
    pub menu_multiplayer: String,
    pub menu_mods: String,
    pub menu_options: String,

    /// Baseline sky color at no-rain daytime.
    pub sky_day: Rgb<f32>,
    /// Baseline sky color at no-rain nighttime.
    pub sky_night: Rgb<f32>,
    /// Baseline sky color at rainy daytime.
    pub sky_day_rain: Rgb<f32>,
    /// Baseline sky color at rainy nighttime.
    pub sky_night_rain: Rgb<f32>,
    /// Baseline fog color at no-rain daytime.
    pub fog_day: Rgb<f32>,
    /// Baseline fog color at no-rain nighttime.
    pub fog_night: Rgb<f32>,
    /// Baseline fog color at rainy daytime.
    pub fog_day_rain: Rgb<f32>,
    /// Baseline fog color at rainy nighttime.
    pub fog_night_rain: Rgb<f32>,
    /// Baseline color of sunset fog (fog with sun behind it during sunset).
    pub sky_sunset: Rgb<f32>,
}

impl Assets {
    pub async fn load(loader: &mut AssetLoader) -> Self {
        Assets {
            
        }
    }
}
