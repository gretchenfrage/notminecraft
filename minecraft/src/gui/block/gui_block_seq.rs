
use crate::gui::{
    context::GuiGlobalContext,
    node::{
        GuiVisitorTarget,
        GuiVisitor,
    },
    block::{
        dim_constraint::DimConstraint,
        gui_block::{
            GuiBlock,
            SizedGuiBlock,
        },
    },
};
use std::ops::Index;


/// Sequence version of `GuiBlock`. Essentially a compile-time heterogenous
/// tuple of `GuiBlock` implementations.
///
/// Blanket-impl'd on tuple types. All elements must have the same dimensional
/// constraints. Facilitates avoiding allocations.
pub trait GuiBlockSeq<'a, W: DimConstraint, H: DimConstraint> {
    /// Sequence of sized version of self's elements. Possibly a tuple.
    type SizedSeq: SizedGuiBlockSeq<'a>;

    /// Sequence of `<W as DimConstraint>::Out` of self's elements. Possibly
    /// a fixed-size array.
    type WOutSeq: Index<usize, Output=W::Out>;

    /// Sequence of `<H as DimConstraint>::Out` of self's elements. Possibly
    /// a fixed-size array.
    type HOutSeq: Index<usize, Output=H::Out>;

    /// Number of elements.
    fn len(&self) -> usize;

    /// Call `GuiBlock::size` on all self's elements.
    ///
    /// Elements' `DimensionalConstraint::In`s and scales are passed in
    /// iterators, which should work for `self.len()` elements. Returns
    /// their return values as three unzipped sequences, which should work
    /// for `self.len()` elements.
    fn size_all<
        WInSeq: IntoIterator<Item=W::In>,
        HInSeq: IntoIterator<Item=H::In>,
        ScaleSeq: IntoIterator<Item=f32>,
    >(
        self,
        ctx: &GuiGlobalContext,
        w_in_seq: WInSeq,
        h_in_seq: HInSeq,
        scale_seq: ScaleSeq,
    ) -> (Self::WOutSeq, Self::HOutSeq, Self::SizedSeq);
}

/// Sequence version of `SizedGuiBlock`. Essentially a compile-time
/// heterogenous tuple of `SizedGuiBlock` implementations.
///
/// Blanket-impl'd on tuple types. Facilitates avoiding allocations.
pub trait SizedGuiBlockSeq<'a> {
    /// Call `GuiBlock::visit_nodes` on all items. A special streaming iterator
    /// is given to produce the visitor for each item, which should work for
    /// `GuiBlockSeq::len(gui_block_seq)` items.
    fn visit_items_nodes<I: GuiVisitorIter<'a>>(self, visitors: I);
}

/// Streaming iterator of `GuiVisitor`s, for use with `SizedGuiBlockSeq`.
pub trait GuiVisitorIter<'a> {
    type Target: GuiVisitorTarget<'a>;

    /// Return the next item.
    ///
    /// Doesn't use `Option` because the number of items this should be able to
    /// yield is indicated by `GuiBlockSeq::size`. Behavior unspecified if
    /// called more times than that.
    fn next<'b>(&'b mut self) -> GuiVisitor<'b, 'b, Self::Target>;
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
            >(
                self,
                _ctx: &GuiGlobalContext,
                w_in_seq: WInSeq,
                h_in_seq: HInSeq,
                scale_seq: ScaleSeq,
            ) -> (Self::WOutSeq, Self::HOutSeq, Self::SizedSeq) {
                let ( $( $a, )* ) = self;

                let mut _w_in_iter = w_in_seq.into_iter();
                let mut _h_in_iter = h_in_seq.into_iter();
                let mut _scale_iter = scale_seq.into_iter();

                $(
                let (
                    $a_w_out,
                    $a_h_out,
                    $a_sized,
                ) = $a.size(
                    _ctx,
                    _w_in_iter.next().unwrap(),
                    _h_in_iter.next().unwrap(),
                    _scale_iter.next().unwrap(),
                );
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
