
use crate::gui::{
    GuiGlobalContext,
    SizedGuiBlock,
    GuiVisitorTarget,
    GuiVisitor,
    GuiBlock,
    DimParentSets,
    blocks::tile_9,
};
use vek::*;


pub fn button_bg<'a>() -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
    GuiButtonBgBlock
}

#[derive(Debug)]
struct GuiButtonBgBlock;

impl<'a> GuiBlock<'a, DimParentSets, DimParentSets> for GuiButtonBgBlock {
    type Sized = GuiButtonBgBlockSized;

    fn size(
        self,
        _ctx: &GuiGlobalContext<'a>,
        w: f32,
        h: f32,
        scale: f32,
    ) -> ((), (), Self::Sized)
    {
        let sized = GuiButtonBgBlockSized {
            size: Extent2 { w, h },
            scale,
        };
        ((), (), sized)
    }
}

#[derive(Debug)]
struct GuiButtonBgBlockSized {
    size: Extent2<f32>,
    scale: f32,
}

impl<'a> SizedGuiBlock<'a> for GuiButtonBgBlockSized {
    fn visit_nodes<T: GuiVisitorTarget<'a>>(
        self,
        visitor: &mut GuiVisitor<'a, '_, T>,
        forward: bool,
    ) {
        let highlight = visitor.ctx.cursor_pos
            .map(|pos|
                pos.x >= 0.0
                && pos.y >= 0.0
                && pos.x <= self.size.w
                && pos.y <= self.size.h
            )
            .unwrap_or(false);
        let images =
            if highlight { &visitor.ctx.assets().menu_button_highlight }
            else { &visitor.ctx.assets().menu_button };
        let ((), (), inner_sized) = tile_9(
            images,
            [400.0, 40.0],
            2.0 / 20.0,
            3.0 / 20.0,
            2.0 / 200.0,
            2.0 / 200.0,
        )
            .size(&visitor.ctx.global, self.size.w, self.size.h, self.scale);
        inner_sized.visit_nodes(visitor, forward);
    }
}
