//! Hex color literal utility.

use vek::*;


/// Convert an RGBA packed int into a normalized `Rgba<f32>`.
pub fn hex_color(hex: u32) -> Rgba<f32> {
    Rgba {
        r: ((hex & 0xFF000000) >> 24) as f32 / 255.0,
        g: ((hex & 0x00FF0000) >> 16) as f32 / 255.0,
        b: ((hex & 0x0000FF00) >> 8) as f32 / 255.0,
        a: (hex & 0x000000FF) as f32 / 255.0,
    }
}
