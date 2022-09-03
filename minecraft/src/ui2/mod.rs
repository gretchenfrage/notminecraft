
use graphics::{
    Renderer,
    modifier::{
        Modifier2,
        Transform2,
        Clip2,
    },
    frame_content::{
        FrameContent,
        FrameItem,
        Canvas2,
    },
};
use std::{
    borrow::Borrow,
    ops::Index,
};
use vek::*;


pub enum InputEvent {}


pub trait GuiNode<'a> {
    fn draw(&'a mut self, renderer: &Renderer, canvas: Canvas2<'a, '_>) {}

    fn handle_input_event(&mut self, renderer: &Renderer, event: InputEvent) {}
}

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

    pub fn visit_node<I: GuiNode<'a>>(mut self, node: I) -> Self {
        self.target.visit_node(self.stack_len, node);
        self
    }
}

pub trait SizedGuiBlock<'a> {
    fn visit_nodes<T: GuiVisitorTarget<'a>>(self, visitor: GuiVisitor<'_, T>);
}


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

pub trait GuiBlock<'a, W: DimConstraint, H: DimConstraint> {
    type Sized: SizedGuiBlock<'a>;

    fn size(self, w_in: W::In, h_in: H::In, scale: f32) -> (W::Out, H::Out, Self::Sized);
}


pub trait GuiBlockSeq<'a, W: DimConstraint, H: DimConstraint>
/*where
    for<'i> &'i Self::WOutSeq: IntoIterator,
    for<'i> <&'i Self::WOutSeq as IntoIterator>::Item: Borrow<W::Out>,
    for<'i> &'i Self::HOutSeq: IntoIterator,
    for<'i> <&'i Self::HOutSeq as IntoIterator>::Item: Borrow<H::Out>,*/
{
    type SizedSeq: SizedGuiBlockSeq<'a>;
    type WOutSeq: Index<usize, Output=W::Out>;
    type HOutSeq: Index<usize, Output=H::Out>;
    //type WOutSeq;
    //type HOutSeq;
    //type WOutSeq: IntoIterator<Item=W::Out>;
    //type HOutSeq: IntoIterator<Item=H::Out>;

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

                let mut w_in_iter = w_in_seq.into_iter();
                let mut h_in_iter = h_in_seq.into_iter();
                let mut scale_iter = scale_seq.into_iter();;

                $(
                let ($a_w_out, $a_h_out, $a_sized) = $a.size(w_in_iter.next().unwrap(), h_in_iter.next().unwrap(), scale_iter.next().unwrap());
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
            fn visit_items_nodes<I: GuiVisitorIter<'a>>(self, mut visitors: I) {
                let ( $( $a, )* ) = self;

                $(
                $a.visit_nodes(visitors.next());
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


pub mod stack_block {
    use super::*;
    use std::iter::repeat;

    struct VStackGuiBlock<I> {
        unscaled_gap: f32,
        items: I,
    }

    impl<'a, I: GuiBlockSeq<'a, DimParentSets, DimChildSets>> GuiBlock<'a, DimParentSets, DimChildSets> for VStackGuiBlock<I> {
        type Sized = VStackSizedGuiBlock<I::HOutSeq, I::SizedSeq>;

        fn size(self, w: f32, (): (), scale: f32) -> ((), f32, Self::Sized) {
            let len = self.items.len();

            let gap = self.unscaled_gap * scale;

            let w_in_seq = repeat(w);
            let h_in_seq = repeat(());
            let scale_seq = repeat(scale);

            let (_, item_heights, sized_seq) = self.items.size_all(w_in_seq, h_in_seq, scale_seq);
            /*
            let h = item_heights.into_iter()
                .enumerate()
                .map(|(i, h)| if i == 0 { 0.0 } else { gap } + *h.borrow())
                .sum();*/

            let mut height = 0.0;
            for i in 0..len {
                if i > 0 {
                    height += gap;
                }
                height += item_heights[i];
            }

            let sized = VStackSizedGuiBlock {
                gap,
                item_heights,
                items: sized_seq,
            };

            ((), height, sized)
        }
    }
    /*
    struct VStackItemVisitorIter<'b, T, I> {
        remaining: usize,
        visitor: GuiVisitor<'b, T>,
        gap: f32,
        y_translate: f32,
        item_heights_iter: I,
    }

    impl<'a, 'b, T: GuiVisitorTarget<'a>, I: Iterator> GuiVisitorIter<'a> for VStackItemVisitorIter<'b, T, I>
    where
        <I as Iterator>::Item: Borrow<f32>,
    {
        type Target = T;

        fn next<'b2>(&'b2 mut self) -> GuiVisitor<'b2, Self::Target> {
            let curr = self.visitor.reborrow()
                .translate([0.0, self.y_translate]);
            self.remaining -= 1;
            if self.remaining > 0 {
                self.y_translate += *self.item_heights_iter.next().unwrap().borrow();
                self.y_translate += self.gap;
            }
            curr
        }
    }
    */
    struct VStackItemVisitorIter<'b, T, H> {
        base_visitor: GuiVisitor<'b, T>,
        item_heights: H,
        gap: f32,
        next_idx: usize,
        next_y_translate: f32,
    }

    impl<'a, 'b, T: GuiVisitorTarget<'a>, H: Index<usize, Output=f32>> GuiVisitorIter<'a> for VStackItemVisitorIter<'b, T, H> {
        type Target = T;

        fn next<'b2>(&'b2 mut self) -> GuiVisitor<'b2, Self::Target> {
            let visitor = self.base_visitor.reborrow()
                .translate([0.0, self.next_y_translate]);

            self.next_y_translate += self.item_heights[self.next_idx];
            self.next_y_translate += self.gap;

            self.next_idx += 1;

            visitor
        }
    }

    struct VStackSizedGuiBlock<H, I> {
        gap: f32,
        item_heights: H,
        items: I,
    }

    impl<'a, H: Index<usize, Output=f32>, I: SizedGuiBlockSeq<'a>> SizedGuiBlock<'a> for VStackSizedGuiBlock<H, I> {
        fn visit_nodes<T: GuiVisitorTarget<'a>>(self, visitor: GuiVisitor<'_, T>) {
            let visitors = VStackItemVisitorIter {
                base_visitor: visitor,
                item_heights: self.item_heights,
                gap: self.gap,
                next_idx: 0,
                next_y_translate: 0.0,
            };
            self.items.visit_items_nodes(visitors);
        }
    }
    /*
    struct VStackSizedGuiBlock<H, I> {
        len: usize,
        gap: f32,
        item_heights: H,
        items: I,
    }

    impl<'a, H, I: SizedGuiBlockSeq<'a>> SizedGuiBlock<'a> for VStackSizedGuiBlock<H, I>
    where
        for<'i> &'i H: IntoIterator,
        for<'i> <&'i H as IntoIterator>::Item: Borrow<f32>,
    {
        fn visit_nodes<T: GuiVisitorTarget<'a>>(self, visitor: GuiVisitor<'_, T>) {
            let visitors = VStackItemVisitorIter {
                remaining: self.len,
                visitor,
                gap: self.gap,
                y_translate: 0.0,
                item_heights_iter: self.item_heights.into_iter(),
            };
            self.items.visit_items_nodes(visitors);
        }
    }*/
}


mod center_block {
    use super::*;
    
    pub fn center_block<'a, H: DimConstraint, I: GuiBlock<'a, DimChildSets, H>>(inner: I) -> impl GuiBlock<'a, DimParentSets, H> {
        HCenterGuiBlock { inner }
    }

    struct HCenterGuiBlock<I> {
        inner: I,
    }

    impl<'a, H: DimConstraint, I: GuiBlock<'a, DimChildSets, H>> GuiBlock<'a, DimParentSets, H> for HCenterGuiBlock<I> {
        type Sized = HCenterSizedGuiBlock<I::Sized>;

        fn size(self, w: f32, h_in: H::In, scale: f32) -> ((), H::Out, Self::Sized) {
            let (inner_w, h_out, inner_sized) = self.inner.size((), h_in, scale);
            let sized = HCenterSizedGuiBlock {
                x_translate: (w - inner_w) / 2.0,
                inner: inner_sized,
            };
            ((), h_out, sized)
        }
    }


    struct HCenterSizedGuiBlock<I> {
        x_translate: f32,
        inner: I,
    }

    impl<'a, I: SizedGuiBlock<'a>> SizedGuiBlock<'a> for HCenterSizedGuiBlock<I> {
        fn visit_nodes<T: GuiVisitorTarget<'a>>(self, mut visitor: GuiVisitor<'_, T>) {
            self.inner.visit_nodes(visitor.reborrow()
                .translate([self.x_translate, 0.0]));
        }
    }
}
