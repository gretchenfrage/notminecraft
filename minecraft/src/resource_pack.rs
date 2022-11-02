
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
	pub fn new(renderer: &mut Renderer) -> Self {
		let font = renderer
			.load_font_437(fs::read(
				"/home/phoenix/minecraft-beta-1.0_01/font/default.png"
			).unwrap()).unwrap();
		let title_pixel = 
			renderer.load_image_raw(
				image::load_from_memory(&fs::read(
					"/home/phoenix/minecraft-beta-1.0_01/terrain.png"
				).unwrap()).unwrap()
				.crop_imm(
					16,
					0,
					16,
					16,
				)
			);

		let gui_png =
			image::load_from_memory(&fs::read(
				"/home/phoenix/minecraft-beta-1.0_01/gui/gui.png"
			).unwrap()).unwrap();
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

        let menu_bg = renderer
        	.load_image(fs::read(
        		"/home/phoenix/minecraft-beta-1.0_01/gui/background.png"
        	).unwrap()).unwrap();
		
		ResourcePack {
			font,
			title_pixel,
			button,
			button_highlighted,
			menu_bg,
		}
	}
}
