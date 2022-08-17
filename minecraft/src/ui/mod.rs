
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


pub use winit_main::reexports::event::{
    MouseButton,
    ElementState,
};


pub mod text;
pub mod tile_9;
pub mod menu_button;
pub mod v_stack;
pub mod h_center;
pub mod v_center;


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

#[derive(Debug, Clone)]
pub enum UiPosInputEvent {
    CursorMoved(Vec2<f32>),
    MouseInput {
        pos: Vec2<f32>,
        button: MouseButton,
        state: ElementState,
    },
}

impl UiPosInputEvent {
    pub fn map_pos<F>(self, f: F) -> Self
    where
        F: Fn(Vec2<f32>) -> Vec2<f32>
    {
        match self {
            UiPosInputEvent::CursorMoved(pos) => UiPosInputEvent::CursorMoved(f(pos)),
            UiPosInputEvent::MouseInput {
                pos,
                button,
                state,
            } => UiPosInputEvent::MouseInput {
                pos: f(pos),
                button,
                state,
            },
        }
    }
}
