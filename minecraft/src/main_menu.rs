
use crate::gui::{
	*,
	blocks::*,
};
use graphics::Renderer;


pub struct MainMenu {
}

impl MainMenu {
	pub fn new(renderer: &Renderer) -> Self {
		MainMenu {
		}
	}

	pub fn gui<'a>(
		&'a mut self,
		ctx: &GuiWindowContext,
	) -> impl GuiBlock<'a, DimParentSets, DimParentSets>
	{
		tile_image(&ctx.spatial.global.resources.menu_bg, [508.0, 460.0])
	}
}

impl GuiStateFrame for MainMenu {
	fn visit_nodes<'a, T: GuiVisitorTarget<'a>>(
        &'a mut self,
        ctx: &GuiWindowContext,
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
