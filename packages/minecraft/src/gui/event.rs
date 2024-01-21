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

/// Corresponds to a key press, represents the associated meaning if the key
/// press is to try to be interpreted in the context of typing text.
#[derive(Copy, Clone, Debug)]
pub enum TypingInput<'a> {
    /// The key press may be an attempt to type the character(s).
    Text(&'a str),
    /// The key press may be an attempt to type the text "control" characters. 
    Control(TypingControl),
}

/// Text "control" characters that may be typed.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum TypingControl {
    /// Typing an enter character.
    Enter,
    /// Typing a backspace character.
    Backspace,
    /// Typing a delete character.
    Delete,
    /// Typing a tab character.
    Tab,
}
