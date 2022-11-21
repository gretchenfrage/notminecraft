
use crate::gui::{
    *,
    blocks::*,
};


#[derive(Debug)]
pub struct BasicDemo {

}

impl BasicDemo {
    pub fn new() -> Self {
        BasicDemo {}
    }

    fn gui<'a>(
        &'a mut self,
        _: &'a GuiWindowContext,
    ) -> impl GuiBlock<'a, DimParentSets, DimParentSets>
    {
        layer(())
    }
}

impl GuiStateFrame for BasicDemo {
    impl_visit_nodes!();

    fn on_key_press_semantic(
        &mut self,
        ctx: &GuiWindowContext,
        key: VirtualKeyCode,
    ) {
        if key == VirtualKeyCode::Escape {
            ctx.global().pop_state_frame();
        }
    }
}
