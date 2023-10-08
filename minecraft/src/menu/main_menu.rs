
use crate::{
    menu::{
        multiplayer_menu::MultiplayerMenu,
        about::AboutMenu,
    },
    asset::Assets,
    gui::prelude::*,
	util::hex_color::hex_color,
    client::Client,
    save_file::SaveFile,
};
use graphics::{
	Renderer,
	frame_content::{
		HAlign,
		VAlign,
	},
};
use rand::thread_rng;
use vek::*;


#[derive(Debug)]
pub struct MainMenu {
    title: GuiTitleBlock,
	version_text: GuiTextBlock<true>,
    uncopyright_text: GuiTextBlock<true>,

    singleplayer_button: MenuButton,
    multiplayer_button: MenuButton,
    about_button: MenuButton,
    exit_game_button: MenuButton,
    
    splash_text: GuiSplashText,
}

impl MainMenu {
	pub fn new(
		renderer: &Renderer,
		assets: &Assets,
	) -> Self
	{
        let title = GuiTitleBlock::new(renderer, &mut thread_rng());
		let version_text = GuiTextBlock::new(&GuiTextBlockConfig {
			text: &assets.lang.menu_version,
			font: assets.font,
			logical_font_size: 16.0,
			color: hex_color(0x505050FF),
			h_align: HAlign::Left,
			v_align: VAlign::Top,
		});
		let uncopyright_text = GuiTextBlock::new(&GuiTextBlockConfig {
			text: &assets.lang.menu_uncopyright,
			font: assets.font,
			logical_font_size: 16.0,
			color: Rgba::white(),
			h_align: HAlign::Right,
			v_align: VAlign::Bottom,
		});
        let singleplayer_button = menu_button(&assets.lang.menu_singleplayer)
            .build(assets);
        let multiplayer_button = menu_button(&assets.lang.menu_multiplayer)
            .build(assets);
        let about_button = menu_button("About")
            .build(assets);
        let exit_game_button = menu_button("Quit")
            .build(assets);
        let splash_text = GuiSplashText::new();
		MainMenu {
            title,
			version_text,
			uncopyright_text,
            singleplayer_button,
            multiplayer_button,
            about_button,
            exit_game_button,
            splash_text,
		}
	}

	fn gui<'a>(
		&'a mut self,
		ctx: &'a GuiWindowContext,
	) -> impl GuiBlock<'a, DimParentSets, DimParentSets>
	{
		layer((
			modify(Rgba::new(0.25, 0.25, 0.25, 1.0),
                tile_image(&ctx.assets().menu_bg, 64.0)
            ),
			margin(4.0, 4.0, 4.0, 4.0,
                layer((
                    v_align(0.0,
                        &mut self.version_text,
                    ),
                    v_align(1.0,
                        &mut self.uncopyright_text,
                    ),
                ))
            ),
            h_align(0.5,
                logical_width(400.0,
                    layer((
                        v_align(0.0,
                            v_stack(8.0, (
                                logical_height(200.0,
                                    align(0.5,
                                        &self.title,
                                    ),
                                ),
                                self.singleplayer_button
                                    .gui(on_singleplayer_click),
                                self.multiplayer_button
                                    .gui(on_multiplayer_click),
                                self.about_button
                                    .gui(on_about_click),
                                self.exit_game_button
                                    .gui(on_exit_game_click),
                            )),
                        ),
                        v_align(0.0,
                            logical_height(200.0,
                                &mut self.splash_text,
                            ),
                        ),
                    )),
                ),
            ),
		))
	}
}

impl GuiStateFrame for MainMenu {
	impl_visit_nodes!();

    fn update(&mut self, _: &GuiWindowContext, elapsed: f32) {
        self.title.update(elapsed);
        self.splash_text.update(elapsed);
    }
}

fn on_singleplayer_click(ctx: &GuiGlobalContext) {
    let save = SaveFile::open("server", ctx.data_dir, ctx.game).unwrap(); // TODO: don't panic
    ctx.push_state_frame(Client::new_internal(save, ctx));
}

fn on_multiplayer_click(ctx: &GuiGlobalContext) {
    ctx.push_state_frame(MultiplayerMenu::new(ctx));
}

fn on_about_click(ctx: &GuiGlobalContext) {
    ctx.push_state_frame(AboutMenu::new(ctx));
}

fn on_exit_game_click(ctx: &GuiGlobalContext) {
    ctx.pop_state_frame();
}
