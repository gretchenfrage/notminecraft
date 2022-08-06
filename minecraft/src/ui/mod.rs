
use graphics::{
    Renderer,
    frame_content::Canvas2,
    modifier::{
        Modifier2,
        Transform2,
        Clip2,
    },
};
use std::borrow::Borrow;
use vek::*;


pub mod text;


#[derive(Debug, Copy, Clone, PartialEq)]
pub struct UiSize {
    pub size: Extent2<f32>,
    pub scale: f32,
}


#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Margins {
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
}


#[derive(Debug, Clone, PartialEq)]
pub struct UiModify(Vec<Modifier2>);

impl UiModify {
    pub fn new() -> Self {
        UiModify(Vec::new())
    }

    pub fn recycle(mut self) -> Self {
        self.0.clear();
        self
    }

    pub fn modify<I: Into<Modifier2>>(mut self, modifier: I) -> Self {
        self.0.push(modifier.into());
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

    pub fn reverse_apply<V: Into<Vec2<f32>>>(
        &self,
        v: V,
    ) -> Option<Vec2<f32>> {
        let mut v = v.into();
        for modifier in self {
            v = match modifier {
                Modifier2::Transform(t) => t.reverse_apply(v),
                Modifier2::Color(_) => continue,
                Modifier2::Clip(c) => Some(v).filter(|_| c.test(v))
            }?;
        }
        Some(v)
    }
}

impl Borrow<[Modifier2]> for UiModify {
    fn borrow(&self) -> &[Modifier2] {
        &self.0
    }
}

impl<'a> IntoIterator for &'a UiModify {
    type Item = Modifier2;
    type IntoIter = std::iter::Copied<std::slice::Iter<'a, Modifier2>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter().copied()
    }    
}

/*
pub trait UiElem {
    fn tick(&mut self, renderer: &Renderer, elapsed: f32) {}

    fn draw<'a>(&self, canvas: Canvas2<'a, '_>);

    fn set_scale(&mut self, renderer: &Renderer, scale: f32);
}

pub trait UiElemSetSize {
    fn set_size(&mut self, renderer: &Renderer, size: Vec2<f32>);
}
*/
