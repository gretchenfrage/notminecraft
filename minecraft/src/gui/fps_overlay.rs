
use crate::{
	asset::Assets,
	gui::{
		*,
		blocks::*,
	},
	util::hex_color::hex_color,
};
use graphics::frame_content::{
	HAlign,
	VAlign,
};


#[derive(Debug)]
pub struct FpsOverlay {
	gui_text: GuiTextBlock,
}

impl FpsOverlay {
	pub fn new(fps: f32, assets: &Assets) -> Self {
		let gui_text = GuiTextBlock::new(&GuiTextBlockConfig {
			text: &format!("{} fps", fps),
			font: assets.font,
			logical_font_size: 16.0,
			color: hex_color(0x505050FF),
			h_align: HAlign::Right,
			v_align: VAlign::Top,
			wrap: false,
		});
		FpsOverlay {
			gui_text,
		}
	}

	fn gui<'a>(
		&'a mut self,
		_ctx: &'a GuiWindowContext,
	) -> impl GuiBlock<'a, DimParentSets, DimParentSets>
	{
		margin(4.0, 4.0, 4.0, 4.0,
			&mut self.gui_text,
		)
	}
}

impl GuiStateFrame for FpsOverlay {
	impl_visit_nodes!();
}
