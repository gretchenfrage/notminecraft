
use crate::{
    gui::blocks::Tile9Parts,
    asset::sound::Sound,
};
use graphics::frame_content::{
	FontId,
	GpuImage,
    GpuImageArray,
};
use vek::*;


#[derive(Debug, Clone)]
pub struct ResourcePack {
	pub font: FontId,

	pub menu_title_pixel: GpuImageArray,
	pub menu_button: Tile9Parts<GpuImage>,
	pub menu_button_highlight: Tile9Parts<GpuImage>,
	pub menu_bg: GpuImage,

    pub hud_crosshair: GpuImage,
    pub hud_hotbar: GpuImage,
    pub hud_hotbar_selected: GpuImage,

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

    pub blocks: GpuImageArray,

    pub click_sound: Sound,
    
    /*
        pub block_grass_side: usize,
        pub block_grass_side_snowy: usize,
        pub block_grass_top: usize,
        pub block_stone: usize,
        pub block_dirt: usize,
        pub block_planks: usize,
        pub block_slab_side: usize,
        pub block_slab_top: usize,
        pub block_bricks: usize,
        pub block_tnt_side: usize,
        pub block_tnt_top: usize,
        pub block_tnt_bottom: usize,
        pub block_spider_web: usize,
        pub block_rose: usize,
        pub block_dandelion: usize,
        pub block_sapling: usize,
        pub block_cobblestone: usize,
        pub block_bedrock: usize,
        pub block_sand: usize,
        pub block_gravel: usize,
        pub block_log_side: usize,
        pub block_log_top: usize,
        pub block_iron_side: usize,
        pub block_iron_top: usize,
        pub block_iron_bottom: usize,
        pub block_gold_side: usize,
        pub block_gold_top: usize,
        pub block_iron_bottom: usize,
        pub block_diamond_side: usize,
        pub block_diamond_top: usize,
        pub block_diamond_bottom: usize,
        pub block_chest_front_single: usize,
        pub block_chest_front_left: usize,
        pub block_chest_front_right: usize,
        pub block_chest_side_single: usize,
        pub block_chest_side_left: usize,
        pub block_chest_side_right: usize,
        pub block_chest_top: usize,
        pub block_red_mushroom: usize,
        pub block_brown_mushroom: usize,
        pub block_gold_ore: usize,
        pub block_iron_ore: usize,
        pub block_coal_ore: usize,
        pub block_diamond_ore: usize,
        pub block_redstone_ore: usize,
        pub block_bookshelf: usize,
        pub block_mossy_cobblestone: usize,
        pub block_obsidian: usize,
        pub block_furnace_front: usize,
        pub block_furnace_front_lit: usize,
        pub block_furnace_side: usize,
        pub block_sponge: usize,
        pub block_glass: usize,
        pub block_leaves_transparent: usize,
        pub block_leaves_opaque: usize,
        pub block_wool: usize,
        pub block_mob_spawner: usize,
        pub block_snow: usize,
        pub block_ice: usize,
        pub block_cactus_side: usize,
        pub block_cactus_top: usize,
        pub block_cactus_bottom: usize,
        pub block_clay: usize,
        pub block_sugar_cane: usize,
        pub block_juke_box_side: usize,
        pub block_juke_box_top: usize,
        pub block_torch: usize,
        pub block_door_top: usize,
        pub block_door_bottom: usize,
        pub block_iron_door_top: usize,
        pub block_iron_door_bottom: usize,
        pub block_ladder: usize,
        pub block_redstone_cross: usize,
        pub block_redstone_cross_powered: usize,
        pub block_redstone_line: usize,
        pub block_redstone_line_powered: usize,
        pub block_tilled_soil: usize,
        pub block_tilled_soil_moist: usize,
        pub block_lever: usize,
        pub block_redstone_torch: usize,
        pub block_redstone_torch_suppressed: usize,
        pub block_pumpkin_side: usize,
        pub block_pumpkin_top: usize,
        pub block_jack_o_lantern: usize,
        pub block_jack_o_lantern_lit: usize,
        pub block_soul_sand: usize,
        pub block_glow_stone: usize,
        pub block_rail_bent: usize,
        pub block_rail_straight: usize,
        pub block_wheat: [usize; 8],
        pub block_water: [usize; 5],
        pub block_lava: [usize; 5],
        pub block_mining: [usize; 10],
    */
}

impl 
