
use crate::gui::node::GuiNode;
use graphics::modifier::{
    Modifier2,
    Transform2,
    Clip2,
};
use vek::*;


pub trait GuiVisitorTarget<'a> {
    fn push_modifier(&mut self, stack_len: usize, modifier: Modifier2);

    fn visit_node<I: GuiNode<'a>>(&mut self, stack_len: usize, node: I);
}

pub struct GuiVisitor<'b, 'c, T> {
    pub target: &'b mut T,
    pub stack_len: usize,
    pub ctx: GuiContext<'c>,
}

impl<'a, 'b, 'c, T: GuiVisitorTarget<'a>> GuiVisitor<'b, 'c, T> {
    /*pub fn new(target: &'b mut T) -> Self {
        GuiVisitor {
            target,
            stack_len: 0,
        }
    }*/

    pub fn reborrow<'b2>(&'b2 mut self) -> GuiVisitor<'b2, 'c, T> {
        GuiVisitor {
            target: self.target,
            stack_len: self.stack_len,
            ctx: self.ctx,
        }
    }

    pub fn modify<I: Into<Modifier2>>(mut self, modifier: I) -> Self {
        self.target.push_modifier(self.stack_len, modifier.into());
        self.stack_len += 1;
        todo!("update ctx");
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
