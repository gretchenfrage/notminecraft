
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
}
