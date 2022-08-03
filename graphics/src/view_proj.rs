
use vek::*;


/// View + projection matrix, essentially representing a camera orientation and
/// configuration.
///
/// Maps from <x,y,z> 3D space, in which +x is "right", +y is "up", and +z is 
/// forward", to <x,y> 2D space, in which +x is "right", +y is "down", <0,0>
/// is the top-left corner of the "screen", <1,1> is the bottom-right corner of
/// the "screen", and +z is "deeper" for the purpose of depth comparisons. 
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ViewProj(pub Mat4<f32>);

impl ViewProj {
    /// Perspective camera at position `pos`, pointing in direction `dir`
    /// (rotated from "forward and pointing +z"), with the given field-of-view
    /// and aspect ratio.
    #[allow(unused_variables)]
    pub fn perspective(
        pos: Vec3<f32>,
        dir: Quaternion<f32>,
        fov: f32,
        aspect: f32,
    ) -> Self {
        let view = Mat4::<f32>::perspective_lh_zo(
            fov,
            aspect,
            0.1,
            100.0, // TODO
        );
        let adjust = Mat4::<f32>::scaling_3d([1.0, -1.0, 1.0]);
        ViewProj(adjust * view)
    }

    // TODO orthographics
}
