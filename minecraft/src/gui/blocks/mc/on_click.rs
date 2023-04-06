
use crate::gui::{
    GuiSpatialContext,
    MouseButton,
    GuiBlock,
    DimParentSets,
    GuiNode,
    blocks::simple_gui_block::{
        SimpleGuiBlock,
        never_blocks_cursor_impl,
    },
};
use std::fmt::{
    self,
    Formatter,
    Debug,
};


pub fn click_sound<'a>() -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
    on_any_click(|ctx, _| {
        ctx.sound_player().play(&ctx.assets().click_sound);
    })
}

pub fn on_left_click<
    'a,
    F: FnOnce(GuiSpatialContext),
>(f: F) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
    on_click(MouseButton::Left, f)
}

pub fn on_right_click<
    'a,
    F: FnOnce(GuiSpatialContext),
>(f: F) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
    on_click(MouseButton::Right, f)
}

pub fn on_middle_click<
    'a,
    F: FnOnce(GuiSpatialContext),
>(f: F) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
    on_click(MouseButton::Middle, f)
}

pub fn on_click<'a, F: FnOnce(GuiSpatialContext)>(
    button: MouseButton,
    f: F,
) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
    on_any_click(move |ctx, button2| if button2 == button { f(ctx) })
}

pub fn on_any_click<
    'a,
    F: FnOnce(GuiSpatialContext, MouseButton),
>(f: F) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
    OnClick(f)
}


struct OnClick<F>(F);

impl<
    'a,
    F: FnOnce(GuiSpatialContext, MouseButton),
> GuiNode<'a> for SimpleGuiBlock<OnClick<F>> {
    never_blocks_cursor_impl!();

    fn on_cursor_click(
        self,
        ctx: GuiSpatialContext,
        hits: bool,
        button: MouseButton,
    ) {
        if !hits { return }
        if !ctx.cursor_in_area(0.0, self.size) { return }

        (self.inner.0)(ctx, button)
    }
}

impl<F> Debug for OnClick<F> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("OnClick(..)")
    }
}

