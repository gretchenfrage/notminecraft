
use graphics::Renderer;
use std::collections::BTreeSet;
use vek::*;


pub use winit_main::reexports::event::{
    VirtualKeyCode,
    ScanCode,
    MouseButton,
};


#[derive(Debug, Copy, Clone)]
pub struct GuiGlobalContext<'c> {
    pub renderer: &'c Renderer,
    pub focus_level: FocusLevel,
    pub pressed_keys_semantic: &'c BTreeSet<VirtualKeyCode>,
    pub pressed_keys_physical: &'c BTreeSet<ScanCode>,
    pub pressed_mouse_buttons: &'c BTreeSet<MouseButton>,
}

#[derive(Debug, Copy, Clone)]
pub struct GuiContext<'c> {
    pub global: &'c GuiGlobalContext<'c>,
    pub cursor_pos: Option<Vec2<f32>>,
    pub size: Extent2<f32>,
    pub scale: f32,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum FocusLevel {
    Unfocused,
    Focused,
    MouseCaptured,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ScrolledAmount {
    Pixels(Vec2<f32>),
    Lines(Vec2<f32>),
}

impl ScrolledAmount {
    pub fn to_pixels(self, font_size: impl Into<Extent2<f32>>) -> Vec2<f32> {
        match self {
            ScrolledAmount::Pixels(v) => v,
            ScrolledAmount::Lines(l) => l * font_size.into(),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BlocksCursor {
    Blocks,
    DoesntBlock,
}
