
use super::{
    GuiNode,
    GuiVisitor,
    GuiVisitorTarget,
    GuiContext,
    MouseButton,
    ScrolledAmount,
};
use graphics::frame_content::Canvas2;
use std::ops::Index;
use vek::*;


//mod mc;
mod axis_swap;
mod center;
mod cursor_is_over_tracker;
mod layer;
mod margin;
//mod modifier;
//mod stable_unscaled_dim_size;
//mod stack;
//mod text;
//mod tile_9;
//mod tile_image;


pub use self::{
    axis_swap::axis_swap,
    center::{h_center, v_center},
    cursor_is_over_tracker::cursor_is_over_tracker,
    layer::layer,
    margin::{h_margin, v_margin},
};


// ==== dim constraint definition ====

pub trait DimConstraint {
    type In;
    type Out;
}

pub enum DimParentSets {}

impl DimConstraint for DimParentSets {
    type In = f32;
    type Out = ();
}

pub enum DimChildSets {}

impl DimConstraint for DimChildSets {
    type In = ();
    type Out = f32;
}


// ==== gui block and sized gui block ====


pub trait GuiBlock<'a, W: DimConstraint, H: DimConstraint> {
    type Sized: SizedGuiBlock<'a>;

    fn size(self, w_in: W::In, h_in: H::In, scale: f32) -> (W::Out, H::Out, Self::Sized);
}

pub trait SizedGuiBlock<'a> {
    fn visit_nodes<T: GuiVisitorTarget<'a>>(self, visitor: GuiVisitor<'_, T>);
}
/*
impl<'a, N: GuiNode<'a>> SizedGuiBlock<'a> for N {
    fn visit_nodes<T: GuiVisitorTarget<'a>>(self, visitor: GuiVisitor<'_, T>) {
        visitor.visit_node(self);
    }
}
*/

// ==== "simple" ((sized) block) / node utility ====


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


// ==== sequence versions ====


pub trait GuiBlockSeq<'a, W: DimConstraint, H: DimConstraint> {
    type SizedSeq: SizedGuiBlockSeq<'a>;
    type WOutSeq: Index<usize, Output=W::Out>;
    type HOutSeq: Index<usize, Output=H::Out>;

    fn len(&self) -> usize;

    fn size_all<
        WInSeq: IntoIterator<Item=W::In>,
        HInSeq: IntoIterator<Item=H::In>,
        ScaleSeq: IntoIterator<Item=f32>,
    >(self, w_in_seq: WInSeq, h_in_seq: HInSeq, scale_seq: ScaleSeq) -> (Self::WOutSeq, Self::HOutSeq, Self::SizedSeq);
}

pub trait SizedGuiBlockSeq<'a> {
    fn visit_items_nodes<I: GuiVisitorIter<'a>>(self, visitors: I);
}

pub trait GuiVisitorIter<'a> {
    type Target: GuiVisitorTarget<'a>;

    fn next<'b>(&'b mut self) -> GuiVisitor<'b, Self::Target>;
}

macro_rules! gui_seq_tuple {
    (
        $len:expr,
        $(($A:ident, $a:ident, $a_w_out:ident, $a_h_out:ident, $a_sized:ident)),*$(,)?
    )=>{
        impl<
            'a, W: DimConstraint, H: DimConstraint,
            $( $A: GuiBlock<'a, W, H>, )*
        > GuiBlockSeq<'a, W, H> for ( $( $A, )* ) {
            type SizedSeq = ( $( $A::Sized, )* );
            type WOutSeq = [W::Out; $len];
            type HOutSeq = [H::Out; $len];

            fn len(&self) -> usize { $len }

            fn size_all<
                WInSeq: IntoIterator<Item=W::In>,
                HInSeq: IntoIterator<Item=H::In>,
                ScaleSeq: IntoIterator<Item=f32>,
            >(self, w_in_seq: WInSeq, h_in_seq: HInSeq, scale_seq: ScaleSeq) -> (Self::WOutSeq, Self::HOutSeq, Self::SizedSeq) {
                let ( $( $a, )* ) = self;

                let mut _w_in_iter = w_in_seq.into_iter();
                let mut _h_in_iter = h_in_seq.into_iter();
                let mut _scale_iter = scale_seq.into_iter();

                $(
                let ($a_w_out, $a_h_out, $a_sized) = $a.size(_w_in_iter.next().unwrap(), _h_in_iter.next().unwrap(), _scale_iter.next().unwrap());
                )*

                let w_out_seq = [ $( $a_w_out, )* ];
                let h_out_seq = [ $( $a_h_out, )* ];
                let sized_seq = ( $( $a_sized, )* );

                (w_out_seq, h_out_seq, sized_seq)
            }
        }

        impl<
            'a,
            $( $A: SizedGuiBlock<'a>, )*
        > SizedGuiBlockSeq<'a> for ( $( $A, )* ) {
            fn visit_items_nodes<I: GuiVisitorIter<'a>>(self, mut _visitors: I) {
                let ( $( $a, )* ) = self;

                $(
                $a.visit_nodes(_visitors.next());
                )*
            }
        }
    };
}

macro_rules! gui_seq_tuples {
    ()=>{
        gui_seq_tuple!(0,);
    };
    (
        ($A:ident, $a:ident, $a_w_out:ident, $a_h_out:ident, $a_sized:ident),
        $(($B:ident, $b:ident, $b_w_out:ident, $b_h_out:ident, $b_sized:ident),)*
    )=>{
        gui_seq_tuple!(
            1 $( + { let $b = 1; $b } )*,
            ($A, $a, $a_w_out, $a_h_out, $a_sized),
            $( ($B, $b, $b_w_out, $b_h_out, $b_sized), )*
        );
        gui_seq_tuples!(
            $( ($B, $b, $b_w_out, $b_h_out, $b_sized), )*
        );
    };
}

gui_seq_tuples!(
    (A01, a01, a01_w_out, a01_h_out, a01_sized),
    (A02, a02, a02_w_out, a02_h_out, a02_sized),
    (A03, a03, a03_w_out, a03_h_out, a03_sized),
    (A04, a04, a04_w_out, a04_h_out, a04_sized),
    (A05, a05, a05_w_out, a05_h_out, a05_sized),
    (A06, a06, a06_w_out, a06_h_out, a06_sized),
    (A07, a07, a07_w_out, a07_h_out, a07_sized),
    (A08, a08, a08_w_out, a08_h_out, a08_sized),
    (A09, a09, a09_w_out, a09_h_out, a09_sized),
    (A10, a10, a10_w_out, a10_h_out, a10_sized),
    (A11, a11, a11_w_out, a11_h_out, a11_sized),
    (A12, a12, a12_w_out, a12_h_out, a12_sized),
    (A13, a13, a13_w_out, a13_h_out, a13_sized),
    (A14, a14, a14_w_out, a14_h_out, a14_sized),
    (A15, a15, a15_w_out, a15_h_out, a15_sized),
    (A16, a16, a16_w_out, a16_h_out, a16_sized),
    (A17, a17, a17_w_out, a17_h_out, a17_sized),
    (A18, a18, a18_w_out, a18_h_out, a18_sized),
    (A19, a19, a19_w_out, a19_h_out, a19_sized),
    (A20, a20, a20_w_out, a20_h_out, a20_sized),
);


// ==== "gui visitor submap iter" utility ====


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
