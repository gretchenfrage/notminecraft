
use crate::gui::blocks::{
	Tile9Images,
	Tile9ImagesBuilder,
};
use graphics::{
	Renderer,
	frame_content::{
		FontId,
		GpuImage,
	},
};
use std::fs;


#[derive(Debug)]
pub struct ResourcePack {
	pub font: FontId,
	pub title_pixel: GpuImage,
	pub button: Tile9Images,
	pub button_highlighted: Tile9Images,
	pub menu_bg: GpuImage,
}

impl ResourcePack {
	pub fn new(renderer: &Renderer) -> Self {
		let font = renderer.load_font(fs::read("/usr/share/fonts/truetype/fonts-gujr-extra/aakar-medium.ttf").unwrap()).unwrap();
		let title_pixel = renderer.load_image(fs::read("/home/phoenix/Pictures/Screenshots/Screenshot from 2022-11-01 18-04-29.png").unwrap()).unwrap();
		let gui_png = image::load_from_memory(&fs::read("/home/phoenix/Pictures/Screenshots/Screenshot from 2022-11-01 18-04-29.png").unwrap()).unwrap();
		let button = Tile9ImagesBuilder {
            base_image: &gui_png,
            px_start: [0, 66].into(),
            px_extent: [200, 20].into(),
            px_top: 2,
            px_bottom: 3,
            px_left: 2,
            px_right: 2,
        }.build(&renderer);
        let button_highlighted = Tile9ImagesBuilder {
            base_image: &gui_png,
            px_start: [0, 86].into(),
            px_extent: [200, 20].into(),
            px_top: 2,
            px_bottom: 3,
            px_left: 2,
            px_right: 2,
        }.build(&renderer);
        let menu_bg = renderer.load_image(fs::read("/home/phoenix/Pictures/Screenshots/Screenshot from 2022-11-01 18-04-29.png").unwrap()).unwrap();
		
		ResourcePack {
			font,
			title_pixel,
			button,
			button_highlighted,
			menu_bg,
		}
	}
}
