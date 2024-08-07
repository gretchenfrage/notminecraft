//! Defining game's specific assets and loading them from file system.

pub mod loader;
pub mod meshes;

use self::{
    loader::{
        AssetLoader,
        Properties,
    },
    meshes::ItemMesh,
    consts::*,
};
use crate::{
    sound::SoundEffect,
    gui::blocks::{
        Tile9Parts,
        Tile9CropConfig,
        tile_9_crop,
    },
};
use std::{
    cmp::max,
    ops::Index,
};
use graphics::frame_content::{
    FontId,
    GpuImageArray,
};
use image::{
    DynamicImage,
    RgbaImage,
};
use vek::*;


pub mod consts {
    // block tex indexes (BTIs):

    pub const BTI_STONE: usize = 0;
    pub const BTI_DIRT: usize = 1;
    pub const BTI_GRASS_SIDE: usize = 2;
    pub const BTI_GRASS_TOP: usize = 3;
    pub const BTI_PLANKS: usize = 4;
    pub const BTI_BRICK: usize = 5;
    pub const BTI_GLASS: usize = 6;
    pub const BTI_LOG_SIDE: usize = 7;
    pub const BTI_LOG_TOP: usize = 8;
    pub const BTI_DOOR_UPPER: usize = 9;
    pub const BTI_DOOR_LOWER: usize = 10;
    pub const BTI_CHEST_FRONT: usize = 11;
    pub const BTI_CHEST_SIDE: usize = 12;
    pub const BTI_CHEST_TOP_BOTTOM: usize = 13;

    // item texture indexes (ITIs):

    pub const ITI_STICK: usize = 0; 

    // item mesh indexes (IMIs):

    pub const IMI_STONE: usize = 0;
    pub const IMI_DIRT: usize = 1;
    pub const IMI_GRASS: usize = 2;
    pub const IMI_PLANKS: usize = 3;
    pub const IMI_BRICK: usize = 4;
    pub const IMI_GLASS: usize = 5;
    pub const IMI_STICK: usize = 6;
}


macro_rules! lang {
    ($( $item:ident, )*)=>{
        #[derive(Debug, Clone)]
        pub struct Lang {$(
            pub $item: String,
        )*}

        #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
        #[allow(non_camel_case_types)]
        pub enum LangKey {$(
            $item,
        )*}

        impl Lang {
            pub fn new(properties: &Properties) -> Self {
                Lang {$(
                    $item: properties[stringify!($item).replace("_", ".")].to_owned(),
                )*}
            }
        }

        impl Index<LangKey> for Lang {
            type Output = str;

            fn index(&self, key: LangKey) -> &str {
                match key {$(
                    LangKey::$item => &self.$item,
                )*}
            }
        }
    };
}

lang!(
    menu_version,
    menu_uncopyright,
    menu_singleplayer,
    menu_multiplayer,
    menu_mods,
    menu_options,
    menu_quit,
    menu_splash,

    gui_done,
    gui_cancel,

    options_title,
    options_on,
    options_off,

    multiplayer_title,
    multiplayer_connect,

    tile_stone_name,
    tile_grass_name,
    tile_dirt_name,
    tile_planks_name,
    tile_brick_name,
    tile_glass_name,

    item_stick_name,
);


#[derive(Debug)]
pub struct Assets {
    pub lang: Lang,

    pub font: FontId,

    pub menu_title_pixel: GpuImageArray,
    pub menu_button: Tile9Parts<GpuImageArray>,
    pub menu_button_highlight: Tile9Parts<GpuImageArray>,
    pub menu_bg: GpuImageArray,

    pub hud_crosshair: GpuImageArray,
    pub hud_hotbar: GpuImageArray,
    pub hud_hotbar_selected: GpuImageArray,

    pub blocks: GpuImageArray,
    pub items: GpuImageArray,

    pub mob_char: GpuImageArray,

    pub click_sound: SoundEffect,
    pub grass_step_sound: SoundEffect,
    pub grass_dig_sound: SoundEffect,

    //pub block_item_mesh: Mesh,
    //pub item_meshes: Vec<ItemMesh>,

    pub gui_inventory: GpuImageArray,
    pub gui_chest: GpuImageArray,

    pub vignette: GpuImageArray,
    pub sun: GpuImageArray,
    pub moon: GpuImageArray,
}


impl Assets {
    pub async fn load(loader: &mut AssetLoader<'_>) -> Self {
        let terrain = loader.load_image_atlas("terrain.png", 16).await;
        let items = loader.load_image_atlas("gui/items.png", 16).await;
        let gui = loader.load_image_clipper("gui/gui.png", 256).await;
        let icons = loader.load_image_clipper("gui/icons.png", 256).await;

        let lang = loader.load_properties("lang/en_US.lang").await
            .with_default("menu.splash", "Now it's YOUR craft!")
            .with_default("menu.version", "Not Minecraft Beta 1.0.2")
            .with_default("menu.uncopyright", "Everything in the universe is in the public domain.");
        let lang = Lang::new(&lang);

        let assets = Assets {
            lang,
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
                [11, 1],
                [10, 1],
                [9, 1],
            ]),
            items: items.load_sprite_array([
                [5, 3], // 0: stick
            ]),
            mob_char: loader.load_image_array(&["mob/char.png"]).await,
            click_sound: loader.load_sound_effect("sound3/random/click.ogg").await,
            grass_step_sound: loader.load_sound_effect("sound3/step/grass*.ogg").await,
            grass_dig_sound: loader.load_sound_effect("sound3/dig/grass*.ogg").await,
            
            //item_meshes: vec![
            //    ItemMesh::load_basic_block(&loader, BTI_STONE),
            //    ItemMesh::load_basic_block(&loader, BTI_DIRT),
            //    ItemMesh::load_grass_block(&loader),
            //    ItemMesh::load_basic_block(&loader, BTI_PLANKS),
            //    ItemMesh::load_basic_block(&loader, BTI_BRICK),
            //    ItemMesh::load_basic_block(&loader, BTI_GLASS),
            //    ItemMesh::Item(ITI_STICK),
            //],
            
            gui_inventory: loader.load_image_clipper("gui/inventory.png", 256).await.load_clip([0, 0], [176, 166]),
            gui_chest: loader.load_image_clipper("gui/container.png", 256).await.load_clip([0, 0], [176, 222]),
            
            vignette: load_vignette(loader).await,
            sun: load_sun_moon(loader, "terrain/sun.png").await,
            moon: load_sun_moon(loader, "terrain/moon.png").await,
        };
        assets
    }
}

async fn load_vignette(loader: &mut AssetLoader<'_>) -> GpuImageArray {
    let mut image;
    if let Some(asset_image) = loader.load_raw_image("misc/vignette.png").await {
        image = asset_image.into_rgba8();
        for pixel in image.pixels_mut() {
            let alpha = pixel[0];
            *pixel = [0, 0, 0, alpha].into()
        }
    } else {
        image = RgbaImage::new(1, 1);
    }
    loader.renderer.borrow().load_image_array_raw(
        [image.width(), image.height()].into(),
        [DynamicImage::from(image)],
    )
}

async fn load_sun_moon(loader: &mut AssetLoader<'_>, path: &str) -> GpuImageArray {
    let mut image;
    if let Some(asset_image) = loader.load_raw_image(path).await {
        image = asset_image.into_rgba8();
        for pixel in image.pixels_mut() {
            let alpha = max(max(pixel[0], pixel[1]), pixel[2]);
            if alpha != 0 {
                for i in 0..3 {
                    pixel[i] = (pixel[i] as u32 * 0xff / alpha as u32) as u8;
                }
            }
            pixel[3] = alpha;
        }
    } else {
        image = RgbaImage::new(1, 1);
    }
    loader.renderer.borrow().load_image_array_raw(
        [image.width(), image.height()].into(),
        [DynamicImage::from(image)],
    )
}
