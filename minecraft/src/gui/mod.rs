
use graphics::{
    Renderer,
    modifier::{
        Modifier2,
        Transform2,
        Clip2,
    },
    frame_content::Canvas2,
};
use std::collections::BTreeSet;
use vek::*;


pub mod block;


pub use winit_main::reexports::event::{
    VirtualKeyCode,
    ScanCode,
    MouseButton,
};


#[derive(Debug, Copy, Clone)]
pub struct GuiGlobalContext<'c> {
    pub renderer: &'c Renderer,
    pub focus_level: FocusLevel,
    pub pressed_keys_semantic: &'c BTreeSet<VirtualKeyCode>,
    pub pressed_keys_physical: &'c BTreeSet<ScanCode>,
    pub pressed_mouse_buttons: &'c BTreeSet<MouseButton>,
}

#[derive(Debug, Copy, Clone)]
pub struct GuiContext<'c> {
    pub global: &'c GuiGlobalContext<'c>,
    pub cursor_pos: Option<Vec2<f32>>,
    pub size: Extent2<f32>,
    pub scale: f32,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum FocusLevel {
    Unfocused,
    Focused,
    MouseCaptured,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ScrolledAmount {
    Pixels(Vec2<f32>),
    Lines(Vec2<f32>),
}

impl ScrolledAmount {
    pub fn to_pixels(self, font_size: impl Into<Extent2<f32>>) -> Vec2<f32> {
        match self {
            ScrolledAmount::Pixels(v) => v,
            ScrolledAmount::Lines(l) => l * font_size.into(),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BlocksCursor {
    Blocks,
    DoesntBlock,
}


pub trait GuiNode<'a> {
    fn on_cursor_move(self, ctx: GuiContext, pos: Vec2<f32>);

    fn on_cursor_click(
        self,
        ctx: GuiContext,
        button: MouseButton,
        pos: Vec2<f32>,
        hits: bool,
    ) -> BlocksCursor;

    fn on_cursor_unclick(
        self,
        ctx: GuiContext,
        button: MouseButton,
        pos: Vec2<f32>,
        hits: bool,
    ) -> BlocksCursor;

    fn on_cursor_scroll(
        self,
        ctx: GuiContext,
        amount: ScrolledAmount,
        pos: Vec2<f32>,
        hits: bool,
    ) -> BlocksCursor;
}


pub trait GuiStateFrame {
    /*fn visit_nodes<'a, T: GuiVisitorTarget<'a>>(
        &'a mut self,
        ctx: GuiContext,
        visitor: GuiVisitor<'_, T>,
    )
    where
        Self: Sized;*/

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
/*
#[allow(unused_variables)]
pub trait GuiNode<'a>: Sized {
    /// Whether this node makes a cursor at the given position, in this node's
    /// space, be considered clipped from the nodes beneath this one.
    fn clips_cursor(&self, pos: Vec2<f32>) -> bool;

    /// Draw to the canvas.
    fn draw(self, ctx: &GuiContext, canvas: Canvas2<'a, '_>) {}

    fn on_cursor_press(self, ctx: &GuiContext, button: MouseButton, pos: Vec2<f32>) {}

    fn on_cursor_release(self, ctx: &GuiContext, button: MouseButton, pos: Vec2<f32>) {}

    fn on_cursor_scroll(self, ctx: &GuiContext, amount: ScrolledAmount, pos: Vec2<f32>) {}

    fn on_cursor_change(self, ctx: &GuiContext) {}
    /*

    /// Called when mouse button is pressed down and:
    /// - window is focused, but not cursor-captured
    /// - cursor pos is not clipped or otherwise modified out of this node's space
    ///
    /// Guaranteed that `ctx.cursor_pos == Some(pos)`.
    ///
    /// For logic regarding something happening during a period where a mouse
    /// button is being depressed, should use `ctx.pressed_mouse_buttons` and
    /// `ctx.focus_level`.
    fn on_cursor_click(self, ctx: &GuiContext, button: MouseButton, pos: Vec2<f32>) -> CursorEventConsumed;

    /// Called when scrolling occurs and _either_:
    /// - cursor pos is over window and not clipped or otherwise modified out of this node's space
    /// - window is cursor-capturedkk
    fn on_cursor_scroll(self, ctx: &GuiContext, amount: ScrolledAmount) -> CursorEventConsumed;*/
}

#[derive(Debug, Clone)]
pub struct GuiContext<'r, 'p> {
    /// The renderer, for loading graphics resources.
    pub renderer: &'r Renderer,

    /// Set of pressed mouse buttons. Updates regardless of focus level.
    pub pressed_mouse_buttons: &'p BTreeSet<MouseButton>,
    /// Set of pressed keys, by semantic identifier. Updates regardless of focus level.
    pub pressed_keys_semantic: &'p BTreeSet<VirtualKeyCode>,
    /// Set of pressed keys, by physical identifier. Updates regardless of focus level.
    pub pressed_keys_physical: &'p BTreeSet<ScanCode>,

    /// Current focus level of node's space.
    pub focus_level: FocusLevel,

    pub cursor: Option<GuiContextCursor>,
    // /// Current position of cursor in node's space, if one currently exists.
    // pub cursor_pos: Option<Vec2<f32>>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct GuiContextCursor {
    pub pos: Vec2<f32>,
    pub unclipped: bool,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum FocusLevel {
    /// Space is not focused.
    ///
    /// Cursor may or may not exist in this space. If one does, cursor input
    /// events will be received. Focus input events will not be received.
    /// Captured mouse events will not be received.
    Unfocused,
    /// Space is focused, but not mouse-captured.
    ///
    /// Cursor may or may not exist in this space. If one does, cursor input
    /// events will be received. Focus input events will be received. Captured
    /// mouse events will not be received.
    Focused,
    /// Space is mouse-captured (focused + cursor is grabbed and hidden).
    ///
    /// Cursor will not exist in this space, so no cursor input events will be
    /// received. Focus input events will be received. Captured mouse events
    /// will be received.
    MouseCaptured,
}

/// Amount of scrolling.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ScrolledAmount {
    /// Scrolling in pixels, such as from a trackpad.
    Pixels(Vec2<f32>),
    /// Scrolling in lines, such as from a traditional mouse wheel.
    Lines(Vec2<f32>),
}

impl ScrolledAmount {
    pub fn to_pixels(self, font_size: impl Into<Extent2<f32>>) -> Vec2<f32> {
        match self {
            ScrolledAmount::Pixels(v) => v,
            ScrolledAmount::Lines(l) => l * font_size.into(),
        }
    }
}
*/

/*
pub trait GuiStateFrame {
    fn size_visit_nodes<'a, T: GuiVisitorTarget<'a>>(
        &'a mut self,
        size: Extent2<f32>,
        scale: f32,
        visitor: GuiVisitor<'_, T>,
    )
    where
        Self: Sized;

    fn on_key_press(&mut self, ctx: &GuiContext, key_semantic: Option<VirtualKeyCode>, key_physical: ScanCode);

    fn on_key_release(&mut self, ctx: &GuiContext, key_semantic: Option<VirtualKeyCode>, key_physical: ScanCode);

    fn on_character_input(&mut self, ctx: &GuiContext, c: char);

    fn on_captured_mouse_click(&mut self, ctx: &GuiContext, button: MouseButton);

    fn on_captured_mouse_move(&mut self, ctx: &GuiContext, amount: Vec2<f32>);

    fn on_captured_mouse_scroll(&mut self, ctx: &GuiContext, amount: ScrolledAmount);
}
*/

pub trait GuiVisitorTarget<'a> {
    fn push_modifier(&mut self, stack_len: usize, modifier: Modifier2);

    fn visit_node<I: GuiNode<'a>>(&mut self, stack_len: usize, node: I);
}

pub struct GuiVisitor<'b, T> {
    pub target: &'b mut T,
    pub stack_len: usize,
}

impl<'a, 'b, T: GuiVisitorTarget<'a>> GuiVisitor<'b, T> {
    pub fn new(target: &'b mut T) -> Self {
        GuiVisitor {
            target,
            stack_len: 0,
        }
    }

    pub fn reborrow<'b2>(&'b2 mut self) -> GuiVisitor<'b2, T> {
        GuiVisitor {
            target: self.target,
            stack_len: self.stack_len,
        }
    }

    pub fn modify<I: Into<Modifier2>>(mut self, modifier: I) -> Self {
        self.target.push_modifier(self.stack_len, modifier.into());
        self.stack_len += 1;
        self
    }

    pub fn translate<V: Into<Vec2<f32>>>(self, v: V) -> Self {
        self.modify(Transform2::translate(v))
    }

    pub fn scale<V: Into<Vec2<f32>>>(self, v: V) -> Self {
        self.modify(Transform2::scale(v))
    }

    pub fn rotate(self, f: f32) -> Self {
        self.modify(Transform2::rotate(f))
    }

    pub fn color<C: Into<Rgba<f32>>>(self, c: C) -> Self {
        self.modify(c.into())
    }

    pub fn min_x(self, f: f32) -> Self {
        self.modify(Clip2::min_x(f))
    }

    pub fn max_x(self, f: f32) -> Self {
        self.modify(Clip2::max_x(f))
    }

    pub fn min_y(self, f: f32) -> Self {
        self.modify(Clip2::min_y(f))
    }

    pub fn max_y(self, f: f32) -> Self {
        self.modify(Clip2::max_y(f))
    }

    pub fn visit_node<I: GuiNode<'a>>(self, node: I) -> Self {
        self.target.visit_node(self.stack_len, node);
        self
    }
}
