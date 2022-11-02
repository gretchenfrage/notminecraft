

use crate::gui::{
    context::{
    	GuiSpatialContext,
        MouseButton,
    },
    event::ScrolledAmount,
};
use graphics::{
    frame_content::Canvas2,
    modifier::{
        Modifier2,
        Transform2,
        Clip2,
    },
};
use std::{
    borrow::Cow,
    fmt::Debug,
};
use vek::*;


/// Positionable unit of GUI behavior. Expected to have its sizing data already
/// bound into it.
pub trait GuiNode<'a>: Sized {
    /// Determine whether this node "blocks" a cursor event at the given
    /// position. The event will still be passed to nodes underneath this one,
    /// but their `hits` argument will be false.
    ///
    /// Handlers' `hits` arguments will also be passed as false if that
    /// position is clipped out for this node.
    fn blocks_cursor(&self, ctx: GuiSpatialContext, pos: Vec2<f32>) -> bool;

    /// Called upon the cursor being moved to a new position, whether or not
    /// the window is focused.
    ///
    /// See `StateFrame::on_captured_mouse_move` for mouse-captured equivalent.
    ///
    /// See `blocks_cursor` regarding `hits`.
    ///
    /// Context guarantees:
    /// - `focus_level` < `MouseCaptured`
    #[allow(unused_variables)]
    fn on_cursor_move(self, ctx: GuiSpatialContext, hits: bool) {}

    /// Called upon a non-captured mouse button being pressed, if the window is
    /// focused.
    ///
    /// As in `on_key_press`, "virtual" calls will not be made upon a window
    /// gaining focus.
    ///
    /// See `blocks_cursor` regarding `hits`.
    ///
    /// See `StateFrame::on_captured_mouse_click` for mouse-captured
    /// equivalent.
    ///
    /// Context guarantees:
    /// - `pressed_mouse_buttons` contains `button`.
    /// - `focus_level` == `Focused`.
    #[allow(unused_variables)]
    fn on_cursor_click(
        self,
        ctx: GuiSpatialContext,
        hits: bool,
        button: MouseButton,
    ) {}

    /// Called upon a non-captured mouse button being released, if the window
    /// is focused.
    ///
    /// As in `on_key_release`, "virtual" calls will not be made upon a window
    /// losing focus. **This means that, if one is putting logic in
    /// `on_cursor_release`, they often should also put logic in
    /// `on_focus_change` to handle "cancellations."**
    ///
    /// See `blocks_cursor` regarding `hits`.
    ///
    /// See `StateFrame::on_captured_mouse_release` for mouse-captured
    /// equivalent.
    ///
    /// Context guarantees:
    /// - `pressed_mouse_button` does not contain `button`.
    /// - `focus_level` == `Focused`.
    #[allow(unused_variables)]
    fn on_cursor_release(
        self,
        ctx: GuiSpatialContext,
        hits: bool,
        button: MouseButton,
    ) {}

    /// Called upon non-captured mouse scrolling, whether or not the window is
    /// focused.
    ///
    /// I hope the OS filters out these events if it's blocked by another
    /// window.
    ///
    /// See `blocks_cursor` regarding `hits`.
    ///
    /// See `StateFrame::on_captured_mouse_scroll` for mosue-captured
    /// equivalent.
    ///
    /// Context guarantees:
    /// - `focus_level` < `MouseCaptured`.
    #[allow(unused_variables)]
    fn on_cursor_scroll(
        self,
        ctx: GuiSpatialContext,
        hits: bool,
        amount: ScrolledAmount,
    ) {}

    /// Called to request that node draw to `canvas`. Canvas is relativized
    /// to this space.
    #[allow(unused_variables)]
    fn draw(self, ctx: GuiSpatialContext, canvas: &mut Canvas2<'a, '_>) {}
}


/// Behavior backing a `GuiVisitor`.
pub trait GuiVisitorTarget<'a> {
    fn push_modifier(&mut self, stack_len: usize, modifier: Modifier2);

    fn visit_node<I: GuiNode<'a>>(&mut self, stack_len: usize, node: I);

    fn push_debug_tag(&mut self, stack_len: usize, tag: Cow<'static, str>);
}

/// Canvas-like visitor for GUI nodes nested within modifiers. Keeps a
/// `GuiSpatialContext` updated as transforms are applied, which may be
/// read.
pub struct GuiVisitor<'b, T> {
    pub target: &'b mut T,
    pub stack_len: usize,
    pub ctx: GuiSpatialContext<'b>,
}

impl<'a, 'b, T: GuiVisitorTarget<'a>> GuiVisitor<'b, T> {
    pub fn new(target: &'b mut T, ctx: GuiSpatialContext<'b>) -> Self {
        GuiVisitor {
            target,
            stack_len: 0,
            ctx,
        }
    }

    pub fn reborrow<'b2>(&'b2 mut self) -> GuiVisitor<'b2, T> {
        GuiVisitor {
            target: self.target,
            stack_len: self.stack_len,
            ctx: self.ctx,
        }
    }

    pub fn modify<I: Into<Modifier2>>(mut self, modifier: I) -> Self {
        let modifier = modifier.into();

        self.target.push_modifier(self.stack_len, modifier);
        self.stack_len += 1;

        if let Modifier2::Transform(transform) = modifier {
            self.ctx.relativize(transform);
        }

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

    /// Generally, this will immediately call the relevant callback on `node`,
    /// passing `self.ctx` as `ctx`.
    pub fn visit_node<I: GuiNode<'a>>(self, node: I) -> Self {
        self.target.visit_node(self.stack_len, node);
        self
    }

    pub fn debug_tag<I: Into<Cow<'static, str>>>(self, tag: I) -> Self {
        self.target.push_debug_tag(self.stack_len, tag.into());
        self
    }
}

