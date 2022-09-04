
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


pub trait GuiNode<'a> { // TODO totally could split these out into seperate traits
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

impl<'a, N: GuiNode<'a>> SizedGuiBlock<'a> for N {
    fn visit_nodes<T: GuiVisitorTarget<'a>>(self, visitor: GuiVisitor<'_, T>) {
        visitor.visit_node(self);
    }
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


pub trait GuiVisitorSubmapIterMapper {
    fn map_next<'a, 'b, T: GuiVisitorTarget<'a>>(&'b mut self, visitor: GuiVisitor<'b, T>) -> GuiVisitor<'b, T>;
}

pub struct GuiVisitorSubmapIter<'b, T, M> {
    base_visitor: GuiVisitor<'b, T>,
    mapper: M,
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


mod margin_block {
    use super::*;

    pub fn h_margin_block<'a, H: DimConstraint, I: GuiBlock<'a, DimParentSets, H>>(unscaled_margin_min: f32, unscaled_margin_max: f32, inner: I) -> impl GuiBlock<'a, DimParentSets, H> {
        HMarginGuiBlock {
            unscaled_margin_min,
            unscaled_margin_max,
            inner,
        }
    }

    struct HMarginGuiBlock<I> {
        unscaled_margin_min: f32,
        unscaled_margin_max: f32,
        inner: I,
    }

    impl<'a, H: DimConstraint, I: GuiBlock<'a, DimParentSets, H>> GuiBlock<'a, DimParentSets, H> for HMarginGuiBlock<I> {
        type Sized = HMarginSizedGuiBlock<I::Sized>;

        fn size(self, w: f32, h_in: H::In, scale: f32) -> ((), H::Out, Self::Sized) {
            let margin_min = self.unscaled_margin_min * scale;
            let margin_max = self.unscaled_margin_max * scale;

            let inner_w = f32::max(w - margin_min - margin_max, 0.0);
            let x_translate = (w - inner_w) / 2.0;

            let ((), h_out, inner_sized) = self.inner.size(inner_w, h_in, scale);

            let sized = HMarginSizedGuiBlock {
                x_translate,
                inner: inner_sized,
            };

            ((), h_out, sized)
        }
    }

    struct HMarginSizedGuiBlock<I> {
        x_translate: f32,
        inner: I,
    }

    impl<'a, I: SizedGuiBlock<'a>> SizedGuiBlock<'a> for HMarginSizedGuiBlock<I> {
        fn visit_nodes<T: GuiVisitorTarget<'a>>(self, mut visitor: GuiVisitor<'_, T>) {
            self.inner.visit_nodes(visitor.reborrow()
                .translate([self.x_translate, 0.0]));
        }
    }


    // ==== TODO dedupe somehow ====


    pub fn v_margin_block<'a, W: DimConstraint, I: GuiBlock<'a, W, DimParentSets>>(unscaled_margin_min: f32, unscaled_margin_max: f32, inner: I) -> impl GuiBlock<'a, W, DimParentSets> {
        VMarginGuiBlock {
            unscaled_margin_min,
            unscaled_margin_max,
            inner,
        }
    }

    struct VMarginGuiBlock<I> {
        unscaled_margin_min: f32,
        unscaled_margin_max: f32,
        inner: I,
    }

    impl<'a, W: DimConstraint, I: GuiBlock<'a, W, DimParentSets>> GuiBlock<'a, W, DimParentSets> for VMarginGuiBlock<I> {
        type Sized = VMarginSizedGuiBlock<I::Sized>;

        fn size(self, w_in: W::In, h: f32, scale: f32) -> (W::Out, (), Self::Sized) {
            let margin_min = self.unscaled_margin_min * scale;
            let margin_max = self.unscaled_margin_max * scale;

            let inner_h = f32::max(h - margin_min - margin_max, 0.0);
            let y_translate = (h - inner_h) / 2.0;

            let (w_out, (), inner_sized) = self.inner.size(w_in, inner_h, scale);

            let sized = VMarginSizedGuiBlock {
                y_translate,
                inner: inner_sized,
            };

            (w_out, (), sized)
        }
    }

    struct VMarginSizedGuiBlock<I> {
        y_translate: f32,
        inner: I,
    }

    impl<'a, I: SizedGuiBlock<'a>> SizedGuiBlock<'a> for VMarginSizedGuiBlock<I> {
        fn visit_nodes<T: GuiVisitorTarget<'a>>(self, mut visitor: GuiVisitor<'_, T>) {
            self.inner.visit_nodes(visitor.reborrow()
                .translate([0.0, self.y_translate]));
        }
    }
}


mod layer_block {
    use super::*;
    use std::iter::repeat;

    pub fn layer_gui_block<'a, I: GuiBlockSeq<'a, DimParentSets, DimParentSets>>(items: I) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
        LayerGuiBlock { items }
    }

    struct LayerGuiBlock<I> {
        items: I,
    }

    impl<'a, I: GuiBlockSeq<'a, DimParentSets, DimParentSets>> GuiBlock<'a, DimParentSets, DimParentSets> for LayerGuiBlock<I> {
        type Sized = SubmapIterSizedGuiBlock<LayerItemVisitorMapper, I::SizedSeq>;

        fn size(self, w: f32, h: f32, scale: f32) -> ((), (), Self::Sized) {
            let w_in_seq = repeat(w);
            let h_in_seq = repeat(h);
            let scale_seq = repeat(scale);

            let (_, _, sized_seq) = self.items.size_all(w_in_seq, h_in_seq, scale_seq);

            let sized = SubmapIterSizedGuiBlock::new(LayerItemVisitorMapper, sized_seq);

            ((), (), sized)
        }
    }

    struct LayerItemVisitorMapper;

    impl GuiVisitorSubmapIterMapper for LayerItemVisitorMapper {
        fn map_next<'a, 'b, T: GuiVisitorTarget<'a>>(&'b mut self, visitor: GuiVisitor<'b, T>) -> GuiVisitor<'b, T> {
            visitor
        }
    }
}


mod stable_unscaled_dim_size {
    use super::*;

    pub fn h_stable_unscaled_dim_size_block<'a, H: DimConstraint, I: GuiBlock<'a, DimParentSets, H>>(unscaled_dim_size: f32, inner: I) -> impl GuiBlock<'a, DimChildSets, H> {
        HStableUnscaledDimSizeGuiBlock {
            unscaled_dim_size,
            inner,
        }
    }

    struct HStableUnscaledDimSizeGuiBlock<I> {
        unscaled_dim_size: f32,
        inner: I,
    }

    impl<'a, H: DimConstraint, I: GuiBlock<'a, DimParentSets, H>> GuiBlock<'a, DimChildSets, H> for HStableUnscaledDimSizeGuiBlock<I> {
        type Sized = I::Sized;

        fn size(self, (): (), h_in: H::In, scale: f32) -> (f32, H::Out, Self::Sized) {
            let w = self.unscaled_dim_size * scale;
            let ((), h_out, sized) = self.inner.size(w, h_in, scale);
            (w, h_out, sized)
        }        
    }


    // ==== TODO dedupe somehow ====


    pub fn v_stable_unscaled_dim_size_block<'a, W: DimConstraint, I: GuiBlock<'a, W, DimParentSets>>(unscaled_dim_size: f32, inner: I) -> impl GuiBlock<'a, W, DimChildSets> {
        VStableUnscaledDimSizeGuiBlock {
            unscaled_dim_size,
            inner,
        }
    }

    struct VStableUnscaledDimSizeGuiBlock<I> {
        unscaled_dim_size: f32,
        inner: I,
    }

    impl<'a, W: DimConstraint, I: GuiBlock<'a, W, DimParentSets>> GuiBlock<'a, W, DimChildSets> for VStableUnscaledDimSizeGuiBlock<I> {
        type Sized = I::Sized;

        fn size(self, w_in: W::In, (): (), scale: f32) -> (W::Out, f32, Self::Sized) {
            let h = self.unscaled_dim_size * scale;
            let (w_out, (), sized) = self.inner.size(w_in, h, scale);
            (w_out, h, sized)
        }        
    }
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


mod stack_block {
    use super::*;
    use std::iter::repeat;

    pub fn v_stack_block<'a, I: GuiBlockSeq<'a, DimParentSets, DimChildSets>>(unscaled_gap: f32, items: I) -> impl GuiBlock<'a, DimParentSets, DimChildSets> {
        VStackGuiBlock {
            unscaled_gap,
            items,
        }
    }

    struct VStackGuiBlock<I> {
        unscaled_gap: f32,
        items: I,
    }

    impl<'a, I: GuiBlockSeq<'a, DimParentSets, DimChildSets>> GuiBlock<'a, DimParentSets, DimChildSets> for VStackGuiBlock<I> {
        type Sized = SubmapIterSizedGuiBlock<VStackItemVisitorMapper<I::HOutSeq>, I::SizedSeq>;

        fn size(self, w: f32, (): (), scale: f32) -> ((), f32, Self::Sized) {
            let len = self.items.len();

            let gap = self.unscaled_gap * scale;

            let w_in_seq = repeat(w);
            let h_in_seq = repeat(());
            let scale_seq = repeat(scale);

            let (_, item_heights, sized_seq) = self.items.size_all(w_in_seq, h_in_seq, scale_seq);

            let mut height = 0.0;
            for i in 0..len {
                if i > 0 {
                    height += gap;
                }
                height += item_heights[i];
            }
            
            let sized = SubmapIterSizedGuiBlock::new(
                VStackItemVisitorMapper {
                    item_heights,
                    gap,
                    next_idx: 0,
                    next_y_translate: 0.0,
                },
                sized_seq,
            );

            ((), height, sized)
        }
    }

    struct VStackItemVisitorMapper<H> {
        item_heights: H,
        gap: f32,
        next_idx: usize,
        next_y_translate: f32,
    }

    impl<H: Index<usize, Output=f32>> GuiVisitorSubmapIterMapper for VStackItemVisitorMapper<H> {
        fn map_next<'a, 'b, T: GuiVisitorTarget<'a>>(&'b mut self, visitor: GuiVisitor<'b, T>) -> GuiVisitor<'b, T> {
            let visitor = visitor
                .translate([0.0, self.next_y_translate]);

            self.next_y_translate += self.item_heights[self.next_idx];
            self.next_y_translate += self.gap;

            self.next_idx += 1;

            visitor
        }
    }
}


mod tile_image_block {
    use super::*;
    use graphics::frame_content::GpuImage;

    pub fn tile_image_gui_block<'a, E: Into<Extent2<f32>>>(image: &'a GpuImage, size_unscaled_untiled: E) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
        let size_unscaled_untiled = size_unscaled_untiled.into();
        TileImageGuiBlock {
            image,
            size_unscaled_untiled,
        }
    }

    struct TileImageGuiBlock<'a> {
        image: &'a GpuImage,
        size_unscaled_untiled: Extent2<f32>,
    }

    impl<'a> GuiBlock<'a, DimParentSets, DimParentSets> for TileImageGuiBlock<'a> {
        type Sized = SizedTileImageGuiBlock<'a>;

        fn size(self, w: f32, h: f32, scale: f32) -> ((), (), Self::Sized) {
            let sized = SizedTileImageGuiBlock {
                block: self,
                size: Extent2 { w, h },
                scale,
            };
            ((), (), sized)
        }
    }

    struct SizedTileImageGuiBlock<'a> {
        block: TileImageGuiBlock<'a>,
        size: Extent2<f32>,
        scale: f32,
    }

    impl<'a> GuiNode<'a> for SizedTileImageGuiBlock<'a> {
        fn draw(&'a mut self, _: &Renderer, mut canvas: Canvas2<'a, '_>) {
            let tex_extent = self.size / (self.block.size_unscaled_untiled * self.scale);
            canvas.reborrow()
                .draw_image_uv(
                    &self.block.image,
                    self.size,
                    [0.0, 0.0],
                    tex_extent,
                );
        }
    }
}


mod modifier_block {
    use super::*;

    pub fn modifier_block<'a, W: DimConstraint, H: DimConstraint, M: Into<Modifier2>, I: GuiBlock<'a, W, H>>(modifier: M, inner: I) -> impl GuiBlock<'a, W, H> {
        let modifier = modifier.into();
        ModifierGuiBlock {
            modifier,
            inner,
        }
    }

    struct ModifierGuiBlock<I> {
        modifier: Modifier2,
        inner: I,
    }

    impl<'a, W: DimConstraint, H: DimConstraint, I: GuiBlock<'a, W, H>> GuiBlock<'a, W, H> for ModifierGuiBlock<I> {
        type Sized = ModifierSizedGuiBlock<I::Sized>;

        fn size(self, w_in: W::In, h_in: H::In, scale: f32) -> (W::Out, H::Out, Self::Sized) {
            let (w_out, h_out, inner_sized) = self.inner.size(w_in, h_in, scale);
            let sized = ModifierSizedGuiBlock {
                modifier: self.modifier,
                inner: inner_sized,
            };
            (w_out, h_out, sized)
        }
    }

    struct ModifierSizedGuiBlock<I> {
        modifier: Modifier2,
        inner: I,
    }

    impl<'a, I: SizedGuiBlock<'a>> SizedGuiBlock<'a> for ModifierSizedGuiBlock<I> {
        fn visit_nodes<T: GuiVisitorTarget<'a>>(self, mut visitor: GuiVisitor<'_, T>) {
            self.inner.visit_nodes(visitor.reborrow()
                .modify(self.modifier));
        }
    }
}


mod text_block {
    pub struct TextGuiBlockLayoutCache {

    }
}
