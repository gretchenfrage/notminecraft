//! General-ish UI framework.

use graphics::{
    Renderer,
    frame_content::Canvas2,
};


pub mod text;
pub mod text_block;
pub mod margin_block;
pub mod tile_9_block;
pub mod layer_block;
pub mod stable_unscaled_size_block;
pub mod center_block;
pub mod stack_block;
pub mod tile_block;


#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct False;

impl Into<bool> for False {
    fn into(self) -> bool {
        false
    }
}


pub trait UiBlock {
    type WidthChanged: Copy + Into<bool>;
    type HeightChanged: Copy + Into<bool>;

    fn draw<'a>(&'a self, canvas: Canvas2<'a, '_>);

    fn width(&self) -> f32;

    fn height(&self) -> f32;

    fn scale(&self) -> f32;

    fn set_scale(&mut self, renderer: &Renderer, scale: f32) -> (
        Self::WidthChanged,
        Self::HeightChanged,
    );
}

pub trait UiBlockSetWidth {
    fn set_width(&mut self, renderer: &Renderer, width: f32);
}

pub trait UiBlockSetHeight {
    fn set_height(&mut self, renderer: &Renderer, height: f32);
}


pub trait UiBlockItems {
    type WidthChanged: Copy + Into<bool>;
    type HeightChanged: Copy + Into<bool>;

    fn len(&self) -> usize;

    fn draw<'a>(&'a self, i: usize, canvas: Canvas2<'a, '_>);

    fn width(&self, i: usize) -> f32;

    fn height(&self, i: usize) -> f32;

    fn scale(&self, i: usize) -> f32;

    fn set_scale(&mut self, i: usize, renderer: &Renderer, scale: f32) -> (
        Self::WidthChanged,
        Self::HeightChanged,
    );
}

pub trait UiBlockItemsSetWidth {
    fn set_width(&mut self, i: usize, renderer: &Renderer, width: f32);
}

pub trait UiBlockItemsSetHeight {
    fn set_height(&mut self, i: usize, renderer: &Renderer, height: f32);
}

macro_rules! ui_block_items_struct {
    (
        settable_width=$settable_width:ident,
        settable_height=$settable_height:ident,
        $struct:ident {$(
            $field:ident: $type:ty
        ),*$(,)?}
    )=>{
        impl $crate::ui::UiBlockItems for $struct {
            type WidthChanged = $crate::ui::ui_block_items_struct!(
                @DimSizeChanged settable=$settable_width
            );
            type HeightChanged = $crate::ui::ui_block_items_struct!(
                @DimSizeChanged settable=$settable_height
            );

            fn len(&self) -> usize {
                0 $( + {
                    let $field = 1;
                    $field
                })*
            }

            fn draw<'a>(&'a self, mut i: usize, canvas: Canvas2<'a, '_>) {
                $({
                    if i == 0 {
                        return <$type as $crate::ui::UiBlock>::draw(
                            &self.$field,
                            canvas,
                        );
                    }
                    i -= 1;
                })*
                panic!("invalid index");
            }

            fn width(&self, mut i: usize) -> f32 {
                $({
                    if i == 0 {
                        return <$type as $crate::ui::UiBlock>::width(
                            &self.$field
                        );
                    }
                    i -= 1;
                })*
                panic!("invalid index");
            }

            fn height(&self, mut i: usize) -> f32 {
                $({
                    if i == 0 {
                        return <$type as $crate::ui::UiBlock>::height(
                            &self.$field
                        );
                    }
                    i -= 1;
                })*
                panic!("invalid index");
            }

            fn scale(&self, mut i: usize) -> f32 {
                $({
                    if i == 0 {
                        return <$type as $crate::ui::UiBlock>::scale(
                            &self.$field
                        );
                    }
                    i -= 1;
                })*
                panic!("invalid index");
            }

            fn set_scale(
                &mut self,
                mut i: usize,
                renderer: &::graphics::Renderer,
                scale: f32,
            ) -> (
                <Self as $crate::ui::UiBlockItems>::WidthChanged,
                <Self as $crate::ui::UiBlockItems>::HeightChanged,
            )
            {
                $({
                    if i == 0 {
                        return <$type as $crate::ui::UiBlock>::set_scale(
                            &mut self.$field,
                            renderer,
                            scale,
                        );
                    }
                    i -= 1;
                })*
                panic!("invalid index");
            }
        }

        $crate::ui::ui_block_items_struct!(
            @impl_dim_size_changed
            settable=$settable_width,
            $crate::ui::UiBlockSetWidth,
            $crate::ui::UiBlockItemsSetWidth,
            set_width,
            $struct {$(
                $field: $type,
            )*}
        );

        $crate::ui::ui_block_items_struct!(
            @impl_dim_size_changed
            settable=$settable_height,
            $crate::ui::UiBlockSetHeight,
            $crate::ui::UiBlockItemsSetHeight,
            set_height,
            $struct {$(
                $field: $type,
            )*}
        );
    };
    (@DimSizeChanged settable=true)=>{ $crate::ui::False };
    (@DimSizeChanged settable=false)=>{ bool };
    (
        @impl_dim_size_changed
        settable=true,
        $inner_trait:path,
        $trait:path,
        $method:ident,
        $struct:ident {$(
            $field:ident: $type:ty
        ),*$(,)?}
    )=>{
        impl $trait for $struct {
            fn $method(
                &mut self,
                mut i: usize,
                renderer: &::graphics::Renderer,
                n: f32,
            ) {
                $({
                    if i == 0 {
                        return <$type as $inner_trait>::$method(
                            &mut self.$field,
                            renderer,
                            n,
                        );
                    }
                    i -= 1;
                })*
                panic!("invalid index");
            }
        }
    };
    (
        @impl_dim_size_changed
        settable=false,
        $inner_trait:path,
        $trait:path,
        $method:ident,
        $struct:ident {$(
            $field:ident: $type:ty
        ),*$(,)?}
    )=>{};
}

pub (crate) use ui_block_items_struct;

/*
impl<I: UiBlock> UiBlockItems for Vec<I> {
    type WidthChanged = <I as UiBlock>::WidthChanged;
    type HeightChanged = <I as UiBlock>::HeightChanged;

    fn len(&self) -> usize {
        Vec::len(self)
    }

    fn draw<'a>(&'a self, i: usize, canvas: Canvas2<'a, '_>) {
        self[i].draw(canvas)
    }

    fn width(&self, i: usize) -> f32 {
        self[i].width()
    }

    fn height(&self, i: usize) -> f32 {
        self[i].height()
    }

    fn scale(&self, i: usize) -> f32 {
        self[i].scale()
    }

    fn set_scale(&mut self, i: usize, renderer: &Renderer, scale: f32) -> (
        Self::WidthChanged,
        Self::HeightChanged,
    )
    {
        self[i].set_scale(renderer, scale)
    }
}

impl<I: UiBlockSetWidth> UiBlockItemsSetWidth for Vec<I> {
    fn set_width(&mut self, i: usize, renderer: &Renderer, width: f32) {
        self[i].set_width(renderer, width)
    }
}

impl<I: UiBlockSetHeight> UiBlockItemsSetHeight for Vec<I> {
    fn set_height(&mut self, i: usize, renderer: &Renderer, height: f32) {
        self[i].set_height(renderer, height)
    }
}
*/

macro_rules! ui_block_items_tuple {
    ($($i:ident: $t:ident),*$(,)?)=>{
        impl<
            A: UiBlock,
            $(
                $t: UiBlock<
                    WidthChanged=<A as UiBlock>::WidthChanged,
                    HeightChanged=<A as UiBlock>::HeightChanged,
                >,
            )*
        > UiBlockItems for (
            A,
            $(
                $t,
            )*
        )
        {
            type WidthChanged = <A as UiBlock>::WidthChanged;
            type HeightChanged = <A as UiBlock>::HeightChanged;

            fn len(&self) -> usize {
                1 $( + {
                    let $i = 1;
                    $i
                } )*
            }

            fn draw<'a>(&'a self, mut i: usize, canvas: Canvas2<'a, '_>) {
                let &(
                    ref a,
                    $( ref $i, )*
                ) = self;

                if i == 0 {
                    return a.draw(canvas);
                }
                i -= 1;

                $({
                    if i == 0 {
                        return $i.draw(canvas);
                    }
                    i -= 1;
                })*

                panic!("invalid index");
            }

            fn width(&self, mut i: usize) -> f32 {
                let &(
                    ref a,
                    $( ref $i, )*
                ) = self;

                if i == 0 {
                    return a.width();
                }
                i -= 1;

                $({
                    if i == 0 {
                        return $i.width();
                    }
                    i -= 1;
                })*
                
                panic!("invalid index");
            }

            fn height(&self, mut i: usize) -> f32 {
                let &(
                    ref a,
                    $( ref $i, )*
                ) = self;

                if i == 0 {
                    return a.height();
                }
                i -= 1;

                $({
                    if i == 0 {
                        return $i.height();
                    }
                    i -= 1;
                })*
                
                panic!("invalid index");
            }

            fn scale(&self, mut i: usize) -> f32 {
                let &(
                    ref a,
                    $( ref $i, )*
                ) = self;

                if i == 0 {
                    return a.scale();
                }
                i -= 1;

                $({
                    if i == 0 {
                        return $i.scale();
                    }
                    i -= 1;
                })*
                
                panic!("invalid index");
            }

            fn set_scale(&mut self, mut i: usize, renderer: &Renderer, scale: f32) -> (
                Self::WidthChanged,
                Self::HeightChanged,
            )
            {
                let &mut (
                    ref mut a,
                    $( ref mut $i, )*
                ) = self;

                if i == 0 {
                    return a.set_scale(renderer, scale);
                }
                i -= 1;

                $({
                    if i == 0 {
                        return $i.set_scale(renderer, scale);
                    }
                    i -= 1;
                })*
                
                panic!("invalid index");
            }
        }

        impl<
            A: UiBlockSetWidth,
            $(
                $t: UiBlockSetWidth,
            )*
        > UiBlockItemsSetWidth for (
            A,
            $(
                $t,
            )*
        )
        {
            fn set_width(&mut self, mut i: usize, renderer: &Renderer, width: f32) {
                let &mut (
                    ref mut a,
                    $( ref mut $i, )*
                ) = self;

                if i == 0 {
                    return a.set_width(renderer, width);
                }
                i -= 1;

                $({
                    if i == 0 {
                        return $i.set_width(renderer, width);
                    }
                    i -= 1;
                })*
                
                panic!("invalid index");
            }
        }

        impl<
            A: UiBlockSetHeight,
            $(
                $t: UiBlockSetHeight,
            )*
        > UiBlockItemsSetHeight for (
            A,
            $(
                $t,
            )*
        )
        {
            fn set_height(&mut self, mut i: usize, renderer: &Renderer, height: f32) {
                let &mut (
                    ref mut a,
                    $( ref mut $i, )*
                ) = self;

                if i == 0 {
                    return a.set_height(renderer, height);
                }
                i -= 1;

                $({
                    if i == 0 {
                        return $i.set_height(renderer, height);
                    }
                    i -= 1;
                })*
                
                panic!("invalid index");
            }
        }
    };
}

ui_block_items_tuple!();
ui_block_items_tuple!(b: B);
ui_block_items_tuple!(b: B, c: C);
ui_block_items_tuple!(b: B, c: C, d: D);
ui_block_items_tuple!(b: B, c: C, d: D, e: E);
ui_block_items_tuple!(b: B, c: C, d: D, e: E, f: F);
ui_block_items_tuple!(b: B, c: C, d: D, e: E, f: F, g: G);
ui_block_items_tuple!(b: B, c: C, d: D, e: E, f: F, g: G, h: H);
ui_block_items_tuple!(b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I);
ui_block_items_tuple!(b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J);
ui_block_items_tuple!(b: B, c: C, d: D, e: E, f: F, g: G, h: H, i: I, j: J, k: K);
