
pub mod loader;

use self::loader::AssetLoader;
use crate::{
    sound::SoundEffect,
    gui::blocks::{
        Tile9Parts,
        Tile9CropConfig,
        tile_9_crop,
    },
};
use graphics::frame_content::{
    FontId,
    GpuImageArray,
};
use vek::*;


#[derive(Debug, Clone)]
pub struct Assets {
    pub font: FontId,

    pub menu_title_pixel: GpuImageArray,
    pub menu_button: Tile9Parts<GpuImageArray>,
    pub menu_button_highlight: Tile9Parts<GpuImageArray>,
    pub menu_bg: GpuImageArray,

    pub hud_crosshair: GpuImageArray,
    pub hud_hotbar: GpuImageArray,
    pub hud_hotbar_selected: GpuImageArray,

    pub blocks: GpuImageArray,

    pub click_sound: SoundEffect,
    pub grass_step_sound: SoundEffect,
    pub grass_dig_sound: SoundEffect,

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
    pub async fn load(loader: &mut AssetLoader<'_>) -> Self {
        let terrain = loader.load_image_atlas("terrain.png", 16).await;
        let gui = loader.load_image_clipper("gui/gui.png", 256).await;
        let icons = loader.load_image_clipper("gui/icons.png", 256).await;
        let lang = loader.load_properties("lang/en_US.lang").await;

        let assets = Assets {
            font: loader.load_font_437("font/default.png").await,
            menu_title_pixel: terrain.load_sprite_array([[1, 0]]),
            menu_button: tile_9_crop(&Tile9CropConfig {
                base: &gui,

                start: [0, 66].into(),
                extent: [200, 20].into(),

                top: 2,
                bottom: 3,
                left: 2,
                right: 2,
            }),
            menu_button_highlight: tile_9_crop(&Tile9CropConfig {
                base: &gui,

                start: [0, 86].into(),
                extent: [200, 20].into(),

                top: 2,
                bottom: 3,
                left: 2,
                right: 2,
            }),
            menu_bg: loader.load_image_array(&["gui/background.png"]).await,
            hud_crosshair: icons.load_clip([0, 0], [15, 15]),
            hud_hotbar: gui.load_clip([0, 0], [182, 22]),
            hud_hotbar_selected: gui.load_clip([0, 22], [24, 24]),
            blocks: terrain.load_sprite_array([
                [1, 0], // 0: stone
                [2, 0], // 1: dirt
                [3, 0], // 2: grass side
                [0, 0], // 3: grass top
                [4, 0], // 4: planks
                [7, 0], // 5: brick
                [1, 3], // 6: glass
                [4, 1], // 7: log side
                [5, 1], // 8: log top
                [1, 5], // 9: door upper
                [1, 6], // 10: door lower
            ]),
            click_sound: loader.load_sound_effect("sound3/random/click.ogg").await,
            grass_step_sound: loader.load_sound_effect("sound3/step/grass*.ogg").await,
            grass_dig_sound: loader.load_sound_effect("sound3/dig/grass*.ogg").await,
            menu_splash_text: "Now it's YOUR craft!".to_owned(),
            menu_version: "Not Minecraft Beta 1.0.2".to_owned(),
            menu_uncopyright: "Everything in the universe is in the public domain.".to_owned(),
            menu_singleplayer: lang["menu.singleplayer"].to_owned(),
            menu_multiplayer: lang["menu.multiplayer"].to_owned(),
            menu_mods: lang["menu.mods"].to_owned(),
            menu_options: lang["menu.options"].to_owned(),
            sky_day:        [0.45, 0.62, 1.00].into(),
            sky_night:      [0.00, 0.02, 0.05].into(),
            sky_day_rain:   [0.24, 0.26, 0.32].into(),
            sky_night_rain: [0.00, 0.01, 0.01].into(),
            fog_day:        [0.70, 0.82, 1.00].into(),
            fog_night:      [0.02, 0.05, 0.13].into(),
            fog_day_rain:   [0.48, 0.52, 0.60].into(),
            fog_night_rain: [0.02, 0.04, 0.07].into(),
            sky_sunset:     [1.00, 0.35, 0.10].into(),
        };
        assets
    }
}
