//! Barrier rect utility.

use super::super::aa_box::AaBox;
use chunk_data::{
    Face,
    Pole,
};


/// Face of an AA box of world geometry.
///
/// Does not itself contain information about which face is facing (that is,
/// which axis it is normal to and which direction along that axis the barrier
/// faces).
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