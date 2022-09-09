
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
    collections::BTreeSet;
};
use vek::*;
use winit_main::event::{
    ElementState,
    VirtualKeyCode,
    ScanCode,
    MouseButton,
};


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

    /// Current position of cursor in node's space, if one currently exists.
    pub cursor_pos: Option<Vec2<f32>>,
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum CursorEventConsumed {
    Consumed,
    NotConsumed,
}

/// Amount of scrolling.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub enum ScrolledAmount {
    /// Scrolling in pixels, such as from a trackpad.
    Pixels(Vec2<f32>),
    /// Scrolling in lines, such as from a traditional mouse wheel.
    Lines(Vec2<f32>),
}

pub trait GuiNode<'a>: Sized {
    /// Draw to the canvas.
    fn draw(self, ctx: &GuiContext, canvas: Canvas2<'a, '_>) {}

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
    fn on_cursor_scroll(self, ctx: &GuiContext, amount: ScrolledAmount) -> CursorEventConsumed;
}

pub trait GuiStateFrame<'a> {
    fn size_visit_nodes<T: GuiVisitorTarget<'a>>(
        &'a mut self,
        size: Extent2<f32>,
        scale: f32,
        visitor: GuiVisitor<'_, T>,
    )
    where
        Self: Sized;

    fn on_keyboard_key_press(&mut self, ctx: &GuiContext, key_semantic: Option<VirtualKeyCode>, key_physical: ScanCode);

    fn on_character_input(&mut self, ctx: &GuiContext, c: char);

    fn on_captured_mouse_click(&mut self, ctx: &GuiContext, button: MouseButton);

    fn on_captured_mouse_move(&mut self, ctx: &GuiContext, amount: Vec2<f32>);

    fn on_captured_mouse_scroll(&mut self, ctx: &GuiContext, amount: ScrolledAmount);
}

/*
#[derive(Debug, Clone)]
pub struct GuiContext<'r, 'p> {
    /// Set of pressed mouse buttons. Updates regardless of focus level.
    pub pressed_mouse_buttons: &'p BTreeSet<MouseButton>,
    /// Set of pressed keys, by semantic identifier. Updates regardless of focus level.
    pub pressed_keys_semantic: &'p BTreeSet<VirtualKeyCode>,
    /// Set of pressed keys, by physical identifier. Updates regardless of focus level.
    pub pressed_keys_physical: &'p BTreeSet<ScanCode>,
    pub renderer: &'r Renderer,
    /// Window's current focus level.
    pub focus_level: FocusLevel,
    /// Current cursor position on screen, if any.
    pub cursor_pos: Option<Vec2<f32>>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum FocusLevel {
    /// Window is not focused, cursor is free.
    Unfocused,
    /// Window is focused, cursor is still free.
    Focused,
    /// Window is focused and cursor is grabbed and hidden.
    MouseCaptured,
}

/*
#[derive(Debug, Clone)]
pub enum InputEvent {
    /// Mouse button was pressed or released.
    MouseButtonStateChanged {
        state: ElementState,
        button: MouseButton,
    },
    /// Virtual cursor was moved.
    CursorMoved {
        /// Old position on screen.
        old_pos: Option<Vec2<f32>>,
        /// New position on screen.
        new_pos: Vec2<f32>,
    },
    /// Physical mouse was moved.
    MouseMoved {
        /// Position change in unspecific units.
        amount: Vec<f32>,
    },
    /// Scrolling occurred.
    Scrolled {
        /// Amount of scrolling.
        amount: ScrolledAmount,
    }
    /// Keyboard key was pressed or released.
    KeyboardKeyStateChanged {
        /// Whether pressed or released.
        state: ElementState,
        /// Semantic identifier of key, if any.
        key_semantic: Option<VirtualKeyCode>,
        /// Physical identifier of key.
        key_physical: ScanCode,
    },
    /// A unicode character was inputted to the window.
    CharacterInput(char),
}*/

/// Amount of scrolling.
#[derive(Debug, Copy, Clone)]
pub enum ScrolledAmount {
    /// Scrolling in pixels, such as from a trackpad.
    Pixels(Vec2<f32>),
    /// Scrolling in lines, such as from a traditional mouse wheel.
    Lines(Vec2<f32>),
}

impl ScrolledAmount {
    pub fn to_pixels(self, pixels_per_line: f32) -> Self {
        match self {
            ScrolledAmount::Pixels(amount) => amount,
            ScrolledAmount::Lines(amount) => amount * pixels_per_line,
        }
    }
}

impl InputEvent {
    pub fn filter_map_pos<F: Fn(Vec2<f32>) -> Option<Vec2<f32>>>(self, f: F) -> Option<Self> {
        match self {
            InputEvent::CursorMoved(pos) => f(pos).map(InputEvent::CursorMoved),
        }
    }
}


pub trait GuiNode<'a>: Sized {
    /// Draw to the canvas.
    fn draw(self, ctx: &GuiContext, canvas: Canvas2<'a, '_>) {}

    /// Called when mouse button is pressed down and:
    /// - window is focused, but not cursor-captured
    /// - cursor pos is not clipped or otherwise modified out of this node's space
    ///
    /// Guaranteed that `ctx.cursor_pos == Some(pos)`.
    ///
    /// For logic regarding something happening during a period where a mouse
    /// button is being depressed, should use `ctx.pressed_mouse_buttons` and
    /// `ctx.focus_level`.
    fn on_cursor_press(self, ctx: &GuiContext, button: MouseButton, pos: Vec2<f32>);

    /// Called when a mouse button is pressed down and window is cursor-captured.
    ///
    /// Guaranteed that `button` in `ctx.pressed_mouse_buttons`.
    fn on_captured_mouse_press(self, ctx: &GuiContext, button: MouseButton);

    /// Called upon a physical mouse movement while the window is cursor-captured.
    ///
    /// Units are unspecified.
    fn on_captured_mouse_movement(self, ctx: &GuiContext, amount: Vec2<f32>);

    /// Called when scrolling occurs and _either_:
    /// - cursor pos is over window and not clipped or otherwise modified out of this node's space
    /// - window is cursor-capturedkk
    fn on_cursor_scroll(self, ctx: &GuiContext, amount: ScrolledAmount);

    /*
    /// Called when any mouse button is pressed while the window is focused.
    ///
    /// For logic that corresponds to a period of a mouse button being depressed, should use
    /// `ctx.pressed_mouse_buttons` instead. 
    fn on_mouse_button_pressed(self, ctx: &GuiContext, button: MouseButton);

    /// Called when the cursor changes position, whether or not the window is focused.
    fn on_cursor_moved(self, ctx: &GuiContext, old_pos: Option<Vec2<f32>>, new_pos: Vec2<f32>);

    /// Called upon physical mouse movement while the window is cursor-captured.
    fn on_mouse_movement(self, ctx: &GuiContext, amount: Vec2<f32>);

    /// Called when scrolling occurs while the window is focused or the cursor is over the window.
    fn on_scrolled(self, ctx: &GuiContext, amount: ScrolledAmount);

    /// Called when any keyboard key is pressed while the window is focused.
    ///
    /// For logic that corresponds to a period of a key being depressed, should use
    /// `ctx.pressed_keys_semantic` and `ctx.pressed_keys_physical` instead.
    fn on_keyboard_key_pressed(self, ctx: &GuiContext, key_semantic: Option<VirtualKeyCode>, key_physical: ScanCode);

    /// Called upon a unicode character being inputted into the window.
    fn on_character_input(self, ctx: &GuiContext, c: char);*/
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

    pub fn h_margin_gui_block<'a, H: DimConstraint, I: GuiBlock<'a, DimParentSets, H>>(unscaled_margin_min: f32, unscaled_margin_max: f32, inner: I) -> impl GuiBlock<'a, DimParentSets, H> {
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


    pub fn v_margin_gui_block<'a, W: DimConstraint, I: GuiBlock<'a, W, DimParentSets>>(unscaled_margin_min: f32, unscaled_margin_max: f32, inner: I) -> impl GuiBlock<'a, W, DimParentSets> {
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


pub use self::{
    margin_block::{
        h_margin_gui_block,
        v_margin_gui_block,
    },
    tile_9_block::{
        LoadTile9ImagesConfig,
        Tile9Images,
        tile_9_gui_block,
    },
    layer_block::layer_gui_block,
    stable_unscaled_dim_size::{
        h_stable_unscaled_dim_size_gui_block,
        v_stable_unscaled_dim_size_gui_block,
    },
    center_block::{
        h_center_gui_block,
        v_center_gui_block,
    },
    stack_block::v_stack_gui_block,
    tile_image_block::tile_image_gui_block,
    modifier_block::modifier_gui_block,
    text_block::{
        TextGuiBlock,
        TextGuiBlockSpan,
    },
    mc_title_block::McTitleGuiBlock,
    mc_splash_text_block::McSplashTextGuiBlock,
    cursor_is_over_tracker_block::CursorIsOverTrackerGuiBlock,
};

mod tile_9_block {
    use super::*;
    use graphics::frame_content::GpuImage;
    use image::DynamicImage;
    use vek::*;

    /// Specification for how to slice a 9-part tileable image from a base image.
    #[derive(Debug, Clone)]
    pub struct LoadTile9ImagesConfig<'a> {
        pub raw_image: &'a DynamicImage,
        pub px_start: Vec2<u32>,
        pub px_extent: Extent2<u32>,
        pub px_top: u32,
        pub px_bottom: u32,
        pub px_left: u32,
        pub px_right: u32,
    }

    impl<'a> LoadTile9ImagesConfig<'a> {
        pub fn load(&self, renderer: &Renderer) -> Tile9Images {
            // TODO: we really could do the cropping on GPU relatively easily
            assert!(self.px_top + self.px_bottom < self.px_extent.h);
            assert!(self.px_left + self.px_right < self.px_extent.w);

            let px_h_middle = self.px_extent.w - self.px_left - self.px_right;
            let px_v_middle = self.px_extent.h - self.px_top - self.px_bottom;

            let corners = [
                (false, false),
                (false, true),
                (true, false),
                (true, true),
            ]
                .map(|(bottom, right)| self.raw_image.crop_imm(
                    // start x:
                    self.px_start.x + match right {
                        false => 0,
                        true => self.px_extent.w - self.px_right,
                    },
                    // start y:
                    self.px_start.y + match bottom {
                        false => 0,
                        true => self.px_extent.h - self.px_bottom,
                    },
                    // extent w:
                    match right {
                        false => self.px_left,
                        true => self.px_right,
                    },
                    // extent h:
                    match bottom {
                        false => self.px_top,
                        true => self.px_bottom,
                    },
                ))
                .map(|texture| renderer.load_image_raw(texture));
            let h_edges = [
                (0, self.px_top),
                (self.px_extent.h - self.px_bottom, self.px_bottom),
            ]
                .map(|(offset, extent)| self.raw_image.crop_imm(
                    // start x:
                    self.px_start.x + self.px_left,
                    // start y:
                    self.px_start.y + offset,
                    // extent w:
                    px_h_middle,
                    // extent h:
                    extent,
                ))
                .map(|texture| renderer.load_image_raw(texture));
            let v_edges = [
                (0, self.px_left),
                (self.px_extent.w - self.px_right, self.px_right)
            ]
                .map(|(offset, extent)| self.raw_image.crop_imm(
                    // start x:
                    self.px_start.x + offset,
                    // start y:
                    self.px_start.y + self.px_top,
                    // extent w:
                    extent,
                    // extent h:
                    px_v_middle,
                ))
                .map(|texture| renderer.load_image_raw(texture));
            let middle = self.raw_image
                .crop_imm(
                    self.px_start.x + self.px_left,
                    self.px_start.y + self.px_top,
                    px_h_middle,
                    px_v_middle,
                );
            let middle = renderer.load_image_raw(middle);
          
            Tile9Images {
                corners,
                h_edges,
                v_edges,
                middle,
            }
        }
    }

    /// 9-part (corners, edges, center) tileable image.
    #[derive(Debug, Clone)]
    pub struct Tile9Images {
        /// Top-left, top-right, bottom-left, bottom-right.
        pub corners: [GpuImage; 4],
        /// Top, bottom.
        pub h_edges: [GpuImage; 2],
        /// Left, right.
        pub v_edges: [GpuImage; 2],
        /// The middle image.
        pub middle: GpuImage,
    }

    pub fn tile_9_gui_block<'a>(
        images: &'a Tile9Images,
        size_unscaled_untiled: Extent2<f32>,
        frac_top: f32,
        frac_bottom: f32,
        frac_left: f32,
        frac_right: f32,
    ) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
        Tile9GuiBlock {
            images,
            size_unscaled_untiled,
            frac_top,
            frac_bottom,
            frac_left,
            frac_right,
        }
    }

    struct Tile9GuiBlock<'a> {
        images: &'a Tile9Images,
        /// Size of the whole (unsliced) image before scaling and tiling.
        size_unscaled_untiled: Extent2<f32>,
        /// Fraction of the whole (unsliced) image taken by the top edge.
        frac_top: f32,
        /// Fraction of the whole (unsliced) image taken by the bottom edge.
        frac_bottom: f32,
        /// Fraction of the whole (unsliced) image taken by the left edge.
        frac_left: f32,
        /// Fraction of the whole (unsliced) image taken by the right edge.
        frac_right: f32,
    }

    impl<'a> GuiBlock<'a, DimParentSets, DimParentSets> for Tile9GuiBlock<'a> {
        type Sized = Tile9SizedGuiBlock<'a>;

        fn size(self, w: f32, h: f32, scale: f32) -> ((), (), Self::Sized) {
            let sized = Tile9SizedGuiBlock {
                block: self,
                size: Extent2 { w, h },
                scale,
            };
            ((), (), sized)
        }
    }

    struct Tile9SizedGuiBlock<'a> {
        block: Tile9GuiBlock<'a>,
        size: Extent2<f32>,
        scale: f32,
    }

    impl<'a> GuiNode<'a> for Tile9SizedGuiBlock<'a> {
        fn draw(mut self, _: &Renderer, mut canvas: Canvas2<'a, '_>) {
            let half_height = self.size.h / 2.0;
            let half_width = self.size.w / 2.0;

            let top = f32::min(self.block.size_unscaled_untiled.h * self.block.frac_top * self.scale, half_height);
            let bottom = f32::min(self.block.size_unscaled_untiled.h * self.block.frac_bottom * self.scale, half_height);

            let left = f32::min(self.block.size_unscaled_untiled.w * self.block.frac_left * self.scale, half_width);
            let right = f32::min(self.block.size_unscaled_untiled.w * self.block.frac_right * self.scale, half_width);

            let middle_size = self.size - Vec2 {
                x: left + right,
                y: top + bottom,
            };
            let middle_tex_extent = 
                middle_size
                / (
                    Extent2 {
                        w: 1.0 - (self.block.frac_left + self.block.frac_right),
                        h: 1.0 - (self.block.frac_top + self.block.frac_bottom),
                    }
                    * self.block.size_unscaled_untiled
                    * self.scale
                );
            

            for ((is_bottom, is_right), image) in [
                (false, false),
                (false, true),
                (true, false),
                (true, true),
            ].into_iter().zip(&self.block.images.corners)
            {
                canvas.reborrow()
                    .translate(Vec2 {
                        x: match is_right {
                            false => 0.0,
                            true => self.size.w - right
                        },
                        y: match is_bottom {
                            false => 0.0,
                            true => self.size.h - bottom,
                        },
                    })
                    .draw_image_uv(
                        image,
                        Extent2 {
                            w: match is_right {
                                false => left,
                                true => right,
                            },
                            h: match is_bottom {
                                false => top,
                                true => bottom,
                            },
                        },
                        [0.0; 2],
                        [1.0; 2],
                    );
            }

            for (is_bottom, image) in [false, true].iter()
                .zip(&self.block.images.h_edges)
            {
                canvas.reborrow()
                    .translate(Vec2 {
                        x: left,
                        y: match is_bottom {
                            false => 0.0,
                            true => self.size.h - bottom,
                        },
                    })
                    .draw_image_uv(
                        image,
                        Extent2 {
                            w: middle_size.w,
                            h: match is_bottom {
                                false => top,
                                true => bottom,
                            },
                        },
                        [0.0; 2],
                        Extent2 {
                            w: middle_tex_extent.w,
                            h: 1.0,
                        },
                    );
            }

            for (is_right, image) in [false, true].iter()
                .zip(&self.block.images.v_edges)
            {
                canvas.reborrow()
                    .translate(Vec2 {
                        x: match is_right {
                            false => 0.0,
                            true => self.size.w - right,
                        },
                        y: top,
                    })
                    .draw_image_uv(
                        image,
                        Extent2 {
                            w: match is_right {
                                false => left,
                                true => right,
                            },
                            h: middle_size.h,
                        },
                        [0.0; 2],
                        Extent2 {
                            w: 1.0,
                            h: middle_tex_extent.h,
                        },
                    );
            }

            canvas.reborrow()
                .translate([left, top])
                .draw_image_uv(
                    &self.block.images.middle,
                    middle_size,
                    [0.0; 2],
                    middle_tex_extent,
                );
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

    pub fn h_stable_unscaled_dim_size_gui_block<'a, H: DimConstraint, I: GuiBlock<'a, DimParentSets, H>>(unscaled_dim_size: f32, inner: I) -> impl GuiBlock<'a, DimChildSets, H> {
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


    pub fn v_stable_unscaled_dim_size_gui_block<'a, W: DimConstraint, I: GuiBlock<'a, W, DimParentSets>>(unscaled_dim_size: f32, inner: I) -> impl GuiBlock<'a, W, DimChildSets> {
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
    
    pub fn h_center_gui_block<'a, H: DimConstraint, I: GuiBlock<'a, DimChildSets, H>>(frac: f32, inner: I) -> impl GuiBlock<'a, DimParentSets, H> {
        HCenterGuiBlock {
            frac,
            inner,
        }
    }

    struct HCenterGuiBlock<I> {
        frac: f32,
        inner: I,
    }

    impl<'a, H: DimConstraint, I: GuiBlock<'a, DimChildSets, H>> GuiBlock<'a, DimParentSets, H> for HCenterGuiBlock<I> {
        type Sized = HCenterSizedGuiBlock<I::Sized>;

        fn size(self, w: f32, h_in: H::In, scale: f32) -> ((), H::Out, Self::Sized) {
            let (inner_w, h_out, inner_sized) = self.inner.size((), h_in, scale);
            let sized = HCenterSizedGuiBlock {
                x_translate: (w - inner_w) * self.frac,
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


    // ==== TODO dedupe somehow ====


    pub fn v_center_gui_block<'a, W: DimConstraint, I: GuiBlock<'a, W, DimChildSets>>(frac: f32, inner: I) -> impl GuiBlock<'a, W, DimParentSets> {
        VCenterGuiBlock {
            frac,
            inner,
        }
    }

    struct VCenterGuiBlock<I> {
        frac: f32,
        inner: I,
    }

    impl<'a, W: DimConstraint, I: GuiBlock<'a, W, DimChildSets>> GuiBlock<'a, W, DimParentSets> for VCenterGuiBlock<I> {
        type Sized = VCenterSizedGuiBlock<I::Sized>;

        fn size(self, w_in: W::In, h: f32, scale: f32) -> (W::Out, (), Self::Sized) {
            let (w_out, inner_h, inner_sized) = self.inner.size(w_in, (), scale);
            let sized = VCenterSizedGuiBlock {
                y_translate: (h - inner_h) * self.frac,
                inner: inner_sized,
            };
            (w_out, (), sized)
        }
    }


    struct VCenterSizedGuiBlock<I> {
        y_translate: f32,
        inner: I,
    }

    impl<'a, I: SizedGuiBlock<'a>> SizedGuiBlock<'a> for VCenterSizedGuiBlock<I> {
        fn visit_nodes<T: GuiVisitorTarget<'a>>(self, mut visitor: GuiVisitor<'_, T>) {
            self.inner.visit_nodes(visitor.reborrow()
                .translate([0.0, self.y_translate]));
        }
    }

}


mod stack_block {
    use super::*;
    use std::iter::repeat;

    pub fn v_stack_gui_block<'a, I: GuiBlockSeq<'a, DimParentSets, DimChildSets>>(unscaled_gap: f32, items: I) -> impl GuiBlock<'a, DimParentSets, DimChildSets> {
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
        fn draw(mut self, _: &Renderer, mut canvas: Canvas2<'a, '_>) {
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

    pub fn modifier_gui_block<'a, W: DimConstraint, H: DimConstraint, M: Into<Modifier2>, I: GuiBlock<'a, W, H>>(modifier: M, inner: I) -> impl GuiBlock<'a, W, H> {
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
    use super::*;
    use graphics::frame_content::{
        TextBlock,
        TextSpan,
        LayedOutTextBlock,
        FontId,
        HAlign,
        VAlign,
    };


    /// The `UiText` drop shadow will be offset from the actual text by 1 /
    /// `SHADOW_DROP_DIVISOR` of the font height in the downwards-right direction.
    const SHADOW_DROP_DIVISOR: f32 = 8.0;

    /// The `UiText` drop shadow will be tinted by this color.
    const SHADOW_DROP_COLOR: Rgba<f32> = Rgba {
        r: 0.25,
        g: 0.25,
        b: 0.25,
        a: 1.0,
    };

    /// When we ask `ab_glyph` to lay out our text with bottom/right alignment,
    /// there's this gap between where it puts the text and the actual bottom-right
    /// corner. For now, we use this hack to fix it.
    const BOTTOM_RIGHT_MYSTERY_GAP: Extent2<f32> =
        Extent2 {
            w: 2.0 / 8.0,
            h: 3.0 / 8.0,
        };


    pub struct TextGuiBlock {
        spans: Vec<TextGuiBlockSpan>,
        unscaled_font_size: f32,
        h_align: HAlign,
        v_align: VAlign,
        wrap: bool,

        cached: Option<TextGuiBlockCached>,
    }

    impl TextGuiBlock {
        pub fn new(spans: Vec<TextGuiBlockSpan>, unscaled_font_size: f32, h_align: HAlign, v_align: VAlign, wrap: bool) -> Self {
            TextGuiBlock {
                spans,
                unscaled_font_size,
                h_align,
                v_align,
                wrap,

                cached: None,
            }
        }
    }

    pub struct TextGuiBlockSpan {
        pub text: String,
        pub font: FontId,
        //pub unscaled_font_size: f32, TODO what we need is to sort of just take manual control of text block border logic
        pub color: Rgba<f32>,
    }

    struct TextGuiBlockCached {
        scale: f32,
        wrap_width: Option<f32>,
        layed_out: LayedOutTextBlock,
    }

    impl<'a> GuiBlock<'a, DimParentSets, DimParentSets> for &'a mut TextGuiBlock {
        type Sized = TextSizedGuiBlock<'a>;

        fn size(self, w: f32, h: f32, scale: f32) -> ((), (), Self::Sized) {
            let sized = TextSizedGuiBlock {
                block: self,
                size: Extent2 { w, h },
                scale,
            }; // TODO factor out this type of thing
            ((), (), sized)
        }
    }

    pub struct TextSizedGuiBlock<'a> {
        block: &'a mut TextGuiBlock,
        size: Extent2<f32>,
        scale: f32,
    }

    impl<'a> GuiNode<'a> for TextSizedGuiBlock<'a> { // TODO hey hold on, does this reference even have to be 'a?
        fn draw(mut self, renderer: &Renderer, mut canvas: Canvas2<'a, '_>) {
            let wrap_width =
                if self.block.wrap { Some(self.size.w) }
                else { None };

            if let &Some(ref cached) = &self.block.cached {
                if cached.wrap_width != wrap_width || cached.scale != self.scale {
                    self.block.cached = None;
                }
            }

            if self.block.cached.is_none() {
                self.block.cached = Some(TextGuiBlockCached {
                    scale: self.scale,
                    wrap_width,
                    layed_out: renderer.lay_out_text(&TextBlock {
                        spans: &self.block.spans.iter()
                            .map(|span| TextSpan {
                                text: &span.text,
                                font: span.font,
                                font_size: self.block.unscaled_font_size * self.scale,
                                color: span.color,
                            })
                            .collect::<Vec<_>>(),
                        h_align: self.block.h_align,
                        v_align: self.block.v_align,
                        wrap_width,
                    }),
                });
            }

            let layed_out = &self.block.cached.as_ref().unwrap().layed_out;

            let align_sign = Vec2 {
                x: self.block.h_align.sign(),
                y: self.block.v_align.sign(),
            };
            let align_translate_fractional = align_sign
                .map(|n| n as f32 / 2.0 + 0.5);
            let align_translate = align_translate_fractional * self.size;

            let mystery_gap_adjust_translate =
                align_translate_fractional
                * self.block.unscaled_font_size
                * self.scale
                * BOTTOM_RIGHT_MYSTERY_GAP;
            
            let shadow_drop = self.block.unscaled_font_size / SHADOW_DROP_DIVISOR * self.scale;
            let text_shadow_translate = align_sign
                .map(|n| (n as f32 / -2.0 + 0.5) * shadow_drop);
            let text_main_translate = align_sign
                .map(|n| (n as f32 / -2.0 - 0.5) * shadow_drop);

            let mut canvas = canvas.reborrow()
                .translate(align_translate)
                .translate(mystery_gap_adjust_translate);
            canvas.reborrow()
                .translate(text_shadow_translate)
                .color(SHADOW_DROP_COLOR)
                .draw_text(&layed_out);
            canvas.reborrow()
                .translate(text_main_translate)
                .draw_text(&layed_out);
        }
    }
}


mod mc_title_block {
    use super::*;
    use graphics::{
        frame_content::{Mesh, GpuImageArray, Vertex, Triangle},
    };
    use std::sync::Arc;
    use rand::Rng;
    use image::DynamicImage;

    const TITLE_PIXELS: &'static [&'static str] = &[
        "█   █ █ █   █ ███ ███ ███ ███ ███ ███",
        "██ ██ █ ██  █ █   █   █ █ █ █ █    █ ",
        "█ █ █ █ █ █ █ ██  █   ██  ███ ██   █ ",
        "█   █ █ █  ██ █   █   █ █ █ █ █    █ ",
        "█   █ █ █   █ ███ ███ █ █ █ █ █    █ ",
    ];

    const FACES_PER_TITLE_PIXEL: usize = 5;
    const VERTS_PER_FACE: usize = 4;

    const POS_X_FACE_POS: [Vec3<f32>; VERTS_PER_FACE] = [
        Vec3 { x: 1.0, y: 1.0, z: 0.0 },
        Vec3 { x: 1.0, y: 1.0, z: 1.0 },
        Vec3 { x: 1.0, y: 0.0, z: 1.0 },
        Vec3 { x: 1.0, y: 0.0, z: 0.0 },
    ];
    const POS_Y_FACE_POS: [Vec3<f32>; VERTS_PER_FACE] = [
        Vec3 { x: 0.0, y: 1.0, z: 1.0 },
        Vec3 { x: 1.0, y: 1.0, z: 1.0 },
        Vec3 { x: 1.0, y: 1.0, z: 0.0 },
        Vec3 { x: 0.0, y: 1.0, z: 0.0 },
    ];  
    const POS_Z_FACE_POS: [Vec3<f32>; VERTS_PER_FACE] = [
        Vec3 { x: 1.0, y: 1.0, z: 1.0 },
        Vec3 { x: 0.0, y: 1.0, z: 1.0 },
        Vec3 { x: 0.0, y: 0.0, z: 1.0 },
        Vec3 { x: 1.0, y: 0.0, z: 1.0 },
    ];
    const NEG_X_FACE_POS: [Vec3<f32>; VERTS_PER_FACE] = [
        Vec3 { x: 0.0, y: 1.0, z: 1.0 },
        Vec3 { x: 0.0, y: 1.0, z: 0.0 },
        Vec3 { x: 0.0, y: 0.0, z: 0.0 },
        Vec3 { x: 0.0, y: 0.0, z: 1.0 },
    ];
    const NEG_Y_FACE_POS: [Vec3<f32>; VERTS_PER_FACE] = [
        Vec3 { x: 0.0, y: 0.0, z: 0.0 },
        Vec3 { x: 1.0, y: 0.0, z: 0.0 },
        Vec3 { x: 1.0, y: 0.0, z: 1.0 },
        Vec3 { x: 0.0, y: 0.0, z: 1.0 },
    ];
    const NEG_Z_FACE_POS: [Vec3<f32>; VERTS_PER_FACE] = [
        Vec3 { x: 0.0, y: 1.0, z: 0.0 },
        Vec3 { x: 1.0, y: 1.0, z: 0.0 },
        Vec3 { x: 1.0, y: 0.0, z: 0.0 },
        Vec3 { x: 0.0, y: 0.0, z: 0.0 },
    ];

    const FACE_TEX: [Vec2<f32>; 4] = [
        Vec2 { x: 0.0, y: 0.0 },
        Vec2 { x: 1.0, y: 0.0 },
        Vec2 { x: 1.0, y: 1.0 },
        Vec2 { x: 0.0, y: 1.0 },
    ];

    const FACE_TRIANGLES: [Triangle; 2] = [
        Triangle([0, 1, 2]),
        Triangle([0, 2, 3]),
    ];


    #[derive(Debug, Clone)]
    pub struct McTitleGuiBlock {
        title_cam_height: f32,
        title_cam_distance: f32,
        title_cam_fov: f32,
        title_angle: f32,
        title_translate: Vec3<f32>,
        
        title_pixel_mesh: Arc<Mesh>,
        title_pixel_texture: GpuImageArray,
        title_pixel_positions: Vec<Vec3<f32>>,
    }

    impl McTitleGuiBlock {
        pub fn new<R: Rng>(
            renderer: &Renderer,
            rng: &mut R,
            raw_title_pixel_texture: DynamicImage,
        ) -> Self
        {
            let title_pixel_vertices: Vec<Vertex> = [
                (POS_X_FACE_POS, 0.5),
                (POS_Y_FACE_POS, 0.5),
                (NEG_X_FACE_POS, 0.5),
                (NEG_Y_FACE_POS, 0.5),
                (NEG_Z_FACE_POS, 1.0),
            ]
                .into_iter()
                .flat_map(|(face_pos, brightness)| face_pos.zip(FACE_TEX)
                    .map(|(pos, tex)| Vertex {
                        pos,
                        tex,
                        color: Rgba::new(brightness, brightness, brightness, 1.0),
                        tex_index: 0,
                    }))
                .collect();
            let title_pixel_triangles: Vec<Triangle> = (0..FACES_PER_TITLE_PIXEL)
                .flat_map(|face_i| FACE_TRIANGLES
                    .map(|tri| tri
                        .map(|vert_i| face_i * VERTS_PER_FACE + vert_i)))
                .collect();
            let title_pixel_mesh = Arc::new(Mesh {
                vertices: renderer.create_gpu_vec_init(&title_pixel_vertices),
                triangles: renderer.create_gpu_vec_init(&title_pixel_triangles),
            });

            let title_pixel_texture = renderer
                .load_image_array_raw(
                    Extent2::new(16, 16), // TODO should be, like, dynamic
                    [raw_title_pixel_texture],
                );

            let title_pixel_positions = TITLE_PIXELS
                .iter()
                .enumerate()
                .flat_map(|(r, &column)| column
                    .chars()
                    .enumerate()
                    .filter(|&(_, c)| c == '█')
                    .map(move |(c, _)| (r, c)))
                .map(|(r, c)| Vec3 {
                    x: c as f32,
                    y: (TITLE_PIXELS.len() - r - 1) as f32,
                    //z: 0.0, // TODO
                    z: rng.gen_range(-75.0..=-40.0),
                })
                .collect();

            McTitleGuiBlock {
                title_cam_distance: -45.0 / 2.5,
                title_cam_height: -10.0 / 6.0 * 0.0,
                title_cam_fov: 1.38753,
                title_angle: 0.48869 / 2.0,
                title_translate: [
                    -(TITLE_PIXELS[0].chars().count() as f32) / 2.0,
                    -(TITLE_PIXELS.len() as f32) / 2.0,
                    0.0,
                ].into(),

                title_pixel_mesh,
                title_pixel_texture,
                title_pixel_positions,
            }
        }

        pub fn update(&mut self, elapsed: f32) {
            for pos in &mut self.title_pixel_positions {
                pos.z = f32::min(0.0, pos.z + 75.0 * elapsed);
            }
        }
    }

    impl<'a> GuiBlock<'a, DimParentSets, DimParentSets> for &'a McTitleGuiBlock {
        type Sized = McTitleSizedGuiBlock<'a>;

        fn size(self, w: f32, h: f32, scale: f32) -> ((), (), Self::Sized) {
            let sized = McTitleSizedGuiBlock {
                block: self,
                size: Extent2 { w, h },
                scale,
            };
            ((), (), sized)
        }
    }

    pub struct McTitleSizedGuiBlock<'a> {
        block: &'a McTitleGuiBlock,
        size: Extent2<f32>,
        scale: f32,
    }

    impl<'a> GuiNode<'a> for McTitleSizedGuiBlock<'a> {
        fn draw(self, _: &Renderer, mut canvas: Canvas2<'a, '_>) {
            /*canvas.reborrow()
                .color([1.0, 0.0, 0.0, 0.1])
                .draw_solid(self.size);*/
            let mut canvas = canvas.reborrow()
                .begin_3d_perspective(
                    self.size,
                    [0.0, self.block.title_cam_height, self.block.title_cam_distance],
                    Quaternion::identity(),
                    self.block.title_cam_fov, // TODO horizontal field of view
                )
                .rotate(Quaternion::rotation_x(self.block.title_angle))
                .translate(self.block.title_translate);
            for &pos in &self.block.title_pixel_positions {
                canvas.reborrow()
                    .translate(pos)
                    .draw_mesh(&self.block.title_pixel_mesh, &self.block.title_pixel_texture);
            }
        }
    }
}


mod mc_splash_text_block {
    use super::*;
    use crate::game::Cosine;
    use graphics::frame_content::{
        TextBlock,
        TextSpan,
        LayedOutTextBlock,
        FontId,
        HAlign,
        VAlign,
    };


    pub struct McSplashTextGuiBlock {
        layed_out: LayedOutTextBlock,
        bounce_scale: Cosine,
        translate_frac: Vec2<f32>,
    }

    impl McSplashTextGuiBlock {
        pub fn new(renderer: &Renderer, font: FontId) -> Self {
            let text = "Splash text!";
            let translate_frac = Vec2 {
                x: 0.75,
                y: 5.0 / 16.0,
            };

            let layed_out = renderer.lay_out_text(&TextBlock {
                spans: &[TextSpan {
                    text,
                    font,
                    font_size: 32.0, // TODO we could revalidate upon scale change and maybe should at some point but where we're doing all sorts of constant things that mess of pixel perfectness like rotation or 3D and not re-rasterizing each time then this is largely fine for now but this should be considered a TODO item
                    color: Rgba::yellow(),
                }],
                h_align: HAlign::Center,
                v_align: VAlign::Center,
                wrap_width: None,
            });
            let bounce_scale = Cosine::new(1.0 / 2.0);

            McSplashTextGuiBlock {
                layed_out,
                bounce_scale,
                translate_frac,
            }
        }

        pub fn update(&mut self, elapsed: f32) {
            self.bounce_scale.add_to_input(elapsed);
        }
    }

    impl<'a> GuiBlock<'a, DimParentSets, DimParentSets> for &'a McSplashTextGuiBlock {
        type Sized = McSplashTextSizedGuiBlock<'a>;

        fn size(self, w: f32, h: f32, scale: f32) -> ((), (), Self::Sized) {
            let sized = McSplashTextSizedGuiBlock {
                block: self,
                size: Extent2 { w, h },
                scale,
            };
            ((), (), sized)
        }
    }

    pub struct McSplashTextSizedGuiBlock<'a> {
        block: &'a McSplashTextGuiBlock,
        size: Extent2<f32>,
        scale: f32,
    }

    impl<'a> GuiNode<'a> for McSplashTextSizedGuiBlock<'a> {
        fn draw(self, _: &Renderer, mut canvas: Canvas2<'a, '_>) {
            canvas.reborrow()
                .translate(self.size * self.block.translate_frac)
                .scale(self.scale)
                .scale(self.block.bounce_scale.get().abs() / 16.0 + 1.0)
                .rotate(f32::to_radians(22.5))
                .draw_text(&self.block.layed_out);
        }
    }
}


mod cursor_is_over_tracker_block {
    use super::*;

    #[derive(Debug, Clone)]
    pub struct CursorIsOverTrackerGuiBlock {
        pub cursor_is_over: bool,
    }

    impl CursorIsOverTrackerGuiBlock {
        pub fn new() -> Self {
            CursorIsOverTrackerGuiBlock {
                cursor_is_over: false,
            }
        }
    }

    impl<'a> GuiBlock<'a, DimParentSets, DimParentSets> for &'a mut CursorIsOverTrackerGuiBlock {
        type Sized = CursorIsOverTrackerSizedGuiBlock<'a>;

        fn size(self, w: f32, h: f32, scale: f32) -> ((), (), Self::Sized) {
            let sized = CursorIsOverTrackerSizedGuiBlock {
                block: self,
                size: Extent2 { w, h },
                scale,
            };
            ((), (), sized)
        }
    }

    pub struct CursorIsOverTrackerSizedGuiBlock<'a> {
        block: &'a mut CursorIsOverTrackerGuiBlock,
        size: Extent2<f32>,
        scale: f32,
    }

    impl<'a> GuiNode<'a> for CursorIsOverTrackerSizedGuiBlock<'a> {
        fn handle_input_event(self, _: &Renderer, event: InputEvent) {
            if let InputEvent::CursorMoved(pos) = event {
                self.block.cursor_is_over =
                    pos.x >= 0.0
                    && pos.y >= 0.0
                    && pos.x <= self.size.w
                    && pos.y <= self.size.h;
            }
        }
    }
}
