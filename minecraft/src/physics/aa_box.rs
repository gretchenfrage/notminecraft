
use vek::*;


/// Axis-aligned box.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct AaBox {
    pub pos: Vec3<f32>,
    /// Extent is assumed to be non-negative.
    pub ext: Extent3<f32>,
}

impl AaBox {
    /// Box from <0,0,0> to <1,1,1>.
    pub const UNIT_BOX: AaBox = AaBox {
        pos: Vec3 { x: 0.0, y: 0.0, z: 0.0 },
        ext: Extent3 { w: 1.0, h: 1.0, d: 1.0 },
    };

    pub fn translate<V: Into<Vec3<f32>>>(mut self, v: V) -> Self {
        self.pos += v.into();
        self
    }

    pub fn expand(mut self, amount: f32) -> Self {
        self.pos -= Vec3::from(amount);
        self.ext += Extent3::from(amount * 2.0);
        self
    }

    pub fn contains<V: Into<Vec3<f32>>>(self, pos: V) -> bool {
        let pos = pos.into();
        let max = self.pos + self.ext;
        pos.x > self.pos.x
            && pos.y > self.pos.y
            && pos.z > self.pos.z
            && pos.x < max.x
            && pos.y < max.y
            && pos.z < max.z
    }

    pub fn intersects(self, rhs: AaBox) -> bool {
        for i in 0..3 {
            if self.pos[i] >= rhs.pos[i] + rhs.ext[i] {
                return false;
            }
            if self.pos[i] + self.ext[i] <= rhs.pos[i] {
                return false;
            }
        }
        true
    }
}
