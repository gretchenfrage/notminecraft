
use crate::gui::{
	context::{
		GuiWindowContext,
	    MouseButton,
        PhysicalKey
	},
    event::{
        ScrolledAmount,
        TypingInput,
    },
	node::{
		GuiVisitorTarget,
		GuiVisitor,
	},
    gui_event_loop::GuiUserEventNotify,
};
use std::{
    fmt::Debug,
    time::Instant,
};
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

    /// The gui should enter a loop of polling for and processing user events.
    ///
    /// If this gui does not have a concept of user events it can ignore this.
    ///
    /// This exists to facilitate the pattern of a gui having channels from
    /// which it asynchronously receives and processes events from other
    /// threads. When this method is called, the implementor should enter a
    /// loop to poll for and process these events until either no event can be
    /// retrieved without blocking or until `Instant::now()` reaches `stop_at`.
    ///
    /// If the loop terminates because no more events can be found without
    /// blocking, `notify` may be hooked up to the channel to tell the event
    /// loop when more events may be available. This is useful because it is
    /// uniquely capable of waking up the event loop while it is sleeping.
    ///
    /// To prevent race conditions when doing so, it may be necessary to poll
    /// once more _after_ hooking up notify as a callback.
    ///
    /// In addition to this method being called in response to `notify` being
    /// used, it is also always called immediately after drawing a frame. This
    /// makes this system nicely "self-healing" from states where the notifier
    /// may not have gotten hooked properly, such as during gui state
    /// transitions.
    ///
    /// The gui event loop chooses a `stop_at` value to maintain performance
    /// for both drawing frames and processing user events and to prevent
    /// either from starving the other.
    #[allow(unused_variables)]
    fn poll_user_events(
        &mut self,
        ctx: &GuiWindowContext,
        stop_at: Instant,
        notify: &GuiUserEventNotify,
    ) {}

    /// Called upon the window's focus level changing, except when caused by
    /// the window explicitly asking for that to happen.
    #[allow(unused_variables)]
    fn on_focus_change(&mut self, ctx: &GuiWindowContext) {}

    /// Called upon a key being pressed, if window focused.
    ///
    /// The context's `pressed_keys` set is empty when the window is unfocused,
    /// but "virtual" `on_key_press` calls will _not_ be made when the window
    /// comes into focus. This is usually not a problem.
    ///
    /// Context guarantees:
    /// - `pressed_keys` will contain `key`.
    /// - `focus_level` >= `Focused`.
    #[allow(unused_variables)]
    fn on_key_press(
        &mut self,
        ctx: &GuiWindowContext,
        key: PhysicalKey,
        typing: Option<TypingInput>,
    ) {}

    /// Called upon a key being released, if window focused.
    ///
    /// The context's `pressed_keys` set is empty when the window is unfocused,
    /// but "virtual" `on_key_release` calls will _not_ be made when the window
    /// goes out of focus. **This means that, when one is putting logic in
    /// `on_key_release`, they often should also put logic in `on_focus_change`
    /// to handle "cancellations."**
    ///
    /// Context guarantees:
    /// - `pressed_keys` will not contain `key`.
    /// - `focus_level` >= `Focused`.
    #[allow(unused_variables)]
    fn on_key_release(
        &mut self,
        ctx: &GuiWindowContext,
        key: PhysicalKey,
    ) {}

    /// Called upon a _captured_ mouse button being pressed down. See `GuiNode`
    /// for positional cursor clicks.
    ///
    /// As in `on_key_press`, "virtual" calls will not be made upon window
    /// gaining focus.
    ///
    /// Context guarantees:
    /// - `pressed_mouse_buttons` contains `button`.
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

    
    /// The gui should process any internal gui effect queue-like things it has.
    ///
    /// If this gui does not have a concept of such things it can ignore this.
    ///
    /// A common lifetime problem in gui systems like these is when a gui component needs to
    /// trigger side effects which conflict with borrows which exist at the time of those effects
    /// being triggered. For example, a menu may contain a button that closes it, but the logic for
    /// that button being clicked involves it being passed a reference to itself, which would be
    /// invalidated if the menu were to be moved or dropped then and there.
    ///
    /// A pattern to solve this is for these gui components to instead write a description of
    /// effects to be actuated to some queue, and then process the queue once any conflicting
    /// borrows have ended. The gui event loop's effect queue is an example of this pattern on the
    /// level of changing gui states. This method exists to facilitate analogous patterns within a
    /// gui state.
    ///
    /// As such, this method is called after every gui state other method is called, alongside the
    /// event loop processing its own gui effect queue.
    ///
    /// Notably, when `visit_nodes` is called, it can return values that borrow `&mut self`. The
    /// gui event loop will call this method after that borrow ends, allowing patterns that would
    /// otherwise be fraught to implement without this method.
    #[allow(unused_variables)]
    fn process_gui_effects(&mut self, ctx: &GuiWindowContext) {}
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
