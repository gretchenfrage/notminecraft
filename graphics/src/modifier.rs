//! Modifiers that map one canvas onto another canvas, and eventually onto the
//! render target.

use vek::*;


/// Any modifier in 2D space.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Modifier2 {
    /// Apply an affine transform to the geometry.
    Transform(Transform2),
    /// Apply color multiplication.
    Color(Rgba<f32>),
    /// Discard all fragments lying in a half-plane.
    Clip(Clip2),
}

impl Modifier2 {
    pub fn to_3d(&self) -> Modifier3 {
        match *self {
            Modifier2::Transform(t) => Modifier3::Transform(t.to_3d()),
            Modifier2::Color(c) => Modifier3::Color(c),
            Modifier2::Clip(c) => Modifier3::Clip(c.to_3d()),
        }
    }
}

impl From<Transform2> for Modifier2 {
    fn from(inner: Transform2) -> Self {
        Modifier2::Transform(inner)
    }
}

impl From<Rgba<f32>> for Modifier2 {
    fn from(inner: Rgba<f32>) -> Self {
        Modifier2::Color(inner)
    }
}

impl From<Clip2> for Modifier2 {
    fn from(inner: Clip2) -> Self {
        Modifier2::Clip(inner)
    }
}


/// Any modifier in 3D space.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Modifier3 {
    /// Apply an affine transform to the geometry.
    Transform(Transform3),
    /// Apply color multiplication.
    Color(Rgba<f32>),
    /// Discard all fragments lying in a half-plane.
    Clip(Clip3),
}

impl From<Transform3> for Modifier3 {
    fn from(inner: Transform3) -> Self {
        Modifier3::Transform(inner)
    }
}

impl From<Rgba<f32>> for Modifier3 {
    fn from(inner: Rgba<f32>) -> Self {
        Modifier3::Color(inner)
    }
}

impl From<Clip3> for Modifier3 {
    fn from(inner: Clip3) -> Self {
        Modifier3::Clip(inner)
    }
}


/// A 2D affine transform modifier. Is a newtype around a matrix.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Transform2(pub Mat3<f32>);

impl Transform2 {
    /// Identity transform.
    pub fn identity() -> Self {
        Transform2(Mat3::identity())
    }

    /// Translate by `v`.
    pub fn translate<V: Into<Vec2<f32>>>(v: V) -> Self {
        Transform2(Mat3::translation_2d(v))
        /*// Mat3::translation_2d seems to be simply wrong
        let v = v.into();
        Transform2(Mat3::new(
            1.0, 0.0, v.x,
            0.0, 1.0, v.y,
            0.0, 0.0, 1.0,
        ))*/
    }

    /// Component-wise scale by `v`.
    pub fn scale<V: Into<Vec2<f32>>>(v: V) -> Self {
        let v = v.into();
        Transform2(Mat3::scaling_3d([v.x, v.y, 1.0]))
    }

    /// Rotate clockwise by `r` radians.
    pub fn rotate(f: f32) -> Self {
        Transform2(Mat3::rotation_z(-f))
    }

    /// Apply this transformation to a point.
    ///
    /// This is useful for mapping from object space to screen space, eg. for
    /// rendering.
    pub fn apply<V: Into<Vec2<f32>>>(&self, v: V) -> Vec2<f32> {
        (self.0 * Vec3::from_point_2d(v)).xy()
    }

    /// Attempt to apply this transformation to a point in reverse such
    /// that `a.reverse_apply(a.apply(v)) == v`.
    ///
    /// This is useful for mapping from screen space to object space, eg for
    /// button clicks.
    ///
    /// Returns `None` if this transformation is irreversible, which will
    /// occur in some unusual situations, such as scaling by 0.
    ///
    /// This operation is slightly expensive, as it computes the matrix
    /// inverse each time. As such, if this is done frequently, one should take
    /// an approach that saves work.
    pub fn reverse_apply<V: Into<Vec2<f32>>>(&self, v: V) -> Option<Vec2<f32>> {
        if self.0.determinant() != 0.0 {
            let inverted = Mat3::from(Mat4::from(self.0).inverted());
            Some((inverted * Vec3::from_point_2d(v)).xy())
        } else {
            None
        }
    }

    /// Compose with another such that
    /// `b.apply(a.apply(v)) == a.compose(b).apply(v)`.
    pub fn then(&self, other: &Self) -> Self {
        Transform2(other.0 * self.0)
    }

    /// Apply this transformation to a clip, such that
    /// `c.clip(v) == a.apply_clip(c).clip(a.apply(v))`.
    ///
    /// As such, this allows one to convert a "clip, then transform" sequence
    /// into a "transform, then clip" sequence such that it remains logically the same.
    pub fn apply_clip(&self, clip: &Clip2) -> Clip2 {
        Clip2(self.0.transposed() * clip.0)
    }

    // TODO do we want to expose this API?
    pub fn to_3d(&self) -> Transform3 {
        let [
            m00, m01, m02,
            m10, m11, m12,
            m20, m21, m22,
        ] = self.0.into_row_array();
        debug_assert_eq!(m20, 0.0);
        debug_assert_eq!(m21, 0.0);
        debug_assert_eq!(m22, 1.0);
        Transform3(Mat4::new(
            m00, m01, 0.0, m02,
            m10, m11, 0.0, m12,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ))
        //Transform3(dbg!(Mat4::from(dbg!(self.0))))
    }
}

/// A 3D affine transform modifier. Is a newtype around a matrix.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Transform3(pub Mat4<f32>);

impl Transform3 {
    /// Identity transform.
    pub fn identity() -> Self {
        Transform3(Mat4::identity())
    }

    /// Translate by `v`.
    pub fn translate<V: Into<Vec3<f32>>>(v: V) -> Self {
        Transform3(Mat4::translation_3d(v))
    }

    /// Component-wise scale by `v`.
    pub fn scale<V: Into<Vec3<f32>>>(v: V) -> Self {
        Transform3(Mat4::scaling_3d(v))
    }

    /// Rotate by `q`.
    pub fn rotate<Q: Into<Quaternion<f32>>>(q: Q) -> Self {
        Transform3(Mat4::from(q.into()))
    }

    /// Apply this transformation to a point.
    ///
    /// This is useful for mapping from object space to screen space.
    pub fn apply<V: Into<Vec3<f32>>>(&self, v: V) -> Vec3<f32> {
        (self.0 * Vec4::from_point(v)).xyz()
    }

    /// Attempt to apply this transformation to a point in reverse such
    /// that `a.reverse_apply(a.apply(v)) == v`.
    ///
    /// This is useful for mapping from screen space to object space, eg for
    /// button clicks.
    ///
    /// Returns `None` if this transformation is irreversible, which will
    /// occur in some unusual situations, such as scaling by 0.
    ///
    /// This operation is slightly expensive, as it computes the matrix
    /// inverse each time. As such, if this is done frequently, one should take
    /// an approach that saves work.
    pub fn reverse_apply<V: Into<Vec3<f32>>>(&self, v: V) -> Option<Vec3<f32>> {
        if self.0.determinant() != 0.0 {
            Some((self.0.inverted() * Vec4::from_point(v)).xyz())
        } else {
            None
        }
    }

    /// Compose with another such that
    /// `b.apply(a.apply(v)) == a.compose(b).apply(v)`.
    pub fn then(&self, other: &Self) -> Self { // TODO none of these should be pass by ref
        Transform3(other.0 * self.0)
    }

    /// Apply this transformation to a clip, such that
    /// `c.clip(v) == a.apply_clip(c).clip(a.apply(v))`.
    ///
    /// As such, this allows one to convert a "clip, then transform" sequence
    /// into a "transform, then clip" sequence such that it remains logically the same.
    pub fn apply_clip(&self, clip: &Clip3) -> Clip3 {
        // TODO quite temporary really
        //*clip
        //Clip3(dbg!(dbg!(dbg!(self.0).transposed()) * dbg!(clip.0)))

        // c dot v    = c' dot (M * v)
        // c^T * v    = c'^T * M * v
        // c^T        = c'^T * M
        // c^T * M^-1 = c'^T * M * M^-1
        // c^T * M^-1 = c'^T
        // (c^T * M^-1)^T = c'

        // TODO expensiveness?
        if self.0.determinant() != 0.0 {
            Clip3(clip.0 * self.0.inverted())
        } else {
            // unimplemented!() // TODO implement better handling for this edge case
            Clip3(Vec4::new(1.0, 1.0, 1.0, 1.0))
        }
    }
}

#[test]
fn test_foo() { // TODO
    let a = Transform3::translate([1.0, 0.0, 0.0])
        .then(&Transform3::scale([1.0, 0.5, 1.0]));
    let c = Clip3::min_x(0.5);
    let v = Vec3::new(1.0, 2.0, 0.0);
    assert_eq!(
        c.dot(v),
        a.apply_clip(&c).dot(a.apply(v)),
    );
}


/// A 2D clip modifier.
///
/// Is a newtype around a vector, <a,b,c>. Represents an instruction to discard
/// any fragment <x,y> for which (<x,y,1> dot <a,b,c>) < 0. As such, can be
/// visualized as a line through the plane which divides it into a "keep" half
/// and a "discard" half.
///
/// See `Transform2::apply_clip`.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Clip2(pub Vec3<f32>);

impl Clip2 {
    /// Discard x < f.
    pub fn min_x(f: f32) -> Self {
        // x > f === ax + by + c > 0
        //           a = 1
        //           b = 0
        // x > f === x + c > 0
        // x > f === x > -c
        //           c = -f
        Clip2([1.0, 0.0, -f].into())
    }

    /// Discard x > f.
    pub fn max_x(f: f32) -> Self {
        // x < f === ax + by + c > 0
        //           b = 0
        // x < f === ax + c > 0
        // x < f === ax > -c
        //           a = -1
        // x < f === -x > -c
        // x < f === x < c
        //           c = f
        Clip2([-1.0, 0.0, f].into())
    }

    /// Discard y < f.
    pub fn min_y(f: f32) -> Self {
        Clip2([0.0, 1.0, -f].into())
    }

    /// Discard y > f.
    pub fn max_y(f: f32) -> Self {
        Clip2([0.0, -1.0, f].into())
    }

    /// Whether this clip would allow the given point to remain (as opposed to
    /// being clipped out).
    pub fn test(&self, v: Vec2<f32>) -> bool {
        self.0.dot(Vec3::from_point_2d(v)) >= 0.0
    }

    // TODO do we want to expose this API?
    pub fn to_3d(&self) -> Clip3 {
        Clip3([self.0.x, self.0.y, /* TODO extremely temporary pseudofix 0.0*/ 0.00, self.0.z].into())
    }
}


/// A 3D clip modifier.
///
/// Is a newtype around a vector, <a,b,c,d>. Represents an instruction to discard
/// any fragment <x,y,z> for which (<x,y,z,1> dot <a,b,c,d>) < 0. As such, can be
/// visualized as a plane through the volume which divides it into a "keep" half
/// and a "discard" half.
///
/// See `Transform3::apply_clip`.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Clip3(pub Vec4<f32>);

impl Clip3 {
    /// Discard x < f.
    pub fn min_x(f: f32) -> Self {
        Clip3([1.0, 0.0, 0.0, -f].into())
    }

    /// Discard x > f.
    pub fn max_x(f: f32) -> Self {
        Clip3([-1.0, 0.0, 0.0, f].into())
    }

    /// Discard y < f.
    pub fn min_y(f: f32) -> Self {
        Clip3([0.0, 1.0, 0.0, -f].into())
    }

    /// Discard y > f.
    pub fn max_y(f: f32) -> Self {
        Clip3([0.0, -1.0, 0.0, f].into())
    }

    /// Discard z < f.
    pub fn min_z(f: f32) -> Self {
        Clip3([0.0, 0.0, 1.0, -f].into())
    }

    /// Discard x > f.
    pub fn max_z(f: f32) -> Self {
        Clip3([0.0, 0.0, -1.0, f].into())
    }

    /// Whether this clip would allow the given point to remain (as opposed to
    /// being clipped out).
    pub fn test(&self, v: Vec3<f32>) -> bool {
        self.0.dot(Vec4::from_point(v)) >= 0.0
    }

    // TODO temporary or something
    fn dot(&self, v: Vec3<f32>) -> f32 {
        self.0.dot(Vec4::from_point(v))
    }
}
