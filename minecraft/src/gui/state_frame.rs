
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
use std::fmt::Debug;
use vek::*;


// ==== state frame ====


/// Item within a stack of GUI states. Doesn't generally share the window.
pub trait GuiStateFrame: Debug {
    /// Size, position, and visit all GUI nodes. This will generally be
    /// followed by invoking some positional handler on the nodes.
    ///
    /// The recommended way to implement this method is like this:
    ///
    /// ```
    /// impl MyGuiStateFrame {
    ///     fn gui<'a>(
    ///         &'a mut self,
    ///         ctx: &'a GuiWindowContext,
    ///     ) -> impl GuiBlock<'a, DimParentSets, DimParentSets>
    ///     { ... }
    /// }
    ///
    /// impl GuiStateFrame for MyGuiStateFrame {
    ///     impl_visit_nodes!();
    ///
    ///     ...
    /// }
    /// ```
    fn visit_nodes<'a, T: GuiVisitorTarget<'a>>(
        &'a mut self,
        ctx: &'a GuiWindowContext<'a>,
        visitor: GuiVisitor<'a, '_, T>,
        forward: bool,
    )
    where
        Self: Sized;

    /// Called immediately before drawing, with `elapsed` being the number of
    /// seconds since this was last called.
    ///
    /// The exception to this is that the first draw call will not be preceded
    /// by a call to `update`.
    #[allow(unused_variables)]
    fn update(&mut self, ctx: &GuiWindowContext, elapsed: f32) {}

    /// Called upon the window's focus level changing, except when caused by
    /// the window explicitly asking for that to happen.
    #[allow(unused_variables)]
    fn on_focus_change(&mut self, ctx: &GuiWindowContext) {}

    /*
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
    */

    /// Called upon a key being pressed, if window focused.
    ///
    /// The context's `pressed_keys` sets are empty when the window is
    /// unfocused, but "virtual" `on_key_press` calls will _not_ be made when
    /// the window comes into focus. This is usually not a problem.
    ///
    /// Context guarantees:
    /// - `pressed_keys_semantic` will contain `key`.
    /// - `focus_level` >= `Focused`.
    #[allow(unused_variables)]
    fn on_key_press_semantic(
        &mut self,
        ctx: &GuiWindowContext,
        key: VirtualKeyCode,
    ) {}

    /// Called upon a key being released, if window focused.
    ///
    /// The context's `pressed_keys` sets are empty when the window is
    /// unfocused, but "virtual" `on_key_release` calls will _not_ be made when
    /// the window goes out of focus. **This means that, when one is putting
    /// logic in `on_key_release`, they often should also put logic in
    /// `on_focus_change` to handle "cancellations."**
    ///
    /// Context guarantees:
    /// - `pressed_keys_semantic` will not contain `key`.
    /// - `focus_level` >= `Focused`.
    #[allow(unused_variables)]
    fn on_key_release_semantic(
        &mut self,
        ctx: &GuiWindowContext,
        key: VirtualKeyCode,
    ) {}

    /// Called upon a key being pressed, if window focused.
    ///
    /// The context's `pressed_keys` sets are empty when the window is
    /// unfocused, but "virtual" `on_key_press` calls will _not_ be made when
    /// the window comes into focus. This is usually not a problem.
    ///
    /// Context guarantees:
    /// - `pressed_keys_physical` will contain `key`
    /// - `focus_level` >= `Focused`.
    #[allow(unused_variables)]
    fn on_key_press_physical(
        &mut self,
        ctx: &GuiWindowContext,
        key: ScanCode,
    ) {}

    /// Called upon a key being released, if window focused.
    ///
    /// The context's `pressed_keys` sets are empty when the window is
    /// unfocused, but "virtual" `on_key_release` calls will _not_ be made when
    /// the window goes out of focus. **This means that, when one is putting
    /// logic in `on_key_release`, they often should also put logic in
    /// `on_focus_change` to handle "cancellations."**
    ///
    /// Context guarantees:
    /// - `pressed_keys_physical` will not contain `key`.
    /// - `focus_level` >= `Focused`.
    #[allow(unused_variables)]
    fn on_key_release_physical(
        &mut self,
        ctx: &GuiWindowContext,
        key: ScanCode,
    ) {}

    /// Called upon a _captured_ mouse button being pressed down. See `GuiNode`
    /// for positional cursor clicks.
    ///
    /// As in `on_key_press`, "virtual" calls will not be made upon window
    /// gaining focus.
    ///
    /// Context guarantees:
    /// - `presed_mouse_buttons` contains `button`.
    /// - `focus_level` == `MouseCaptured`.
    #[allow(unused_variables)]
    fn on_captured_mouse_click(
        &mut self,
        ctx: &GuiWindowContext,
        button: MouseButton,
    ) {}

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
    #[allow(unused_variables)]
    fn on_captured_mouse_release(
        &mut self,
        ctx: &GuiWindowContext,
        button: MouseButton,
    ) {}

    /// Called upon a _captured_ mouse being moved. See `GuiNode` for
    /// positional cursor movements.
    ///
    /// Context guarantees:
    /// - `focus_level` == `MouseCaptured`.
    #[allow(unused_variables)]
    fn on_captured_mouse_move(
        &mut self,
        ctx: &GuiWindowContext,
        amount: Vec2<f32>,
    ) {}

    /// Called upon a _captured_ mouse scrolling. See `GuiNode` for positional
    /// cursor scrolling.
    ///
    /// Context guarantees:
    /// - `focus_level` == `MouseCaptured`.
    #[allow(unused_variables)]
    fn on_captured_mouse_scroll(
        &mut self,
        ctx: &GuiWindowContext,
        amount: ScrolledAmount,
    ) {}

    /// Called upon, uh, character input to the window.
    #[allow(unused_variables)]
    fn on_character_input(&mut self, ctx: &GuiWindowContext, c: char) {}
}


#[macro_export]
macro_rules! impl_visit_nodes {
    ()=>{
        fn visit_nodes<'a, T: GuiVisitorTarget<'a>>(
            &'a mut self,
            ctx: &'a GuiWindowContext<'a>,
            mut visitor: GuiVisitor<'a, '_, T>,
            forward: bool,
        ) {
            let ((), (), sized) = self
                .gui(ctx)
                .size(
                    ctx.spatial.global,
                    ctx.size.w as f32,
                    ctx.size.h as f32,
                    ctx.scale,
                );
            sized.visit_nodes(&mut visitor, forward);
        }
    };
}

pub use impl_visit_nodes;
