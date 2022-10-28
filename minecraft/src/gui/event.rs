//! Types which may exist transiently to convey GUI events.


use vek::*;


/// Amount of scrolling.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ScrolledAmount {
    Pixels(Vec2<f32>),
    Lines(Vec2<f32>),
}

impl ScrolledAmount {
    /// Convert to pixels, using the given line-to-pixel conversion if is
    /// `Lines`.
    pub fn to_pixels(self, font_size: impl Into<Extent2<f32>>) -> Vec2<f32> {
        match self {
            ScrolledAmount::Pixels(v) => v,
            ScrolledAmount::Lines(l) => l * font_size.into(),
        }
    }
}
