
use vek::*;


/// Axis-aligned box.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct AaBox {
    pub pos: Vec3<f32>,
    /// Extent is assumed to be non-negative.
    pub ext: Extent3<f32>,
}

impl AaBox {
    pub fn translate<V: Into<Vec3<f32>>>(mut self, v: V) -> Self {
        self.pos += v.into();
        self
    }
}
