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
