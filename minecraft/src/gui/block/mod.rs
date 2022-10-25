/*
use super::{
    context::{
        GuiContext,
        GuiGlobalContext,
        MouseButton,
        ScrolledAmount,
    },
    node::{
        GuiNode,
        GuiVisitor,
        GuiVisitorTarget,
    },
};
use graphics::frame_content::Canvas2;
use std::ops::Index;
use vek::*;

mod blocks;
*/
//mod mc;
/*mod axis_swap;
mod center;
mod cursor_is_over_tracker;
mod layer;
mod margin;
mod modify;
mod stable_unscaled_size;
mod stack;*/
//mod text;
//mod tile_9;
//mod tile_image;


pub mod dim_constraint;
pub mod gui_block;
pub mod gui_block_seq;

/*
pub use blocks::{
    axis_swap::{
        axis_swap,
        axis_swap_seq,
    },
    center::{h_center, v_center},
    cursor_is_over_tracker::cursor_is_over_tracker,
    layer::layer,
    margin::{h_margin, v_margin},
    modify::modify,
    stable_unscaled_size::{h_stable_unscaled_size, v_stable_unscaled_size},
    stack::{v_stack, h_stack},
};
*/

// ==== dim constraint definition ====





// ==== gui block and sized gui block ====




/*
impl<'a, N: GuiNode<'a>> SizedGuiBlock<'a> for N {
    fn visit_nodes<T: GuiVisitorTarget<'a>>(self, visitor: GuiVisitor<'_, T>) {
        visitor.visit_node(self);
    }
}
*/

// ==== "simple" ((sized) block) / node utility ====

/*
pub trait SimpleGuiBlock<'a> {
    fn visit_nodes<T: GuiVisitorTarget<'a>>(self, size: Extent2<f32>, scale: f32, visitor: GuiVisitor<'_, T>);
}

pub struct SimpleSizedGuiBlock<B> {
    block: B,
    size: Extent2<f32>,
    scale: f32,
}

impl<'a, B: SimpleGuiBlock<'a>> GuiBlock<'a, DimParentSets, DimParentSets> for B {
    type Sized = SimpleSizedGuiBlock<B>;

    fn size(self, w: f32, h: f32, scale: f32) -> ((), (), Self::Sized) {
        let sized = SimpleSizedGuiBlock {
            block: self,
            size: Extent2 { w, h },
            scale,
        };
        ((), (), sized)
    }
}

impl<'a, B: SimpleGuiBlock<'a>> SizedGuiBlock<'a> for SimpleSizedGuiBlock<B> {
    fn visit_nodes<T: GuiVisitorTarget<'a>>(self, visitor: GuiVisitor<'_, T>) {
        self.block.visit_nodes(self.size, self.scale, visitor);
    }
}

#[allow(unused_variables)]
pub trait SimpleGuiNode<'a>: Sized {
    fn clips_cursor(&self, size: Extent2<f32>, scale: f32, pos: Vec2<f32>) -> bool {
        pos.x >= 0.0
            && pos.y >= 0.0
            && pos.x <= size.w
            && pos.y <= size.h
    }

    fn draw(self, size: Extent2<f32>, scale: f32, ctx: &GuiContext, canvas: Canvas2<'a, '_>) {}

    fn on_cursor_press(self, size: Extent2<f32>, scale: f32, ctx: &GuiContext, button: MouseButton, pos: Vec2<f32>) {}

    fn on_cursor_release(self, size: Extent2<f32>, scale: f32, ctx: &GuiContext, button: MouseButton, pos: Vec2<f32>) {}

    fn on_cursor_scroll(self, size: Extent2<f32>, scale: f32, ctx: &GuiContext, amount: ScrolledAmount, pos: Vec2<f32>) {}

    fn on_cursor_change(self, size: Extent2<f32>, scale: f32, ctx: &GuiContext) {}
}

impl<'a, B: SimpleGuiNode<'a>> GuiNode<'a> for SimpleSizedGuiBlock<B> {
    fn clips_cursor(&self, pos: Vec2<f32>) -> bool {
        self.block.clips_cursor(self.size, self.scale, pos)
    }

    fn draw(self, ctx: &GuiContext, canvas: Canvas2<'a, '_>) {
        self.block.draw(self.size, self.scale, ctx, canvas)
    }

    fn on_cursor_press(self, ctx: &GuiContext, button: MouseButton, pos: Vec2<f32>) {
        self.block.on_cursor_press(self.size, self.scale, ctx, button, pos)
    }

    fn on_cursor_release(self, ctx: &GuiContext, button: MouseButton, pos: Vec2<f32>) {
        self.block.on_cursor_release(self.size, self.scale, ctx, button, pos)
    }

    fn on_cursor_scroll(self, ctx: &GuiContext, amount: ScrolledAmount, pos: Vec2<f32>) {
        self.block.on_cursor_scroll(self.size, self.scale, ctx, amount, pos)
    }

    fn on_cursor_change(self, ctx: &GuiContext) {
        self.block.on_cursor_change(self.size, self.scale, ctx)
    }
}

impl<'a, B: SimpleGuiNode<'a>> SimpleGuiBlock<'a> for B {
    fn visit_nodes<T: GuiVisitorTarget<'a>>(self, size: Extent2<f32>, scale: f32, visitor: GuiVisitor<'_, T>) {
        visitor
            .visit_node(SimpleSizedGuiBlock {
                block: self,
                size,
                scale,
            });
    }
}

*/
// ==== sequence versions ====





// ==== "gui visitor submap iter" utility ====

/*
pub struct GuiVisitorSubmapIter<'b, T, M> {
    base_visitor: GuiVisitor<'b, T>,
    mapper: M,
}

pub trait GuiVisitorSubmapIterMapper {
    fn map_next<'a, 'b, T: GuiVisitorTarget<'a>>(&'b mut self, visitor: GuiVisitor<'b, T>) -> GuiVisitor<'b, T>;
}

impl<'a, 'b, T: GuiVisitorTarget<'a>, M: GuiVisitorSubmapIterMapper> GuiVisitorIter<'a> for GuiVisitorSubmapIter<'b, T, M> {
    type Target = T;

    fn next<'b2>(&'b2 mut self) -> GuiVisitor<'b2, Self::Target> {
        self.mapper.map_next(self.base_visitor.reborrow())
    }
}

pub struct SubmapIterSizedGuiBlock<M, I> {
    mapper: M,
    items: I,
}

impl<M, I> SubmapIterSizedGuiBlock<M, I> {
    pub fn new(mapper: M, items: I) -> Self {
        SubmapIterSizedGuiBlock {
            mapper,
            items,
        }
    }
}

impl<'a, M: GuiVisitorSubmapIterMapper, I: SizedGuiBlockSeq<'a>> SizedGuiBlock<'a> for SubmapIterSizedGuiBlock<M, I> {
    fn visit_nodes<T: GuiVisitorTarget<'a>>(self, visitor: GuiVisitor<'_, T>) {
        let visitors = GuiVisitorSubmapIter {
            base_visitor: visitor,
            mapper: self.mapper,
        };
        self.items.visit_items_nodes(visitors);
    }
}
*/