
use crate::gui::{
	context::{
		GuiWindowContext,
		VirtualKeyCode,
	    ScanCode,
	    MouseButton,
	},
    event::ScrolledAmount,
	node::{
		GuiVisitorTarget,
		GuiVisitor,
	},
};
use vek::*;


/// Item within a stack of GUI states. Doesn't generally share the window.
pub trait GuiStateFrame {
    /// Size, position, and visit all GUI nodes. This will generally be
    /// followed by invoking some positional handler on the nodes.
    fn visit_nodes<'a, T: GuiVisitorTarget<'a>>(
        &'a mut self,
        ctx: GuiWindowContext,
        visitor: GuiVisitor<T>,
    )
    where
        Self: Sized;

    /// Called upon the window's focus level changing, except when caused by
    /// the window explicitly asking for that to happen.
    fn on_focus_change(&mut self, ctx: &GuiWindowContext);

    /// Called upon a key being pressed, if window focused.
    ///
    /// The context's `pressed_keys` sets are empty when the window is
    /// unfocused, but "virtual" `on_key_press` calls will _not_ be made when
    /// the window comes into focus. This is usually not a problem.
    ///
    /// Context guarantees:
    /// - `pressed_keys` sets will contain these respective key identifiers.
    /// - `focus_level` >= `Focused`.
    fn on_key_press(
        &mut self,
        ctx: &GuiWindowContext,
        key_semantic: Option<VirtualKeyCode>,
        key_physical: ScanCode,
    );

    /// Called upon a key being released, if window focused.
    ///
    /// The context's `pressed_keys` sets are empty when the window is
    /// unfocused, but "virtual" `on_key_release` calls will _not_ be made when
    /// the window goes out of focus. **This means that, when one is putting
    /// logic in `on_key_release`, they often should also put logic in
    /// `on_focus_change` to handle "cancellations."**
    ///
    /// Context guarantees:
    /// - `pressed_keys` sets will not contain these respective key identifiers.
    /// - `focus_level` >= `Focused`.
    fn on_key_release(
        &mut self,
        ctx: &GuiWindowContext,
        key_semantic: Option<VirtualKeyCode>,
        key_physical: ScanCode,
    );

    /// Called upon a _captured_ mouse button being pressed down. See `GuiNode`
    /// for positional cursor clicks.
    ///
    /// As in `on_key_press`, "virtual" calls will not be made upon window
    /// gaining focus.
    ///
    /// Context guarantees:
    /// - `presed_mouse_buttons` contains `button`.
    /// - `focus_level` == `MouseCaptured`.
    fn on_captured_mouse_click(
        &mut self,
        ctx: &GuiWindowContext,
        button: MouseButton,
    );

    /// Called upon a _captured_ mouse button being released. See `GuiNode` for
    /// positional cursor unclicks.
    ///
    /// As in `on_key_release`, "virtual" calls will not be made upon a window
    /// losing mouse capture. **This means that, if one is putting logic in
    /// `on_captured_mouse_release`, they often should also put logic in
    /// `on_focus_change` to handle "cancellations."**
    ///
    /// Context guarantees:
    /// - `pressed_mouse_buttons` does not contain `button`.
    /// - `focus_level` == `MouseCaptured`.
    fn on_captured_mouse_release(
        &mut self,
        ctx: &GuiWindowContext,
        button: MouseButton,
    );

    /// Called upon a _captured_ mouse being moved. See `GuiNode` for
    /// positional cursor movements.
    ///
    /// Context guarantees:
    /// - `focus_level` == `MouseCaptured`.
    fn on_captured_mouse_move(
        &mut self,
        ctx: &GuiWindowContext,
        amount: Vec2<f32>,
    );

    /// Called upon a _captured_ mouse scrolling. See `GuiNode` for positional
    /// cursor scrolling.
    ///
    /// Context guarantees:
    /// - `focus_level` == `MouseCaptured`.
    fn on_captured_mouse_scroll(
        &mut self,
        ctx: &GuiWindowContext,
        amount: ScrolledAmount,
    );

    /// Called upon, uh, character input to the window.
    fn on_character_input(&mut self, ctx: &GuiWindowContext, c: char);
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
