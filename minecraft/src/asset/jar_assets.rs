
use crate::{
	asset::{
		localization::Localization,
		resource_pack::ResourcePack,
		jar_acquire::jar_acquire,
	},
	gui::blocks::{
		Tile9CropConfig,
		tile_9_crop,
		Tile9Parts,
	},
};
use graphics::Renderer;
use image::DynamicImage;
use anyhow::*;
use vek::*;


pub struct JarAssets {
	font: DynamicImage,

	block_stone: DynamicImage,
	block_dirt: DynamicImage,

	menu_button: Tile9Parts<DynamicImage>,
	menu_button_highlight: Tile9Parts<DynamicImage>,
	menu_bg: DynamicImage,
	
	lang: Localization,
}

impl JarAssets {
	pub async fn read() -> Result<Self> {
		let jar = jar_acquire().await?;

		let font = jar.read_image("font/default.png").await?;
		ensure!(font.width() == 128, "font/default.png wrong w size");
		ensure!(font.height() == 128, "font/default.png wrong h size");
		
		let terrain = jar.read_image("terrain.png").await?;
		ensure!(terrain.width() == 256, "terrain.png wrong w size");
		ensure!(terrain.height() == 256, "terrain.png wrong h size");

		let block_stone = terrain.crop_imm(16, 0, 16, 16);
		let block_dirt = terrain.crop_imm(32, 0, 16, 16);

		let gui = jar.read_image("gui/gui.png").await?;

		let menu_button = tile_9_crop(&Tile9CropConfig {
			base: &gui,

			start: [0, 66].into(),
			extent: [200, 20].into(),

			top: 2,
			bottom: 3,
			left: 2,
			right: 2,
		})?;
		let menu_button_highlight = tile_9_crop(&Tile9CropConfig {
			base: &gui,

			start: [0, 86].into(),
			extent: [200, 20].into(),

			top: 2,
			bottom: 3,
			left: 2,
			right: 2,
		})?;

		let menu_bg = jar.read_image("gui/background.png").await?;

		let lang = jar.read_properties("lang/en_US.lang").await?;

		let	menu_splash_text =
			"Now it's YOUR craft!".to_owned();
		let menu_version =
			"Not Minecraft Beta 1.0.2".to_owned();
		let menu_uncopyright =
			"Everything in the universe is in the public domain.".to_owned();

		let lang = Localization {
			menu_splash_text,

			menu_version,
			menu_uncopyright,

			menu_singleplayer: lang["menu.singleplayer"].to_owned(),
			menu_multiplayer: lang["menu.multiplayer"].to_owned(),
			menu_mods: lang["menu.mods"].to_owned(),
			menu_options: lang["menu.options"].to_owned(),
		};

		Ok(JarAssets {
			font,
			menu_button,
			menu_button_highlight,
			menu_bg,
			block_stone,
			block_dirt,
			lang,
		})
	}

	pub fn load(self, renderer: &mut Renderer) -> (ResourcePack, Localization)
	{
		let font = renderer.load_font_437_raw(&self.font).unwrap();

		let menu_title_pixel = renderer
			.load_image_array_raw(
				Extent2::new(
					self.block_stone.width(),
					self.block_stone.height(),
				),
				[
					&self.block_stone,
				],
			);
		let menu_button = self.menu_button.load(renderer);
		let menu_button_highlight = self.menu_button_highlight.load(renderer);
		let menu_bg = renderer.load_image_raw(&self.menu_bg);

		/*
		let mut blocks = Vec::new();

		let block_stone = blocks.len();
		blocks.push(self.block_stone);

		let block_dirt = blocks.len();
		blocks.push(self.block_dirt);
		
		let blocks = renderer.load_image_array_raw(16.into(), blocks);
		*/
		let blocks = renderer
			.load_image_array_raw(
				16.into(),
				[
					self.block_stone,
					self.block_dirt,
				],
			);

		let resources = ResourcePack {
			font,
			menu_title_pixel,
			menu_button,
			menu_button_highlight,
			menu_bg,
			blocks,
			//block_stone,
			//block_dirt,
		};

		(resources, self.lang)
	}
}
