//! Barrier rect utility.

use super::super::aa_box::AaBox;
use chunk_data::{
    Face,
    Pole,
};


/// Face of an AA box of world geometry.
///
/// Takes advantage of `Axis.other_axes`. For a given `Axis` (X, Y, or Z), `Axis.other_axes`
/// defines for each of the indexes `0` and `1` one of the two "other" axes. We can use this to
/// write code that loops over the X, Y, and Z axes in a generic way: in the inner loop, we phrase
/// our code in terms of "the axis" of the current loop iteration, and the "other axes" 0 and 1.
///
/// As such, `AaBoxFace`, being a utility meant to be used within these inner loops, becomes
/// meaningful within the context of a some axis, but does not itself store that axis.
#[derive(Debug, Copy, Clone)]
pub struct AaBoxFace {
    pub axis_pos: f32,
    pub other_axes_pos: [f32; 2],
    pub other_axes_ext: [f32; 2],
}

impl AaBoxFace {
    pub fn new(aa_box: AaBox, face: Face) -> Self {
        let (axis, pole) = face.to_axis_pole();
        let other_axes = axis.other_axes();

        let axis_pos =
            aa_box.pos[axis as usize]
            + match pole {
                Pole::Neg => 0.0,
                Pole::Pos => aa_box.ext[axis as usize],
            };
        let mut other_axes_pos = [0.0; 2];
        let mut other_axes_ext = [0.0; 2];
        for i in 0..2 {
            other_axes_pos[i] = aa_box.pos[other_axes[i] as usize];
            other_axes_ext[i] = aa_box.ext[other_axes[i] as usize];
        }

        AaBoxFace {
            axis_pos,
            other_axes_pos,
            other_axes_ext,
        }
    }
}
