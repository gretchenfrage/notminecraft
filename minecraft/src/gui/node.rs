

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
use vek::*;


/// Positionable unit of GUI behavior. Expected to have its sizing data already
/// bound into it.
pub trait GuiNode<'a>: Sized {
    /// Determine whether this node "blocks" a cursor event at the given
    /// position. The event will still be passed to nodes underneath this one,
    /// but their `hits` argument will be false.
    fn blocks_cursor(&self, pos: Vec2<f32>) -> bool;

    /// Called upon the cursor being moved to a new position, if the cursor
    /// exists in this space, whether or not window is focused. Position
    /// relativized to this space.
    ///
    /// Context guarantees:
    /// - `cursor_pos` == `Some(pos)`.
    #[allow(unused_variables)]
    fn on_cursor_move(self, ctx: GuiSpatialContext, pos: Vec2<f32>) {}

    /// Called upon a mouse button being pressed, if the cursor exists in this
    /// space and the window is focused. Position is relativized to this space.
    ///
    /// As in `on_key_press`, "virtual" calls will not be made upon a window
    /// gaining focus.
    ///
    /// See `blocks_cursor` regarding `hits`.
    ///
    /// Context guarantees:
    /// - `pressed_mouse_buttons` contains `button`.
    /// - `cursor_pos` == `Some(pos)`.
    /// - `focus_level` == `Focused`.
    #[allow(unused_variables)]
    fn on_cursor_click(
        self,
        ctx: GuiSpatialContext,
        button: MouseButton,
        pos: Vec2<f32>,
        hits: bool,
    ) {}

    /// Called upon a mouse button being released, if the cursor exists in
    /// this space and the window is focused. Position is relativized to this
    /// space.
    ///
    /// As in `on_key_release`, "virtual" calls will not be made upon a window
    /// losing focus. **This means that, if one is putting logic in
    /// `on_cursor_unclick`, they often should also put logic in
    /// `on_focus_change` to handle "cancellations."**
    ///
    /// See `blocks_cursor` regarding `hits`.
    ///
    /// Context guarantees:
    /// - `pressed_mouse_button` does not contain `button`.
    /// - `cursor_pos` == `Some(pos)`.
    /// - `focus_level` == `Focused`.
    #[allow(unused_variables)]
    fn on_cursor_unclick(
        self,
        ctx: GuiSpatialContext,
        button: MouseButton,
        pos: Vec2<f32>,
        hits: bool,
    ) {}

    /// Called upon a mouse scrolling, if the cursor exists in this space,
    /// whether or not the window is focused. Position is relativized to this
    /// space.
    ///
    /// I hope the OS filters out these events if it's blocked by another
    /// window.
    ///
    /// See `blocks_cursor` regarding `hits`.
    ///
    /// Context guarantees:
    /// - `cursor_pos` == `Some(pos)`.
    #[allow(unused_variables)]
    fn on_cursor_scroll(
        self,
        ctx: GuiSpatialContext,
        amount: ScrolledAmount,
        pos: Vec2<f32>,
        hits: bool,
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
}

