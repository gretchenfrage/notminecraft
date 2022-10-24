
use crate::gui::{
	context::{
		GuiContext,
		VirtualKeyCode,
	    ScanCode,
	    MouseButton,
	    ScrolledAmount,
	},
	node::{
		GuiVisitorTarget,
		GuiVisitor,
	},
};
use vek::*;


pub trait GuiStateFrame {
    fn visit_nodes<'a, T: GuiVisitorTarget<'a>>(
        &'a mut self,
        ctx: GuiContext,
        visitor: GuiVisitor<'_, T>,
    )
    where
        Self: Sized;

    fn on_focus_change(&mut self, ctx: GuiContext);

    fn on_key_press(&mut self, ctx: GuiContext, key_semantic: Option<VirtualKeyCode>, key_physical: ScanCode);

    fn on_key_release(&mut self, ctx: GuiContext, key_semantic: Option<VirtualKeyCode>, key_physical: ScanCode);

    fn on_captured_mouse_click(&mut self, ctx: GuiContext, button: MouseButton);

    fn on_captured_mouse_move(&mut self, ctx: GuiContext, amount: Vec2<f32>);

    fn on_captured_mouse_scroll(&mut self, ctx: GuiContext, amount: ScrolledAmount);

    fn on_character_input(&mut self, ctx: GuiContext, c: char);
}

/*
pub trait GuiStateFrameObj {
    fn on_cursor_move(&mut self, ctx: GuiContext, pos: Vec2<f32>);

    fn on_cursor_click(
        &mut self,
        ctx: GuiContext,
        button: MouseButton,
        pos: Vec2<f32>,
    );

    fn on_cursor_unclick(
        &mut self,
        ctx: GuiContext,
        button: MouseButton,
        pos: Vec2<f32>,
    );

    fn on_cursor_scroll(
        &mut self,
        ctx: GuiContext,
        amount: ScrolledAmount,
        pos: Vec2<f32>,
    );
}

impl<T: GuiStateFrame> GuiStateFrameObj for T {

}
*/
