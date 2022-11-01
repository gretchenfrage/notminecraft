
#[derive(Debug)]
pub struct Localization {
	pub menu_version: String,
	pub menu_uncopyright: String,
	pub menu_singleplayer: String,
	pub menu_multiplayer: String,
	pub menu_mods: String,
	pub menu_options: String,
	pub splash_text: String,
}

impl Localization {
	pub fn new() -> Self {
		Localization {
			menu_version: "menu_version".to_owned(),
			menu_uncopyright: "menu_uncopyright".to_owned(),
			menu_singleplayer: "menu_singleplayer".to_owned(),
			menu_multiplayer: "menu_multiplayer".to_owned(),
			menu_mods: "menu_mods".to_owned(),
			menu_options: "menu_options".to_owned(),
			splash_text: "splash_text".to_owned(),
		}
	}
}
