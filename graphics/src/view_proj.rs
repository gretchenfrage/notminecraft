
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

    pub fn extract_frustum_planes(&self) -> [Vec4<f32>; 6] {
        let rows = self.0.into_row_arrays();
        let mut planes = [Vec4::zero(); 6];

        for i in 0..6 {
            let row = if i % 2 == 0 {
                i / 2
            } else {
                3 - (i / 2)
            };

            planes[i] = Vec4::from(rows[3]) - Vec4::from(rows[row]);
        }

        // Normalize the planes
        for plane in planes.iter_mut() {
            let length = f32::sqrt(plane.x * plane.x + plane.y * plane.y + plane.z * plane.z);
            *plane /= length;
        }

        planes
    }

    pub fn is_volume_visible(&self, min: Vec3<f32>, ext: Extent3<f32>) -> bool {
        let max = min + ext;
        let planes = self.extract_frustum_planes();
        
        for plane in planes.iter() {
            let p_vertex = Vec4::new(
                if plane.x >= 0.0 { max.x } else { min.x },
                if plane.y >= 0.0 { max.y } else { min.y },
                if plane.z >= 0.0 { max.z } else { min.z },
                1.0,
            );

            if plane.dot(p_vertex) < 0.0 {
                return false;
            }
        }
        true
    }
}
