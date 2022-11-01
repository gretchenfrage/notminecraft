
use crate::gui::{
	*,
	blocks::*,
};
use graphics::{
	Renderer,
	frame_content::GpuImage,
};


pub struct MainMenu {
	bg_image: GpuImage,
}

impl MainMenu {
	pub fn new(renderer: &Renderer) -> Self {
		let bg_image = renderer
			.load_image(include_bytes!("eg-img.png"))
			.unwrap();
		MainMenu {
			bg_image
		}
	}

	pub fn gui(&mut self) -> impl GuiBlock<DimParentSets, DimParentSets> {
		tile_image(&self.bg_image, [508.0, 460.0])
	}
}

impl GuiStateFrame for MainMenu {
	fn visit_nodes<'a, T: GuiVisitorTarget<'a>>(
        &'a mut self,
        ctx: &GuiWindowContext,
        mut visitor: GuiVisitor<T>,
    ) {
		let ((), (), sized) = self
			.gui()
			.size(
				ctx.spatial.global,
				ctx.size.w as f32,
				ctx.size.h as f32,
				ctx.scale,
			);
		sized.visit_nodes(&mut visitor)
    }
}
