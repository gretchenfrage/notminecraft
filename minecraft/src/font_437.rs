//! An implementation of `ab_glyph::Font` based on a code page 437 image.

// this has been a very useful resource for this:
// https://freetype.org/freetype2/docs/glyphs/glyphs-3.html

use std::{
    ops::Index,
    borrow::Borrow,
};
use anyhow::*;
use ab_glyph::{
    Font,
    GlyphId,
    Outline,
    CodepointIdIter,
    GlyphImage,
    Rect,
    Point,
    OutlineCurve,
};
use image::{
    DynamicImage,
    GenericImageView,
};
use oem_cp::code_table::{
    ENCODING_TABLE_CP437 as ENCODE_TABLE,
    DECODING_TABLE_CP437 as DECODE_TABLE,
};


/// Bitpacked equivalent of [bool; 8].
#[derive(Copy, Clone)]
struct Bits8(u8);

impl Bits8 {
    const ZEROES: Self = Bits8(0);

    fn get(&self, i: usize) -> bool {
        assert!(i < 8);
        (self.0 & (1 << i)) != 0
    }

    fn set(&mut self, i: usize, value: bool) {
        assert!(i < 8);
        self.0 &= !(1 << i);
        if value {
            self.0 |= 1 << i;
        }
    }
}

impl Index<usize> for Bits8 {
    type Output = bool;

    fn index(&self, i: usize) -> &bool {
        if self.get(i) { &true } else { &false }
    }
}


/// Implementation of `ab_glyph::Font` based on a code page 437 image.
pub struct Font437 {
    // glyph pixel value array for each of the 256 code points
    glyphs: Box<[[Bits8; 8]; 256]>,
    // number of 0 columns on the left side of each glyph
    left_zero: Box<[u8; 256]>,
    // number of 0 rows on the top of each glyph
    top_zero: Box<[u8; 256]>,
    // number of 0 rows on the bottom of each glyph
    bottom_zero: Box<[u8; 256]>,
    // width of the range of non-0 columns of each glyph
    nonzero_width: Box<[u8; 256]>,
}

impl Font437 {
    pub fn new(file_data: impl AsRef<[u8]>) -> Result<Self> {
        let image = image::load_from_memory(file_data.as_ref())?;
        Self::new_raw(image)
    }

    pub fn new_raw(image: impl Borrow<DynamicImage>) -> Result<Self> {
        let image = image.borrow();

        // verify dimensions
        ensure!(image.width() == 128, "437 font wrong width");
        ensure!(image.height() == 128, "437 font wrong height");

        // convert from image to bitmapped glyph array
        let mut glyphs = Box::new([[Bits8::ZEROES; 8]; 256]);
        for y1 in 0..16 {
            for x1 in 0..16 {
                let cp = y1 * 16 + x1;
                let glyph = &mut glyphs[cp];
                for y2 in 0..8 {
                    for x2 in 0..8 {
                        let rgba = image
                            .get_pixel(
                                (x1 * 8 + x2) as u32,
                                (y1 * 8 + y2) as u32,
                            );
                        glyph[x2].set(y2, rgba[3] != 0);
                    }
                }
            }
        }

        // compute left zero
        let mut left_zero = Box::new([0; 256]);
        for cp in 0..256 {
            let glyph = &glyphs[cp];
            left_zero[cp] = (0..8)
                .take_while(|&x| (0..8)
                    .all(|y| !glyph[x][y]))
                .count() as u8;
        }

        // compute top zero
        let mut top_zero = Box::new([0; 256]);
        for cp in 0..256 {
            let glyph = &glyphs[cp];
            top_zero[cp] = (0..8)
                .take_while(|&y| (0..8)
                    .all(|x| !glyph[x][y]))
                .count() as u8;
        }

        // compute bottom zero
        let mut bottom_zero = Box::new([0; 256]);
        for cp in 0..256 {
            let glyph = &glyphs[cp];
            bottom_zero[cp] = (0..8)
                .rev()
                .take_while(|&y| (0..8)
                    .all(|x| !glyph[x][y]))
                .count() as u8;
        }

        // compute nonzero width
        let mut nonzero_width = Box::new([0; 256]);
        for cp in 0..256 {
            let glyph = &glyphs[cp];
            nonzero_width[cp] = (0..8)
                .rev()
                .find(|&x| (0..8)
                    .any(|y| glyph[x][y]))
                .map(|x| x as u8 + 1 - left_zero[cp])
                .unwrap_or(0);

        }


        Ok(Font437 {
            glyphs,
            left_zero,
            top_zero,
            bottom_zero,
            nonzero_width,
        })
    }
}

impl Font for Font437 {
    fn units_per_em(&self) -> Option<f32> {
        Some(8.0)
    }

    fn ascent_unscaled(&self) -> f32 {
        7.0
    }

    fn descent_unscaled(&self) -> f32 {
        -1.0
    }

    fn line_gap_unscaled(&self) -> f32 {
        1.0
    }

    fn glyph_id(&self, c: char) -> GlyphId {
        const NO_GLYPH: char = '♦';
        let cp = ENCODE_TABLE.get(&c).copied().unwrap_or(ENCODE_TABLE[&NO_GLYPH]);
        GlyphId(cp as u16)
    }

    fn h_advance_unscaled(&self, id: GlyphId) -> f32 {
        let cp = id.0 as usize;
        let nonzero_width = self.nonzero_width[cp];
        if nonzero_width == 0 {
            8.0
        } else {
            nonzero_width as f32 + self.left_zero[cp] as f32 * 2.0 + 1.0
        }
    }

    fn h_side_bearing_unscaled(&self, id: GlyphId) -> f32 {
        let cp = id.0 as usize;
        let nonzero_width = self.nonzero_width[cp];
        if nonzero_width == 0 {
            0.0
        } else {
            self.left_zero[cp] as f32 + 0.5
        }
    }

    fn v_advance_unscaled(&self, _id: GlyphId) -> f32 {
        0.0
    }

    fn v_side_bearing_unscaled(&self, id: GlyphId) -> f32 {
        let cp = id.0 as usize;
        let top_zero = self.top_zero[cp];
        if top_zero == 8 {
            0.0
        } else {
            7.0 - top_zero as f32
        }
    }

    fn kern_unscaled(&self, first: GlyphId, second: GlyphId) -> f32 {
        let g1 = &self.glyphs[first.0 as usize];
        let g2 = &self.glyphs[second.0 as usize];
        (0..8)
            .map(|y|
                (0..8).rev().take_while(|&x| !g1[x][y]).count()
                +
                (0..8).take_while(|&x| !g2[x][y]).count()
            )
            .min().unwrap() as f32 * -1.0
    }

    fn outline(&self, id: GlyphId) -> Option<Outline> {
        let cp = id.0 as usize;
        Some(if self.nonzero_width[cp] == 0 {
            Outline {
                bounds: Rect {
                    min: Point { x: 0.0, y: 0.0 },
                    max: Point { x: 0.0, y: 0.0 },
                },
                curves: Vec::new(),
            }
        } else {
            let glyph = &self.glyphs[cp];

            let min_x = 0.5 + self.left_zero[cp] as f32;
            let bounds = Rect {
                min: Point {
                    x: min_x,
                    y: -7.0 + self.top_zero[cp] as f32,
                },
                max: Point {
                    x: min_x + self.nonzero_width[cp] as f32,
                    y: 1.0 - self.bottom_zero[cp] as f32,
                },
            };

            let curves = (0..8)
                .flat_map(|y| (0..8)
                    .map(move |x| (x, y)))
                .filter(|&(x, y)| glyph[x][y])
                .flat_map(|(x, y)| {
                    let x = x as f32 + 0.5;
                    let y = y as f32 - 7.0;

                    let p1 = Point { x, y };
                    let p2 = Point { x: x + 1.0, y };
                    let p3 = Point { x: x + 1.0, y: y + 1.0 };
                    let p4 = Point { x, y: y + 1.0 };
                    [
                        OutlineCurve::Line(p1, p2),
                        OutlineCurve::Line(p2, p3),
                        OutlineCurve::Line(p3, p4),
                        OutlineCurve::Line(p4, p1),
                    ]
                })
                .collect();

            Outline { bounds, curves }
        })
    }

    fn glyph_count(&self) -> usize {
        256
    }

    fn codepoint_ids(&self) -> CodepointIdIter {
        // TODO: lol. lol.
        #[allow(dead_code)]
        struct CodepointIdIter2<'a> {
            inner: Box<dyn Iterator<Item=(GlyphId, char)> + 'a>,
        }
        let iter = DECODE_TABLE
            .iter()
            .copied()
            .enumerate()
            .map(|(cp, c)| (GlyphId(cp as u16), c));
        let iter = CodepointIdIter2 { inner: Box::new(iter) };
        unsafe {
            // safety:
            // this is just outright undefined behavior that I'm relying on.
            std::mem::transmute(iter)
        }
    }

    fn glyph_raster_image(&self, _id: GlyphId, _pixel_size: u16) -> Option<GlyphImage> {
        None
    }
}
