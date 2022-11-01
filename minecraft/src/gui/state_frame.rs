
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


// ==== state frame ====


/// Item within a stack of GUI states. Doesn't generally share the window.
pub trait GuiStateFrame {
    /// Size, position, and visit all GUI nodes. This will generally be
    /// followed by invoking some positional handler on the nodes.
    fn visit_nodes<'a, T: GuiVisitorTarget<'a>>(
        &'a mut self,
        ctx: &GuiWindowContext,
        visitor: GuiVisitor<T>,
    )
    where
        Self: Sized;

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

    #[allow(unused_variables)]
    fn on_key_press_semantic(
        &mut self,
        ctx: &GuiWindowContext,
        key: VirtualKeyCode,
    ) {}

    #[allow(unused_variables)]
    fn on_key_release_semantic(
        &mut self,
        ctx: &GuiWindowContext,
        key: VirtualKeyCode,
    ) {}

    #[allow(unused_variables)]
    fn on_key_press_physical(
        &mut self,
        ctx: &GuiWindowContext,
        key: ScanCode,
    ) {}

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


// ==== state frame obj ====

/*
pub trait GuiStateFrameObj {
    fn on_cursor_move(&mut self, ctx: &GuiWindowContext);

    fn on_cursor_click(
        &mut self,
        ctx: &GuiWindowContext,
        button: MouseButton,
    );

    fn on_cursor_unclick(
        &mut self,
        ctx: &GuiWindowContext,
        button: MouseButton,
    );

    fn on_cursor_scroll(
        &mut self,
        ctx: &GuiWindowContext,
        amount: ScrolledAmount,
    );

    fn draw<'a>(
        &'a mut self,
        ctx: &GuiWindowContext,
        canvas: &mut Canvas2<'a, '_>,
    );
}

struct ContextStack<'c> {
    top: StackElem<'c>,
    under: Vec<StackElem<'c>>,
}

#[derive(Copy, Clone)]
struct StackElem<'c> {
    ctx: GuiSpatialContext<'c>,
    unclipped: bool,
}

impl<'c> StackElem<'c> {
    fn relativize(&mut self, modifier: Modifier2) {
        match modifier {
            Modifier2::Transform(transform) => self.ctx.relativize(transform),
            Modifier2::Color(_) => (),
            Modifier2::Clip(clip) => {
                if let Some(pos) = self.ctx.cursor_pos {
                    self.unclipped = self.unclipped && clip.test(pos);
                }
            }
        }
    }
}

impl<'c> ContextStack<'c> {
    fn new(ctx: GuiSpatialContext<'c>) -> Self {
        ContextStack {
            top: StackElem {
                ctx,
                unclipped: true,
            },
            under: Vec::new(),
        }
    }

    fn set_stack_len(&mut self, stack_len: usize) {
        assert!(self.under.len() <= stack_len, "stack_len too high");
        while self.under.len() > stack_len {
            self.top = self.under.pop().unwrap();
        }
    }

    fn push_modifier(&mut self, stack_len: usize, modifier: Modifier2) {
        self.set_stack_len(stack_len);
        self.under.push(self.top);
        self.top.relativize(modifier);
    }
}

impl<T: GuiStateFrame> GuiStateFrameObj for T {
    fn on_cursor_move(&mut self, ctx: &GuiWindowContext) {
        struct T<'c> {
            stack: ContextStack<'c>,
            blocked: bool,
        }

        impl<'a, 'c> GuiVisitorTarget<'a> for T<'c> {
            fn push_modifier(&mut self, stack_len: usize, modifier: Modifier2) {
                self.stack.push_modifier(stack_len, modifier);
            }

            fn visit_node<I: GuiNode<'a>>(&mut self, stack_len: usize, node: I) {
                self.stack.set_stack_len(stack_len);
                
                let ctx = self.stack.top.ctx;
                let hits = !self.blocked && self.stack.top.unclipped;

                if let Some(pos) = ctx.cursor_pos {
                    self.blocked = self.blocked || node.blocks_cursor(ctx, pos);
                }

                node.on_cursor_move(ctx, hits);
            }
        }
    }

    fn on_cursor_click(
        &mut self,
        ctx: &GuiWindowContext,
        button: MouseButton,
    ) {

    }

    fn on_cursor_unclick(
        &mut self,
        ctx: &GuiWindowContext,
        button: MouseButton,
    );

    fn on_cursor_scroll(
        &mut self,
        ctx: &GuiWindowContext,
        amount: ScrolledAmount,
    );

    fn draw<'a>(
        &'a mut self,
        ctx: &GuiWindowContext,
        canvas: &mut Canvas2<'a, '_>,
    );
}


struct StackHelper<'a> {
    top: Option<StackElem<'a>>,
    under: Vec<Option<StackElem<'a>>>,
}

#[derive(Copy, Clone)]
struct StackElem<'a> {
    ctx: GuiSpatialContext<'a>,
    pos: Vec2<f32>,
    unclipped: bool,
}

impl<'a> StackHelper<'a> {
    fn new(ctx: &GuiSpatialContext<'a>, pos: Vec2<f32>) -> Self {
        StackHelper {
            top: Some(StackElem {
                ctx: *ctx,
                pos,
                unclipped: true,
            }),
            under: Vec::new(),
        }
    }

    fn set_stack_len(&mut self, stack_len: usize) {
        assert!(self.under.len() <= stack_len);
        while self.under.len() > stack_len {
            self.top = self.under.pop().unwrap();
        }
    }

    fn push_modifier(&mut self, stack_len: usize, modifier: Modifier2) {
        self.set_stack_len(stack_len);

        let new_top = self.top.and_then(|elem| elem.modified(modifier));
        self.under.push(replace(&mut self.top, new_top));
    }

    fn top_pos(&self)
}

impl<'a> StackElem<'a> {
    fn modified(mut self, modifier: Modifier2) -> Option<Self> {
        match modifier {
            Modifier2::Transform(transform) => {
                let reversed = transform.reverse()?;

                self.ctx.cursor_pos = self.ctx.cursor_pos
                    .map(|pos| reversed.apply(pos));
                self.pos = reversed.apply(self.pos);

                Some(self)
            }
            Modifier2::Color(_) => Some(self),
            Modifier2::Clip(clip) => {
                self.unclipped &= clip.test(self.pos);
                Some(self)
            }
        }
    }
}

impl<T: GuiStateFrame> GuiStateFrameObj for T {
    fn on_cursor_move(&mut self, ctx: &GuiWindowContext, pos: Vec2<f32>)
    {
        struct T<'a> {
            helper: StackHelper<'a>,
            blocked: bool,
        }

        impl<'a> GuiVisitorTarget<'a> for T<'a> {
            fn push_modifier(&mut self, stack_len: usize, modifier: Modifier2) {
                self.helper.push_modifier(stack_len, modifier);
            }

            fn visit_node<I: GuiNode<'a>>(&mut self, stack_len: usize, node: I) {
                self.helper.set_stack_len(stack_len);
                if let Some(top) = self.helper.top {
                    
                    let blocks = node.blocks_cursor(self.helper.) 
                }
            }
        }
    }

    fn on_cursor_click(
        &mut self,
        ctx: &GuiWindowContext,
        button: MouseButton,
        pos: Vec2<f32>,
    ) {

    }

    fn on_cursor_unclick(
        &mut self,
        ctx: &GuiWindowContext,
        button: MouseButton,
        pos: Vec2<f32>,
    ) {

    }

    fn on_cursor_scroll(
        &mut self,
        ctx: &GuiWindowContext,
        amount: ScrolledAmount,
        pos: Vec2<f32>,
    ) {

    }

    fn draw<'a>(
        &'a mut self,
        ctx: &GuiWindowContext,
        canvas: &mut Canvas2<'a, '_>,
    ) {

    }
}
*/