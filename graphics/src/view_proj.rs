
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

impl From<Mat4<f32>> for ViewProj {
    fn from(mat: Mat4<f32>) -> Self {
        ViewProj(mat)
    }
}

impl ViewProj {
    /// Perspective camera at position `pos`, pointing in direction `dir`
    /// (rotated from "forward and pointing +z"), with the given field-of-view
    /// and aspect ratio.
    #[allow(unused_variables)]
    pub fn perspective(
        pos: impl Into<Vec3<f32>>,
        dir: impl Into<Quaternion<f32>>,
        fov: f32,
        aspect: f32,
    ) -> Self {
        let view1 = Mat4::<f32>::translation_3d(-pos.into());
        let view2 = Mat4::<f32>::from(-dir.into());
        let proj = Mat4::<f32>::infinite_perspective_lh(
            fov,
            aspect,
            0.01,
            //1.0, // TODO
        );
        /*
        proj.cols.x.w = 0.0;
        proj.cols.y.w = 0.0; // TODO wtf?
        proj.cols.z.w = 0.0;
        proj.cols.w.w = 1.0;*/
        let adjust1 = Mat4::<f32>::scaling_3d([1.0, 1.0, 1.0 / 1000.0]);
        let adjust2 = Mat4::<f32>::scaling_3d([1.0, -1.0, 1.0]);
        let adjust3 = Mat4::<f32>::translation_3d([0.5, 0.5, 0.0]);
        ViewProj(adjust3 * adjust2 * adjust1 * proj * view2 * view1)
        //ViewProj(proj)
    }

    // TODO orthographics
}
