
use crate::{
	resource_pack::ResourcePack,
	localization::Localization,
	gui::{
		*,
		blocks::*,
	},
	util::hex_color::hex_color,
};
use graphics::{
	Renderer,
	frame_content::{
		HAlign,
		VAlign,
	},
};
use vek::*;


pub struct MainMenu {
	version_text: GuiTextBlock,
    uncopyright_text: GuiTextBlock,
}

impl MainMenu {
	pub fn new(
		renderer: &Renderer,
		resources: &ResourcePack,
		lang: &Localization,
	) -> Self
	{
		let version_text = GuiTextBlock::new(&GuiTextBlockConfig {
			text: &lang.menu_version,
			font: resources.font,
			logical_font_size: 16.0,
			color: hex_color(0x505050FF),
			h_align: HAlign::Left,
			v_align: VAlign::Top,
			wrap: true,
		});
		let uncopyright_text = GuiTextBlock::new(&GuiTextBlockConfig {
			text: &lang.menu_uncopyright,
			font: resources.font,
			logical_font_size: 16.0,
			color: Rgba::white(),
			h_align: HAlign::Right,
			v_align: VAlign::Bottom,
			wrap: true,
		});
		MainMenu {
			version_text,
			uncopyright_text,
		}
	}

	fn gui_bg<'a>(
		ctx: &'a GuiWindowContext
	) -> impl GuiBlock<'a, DimParentSets, DimParentSets>
	{
		modify(Rgba::new(0.25, 0.25, 0.25, 1.0),
            tile_image(&ctx.resources().menu_bg, 64.0)
        )
	}

	fn gui_corner_text<'a>(
		&'a mut self,
		ctx: &'a GuiWindowContext,
	) -> impl GuiBlock<'a, DimParentSets, DimParentSets>
	{
		margin(4.0, 4.0, 4.0, 4.0,
			layer((
				&mut self.version_text,
				&mut self.uncopyright_text,
			))
		)
	}

	fn gui<'a>(
		&'a mut self,
		ctx: &'a GuiWindowContext,
	) -> impl GuiBlock<'a, DimParentSets, DimParentSets>
	{
		layer((
			Self::gui_bg(ctx),
			self.gui_corner_text(ctx),
		))
		//tile_image(&ctx.spatial.global.resources.menu_bg, [508.0, 460.0])
		/*
	layer_gui_block((
            modifier_gui_block(
                Rgba::new(0.25, 0.25, 0.25, 1.0),
                    tile_image_gui_block(
                    &self.bg_image,
                    [64.0; 2],
                ),
            ),
            h_margin_gui_block(
                4.0,
                4.0,
                v_margin_gui_block(
                    4.0,
                    4.0,
                    layer_gui_block((
                        &mut self.version_text,
                        &mut self.copyright_text,
                    )),
                ),
            ),
            v_center_gui_block(
                0.0,
                v_stack_gui_block(
                    0.0,
                    (
                        h_center_gui_block(
                            0.5,
                            h_stable_unscaled_dim_size_gui_block(
                                500.0,
                                v_stable_unscaled_dim_size_gui_block(
                                    200.0,
                                    &self.title_block,
                                ),
                            ),
                        ),
                        h_center_gui_block(
                            0.5,
                            h_stable_unscaled_dim_size_gui_block(
                                400.0,
                                v_stack_gui_block(
                                    25.0 / 2.0,
                                    (
                                        v_stable_unscaled_dim_size_gui_block(
                                            40.0,
                                            layer_gui_block((
                                                tile_9_gui_block(
                                                    match self.singleplayer_button_cursor_is_over_tracker.cursor_is_over {
                                                        false => &self.button_images,
                                                        true => &self.button_highlighted_images,
                                                    },
                                                    Extent2::new(200.0, 20.0) * 2.0,
                                                    2.0 / 20.0,
                                                    3.0 / 20.0,
                                                    2.0 / 200.0,
                                                    2.0 / 200.0,
                                                ),
                                                &mut self.singleplayer_button_text,
                                                &mut self.singleplayer_button_cursor_is_over_tracker,
                                            )),
                                        ),
                                        v_stable_unscaled_dim_size_gui_block(
                                            40.0,
                                            layer_gui_block((
                                                tile_9_gui_block(
                                                    match self.multiplayer_button_cursor_is_over_tracker.cursor_is_over {
                                                        false => &self.button_images,
                                                        true => &self.button_highlighted_images,
                                                    },
                                                    Extent2::new(200.0, 20.0) * 2.0,
                                                    2.0 / 20.0,
                                                    3.0 / 20.0,
                                                    2.0 / 200.0,
                                                    2.0 / 200.0,
                                                ),
                                                &mut self.multiplayer_button_text,
                                                &mut self.multiplayer_button_cursor_is_over_tracker,
                                            )),
                                        ),
                                        v_stable_unscaled_dim_size_gui_block(
                                            40.0,
                                            layer_gui_block((
                                                tile_9_gui_block(
                                                    match self.mods_button_cursor_is_over_tracker.cursor_is_over {
                                                        false => &self.button_images,
                                                        true => &self.button_highlighted_images,
                                                    },
                                                    Extent2::new(200.0, 20.0) * 2.0,
                                                    2.0 / 20.0,
                                                    3.0 / 20.0,
                                                    2.0 / 200.0,
                                                    2.0 / 200.0,
                                                ),
                                                &mut self.mods_button_text,
                                                &mut self.mods_button_cursor_is_over_tracker,
                                            )),
                                        ),
                                        v_stable_unscaled_dim_size_gui_block(
                                            40.0,
                                            layer_gui_block((
                                                tile_9_gui_block(
                                                    match self.options_button_cursor_is_over_tracker.cursor_is_over {
                                                        false => &self.button_images,
                                                        true => &self.button_highlighted_images,
                                                    },
                                                    Extent2::new(200.0, 20.0) * 2.0,
                                                    2.0 / 20.0,
                                                    3.0 / 20.0,
                                                    2.0 / 200.0,
                                                    2.0 / 200.0,
                                                ),
                                                &mut self.options_button_text,
                                                &mut self.options_button_cursor_is_over_tracker,
                                            )),
                                        ),
                                    ),
                                ),
                            ),
                        ),
                    ),
                ),
            ),
            &self.splash_text,
        ))
		*/
	}
}

impl GuiStateFrame for MainMenu {
	fn visit_nodes<'a, T: GuiVisitorTarget<'a>>(
        &'a mut self,
        ctx: &'a GuiWindowContext,
        mut visitor: GuiVisitor<T>,
    ) {
		let ((), (), sized) = self
			.gui(ctx)
			.size(
				ctx.spatial.global,
				ctx.size.w as f32,
				ctx.size.h as f32,
				ctx.scale,
			);
		sized.visit_nodes(&mut visitor)
    }
}
