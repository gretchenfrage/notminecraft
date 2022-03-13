//! Serialization of data into the std140 layout format, suitable for use in
//! uniform buffers, as described in:
//!
//! https://www.oreilly.com/library/view/opengl-programming-guide/9780132748445/app09lev1sec2.html

use vek::*;


/// Type which can serialize into the std140 layout format, suitable for use
/// in uniform buffers. Furthermore, these types have a _statically_ known size
/// and alignment.
pub trait Std140: Sized + Clone {
    /// Required alignment of this type in the uniform buffer.
    const ALIGN: usize;

    /// Amount of size this type takes in the uniform buffer. A multiple of
    /// `Self::ALIGN`.
    const SIZE: usize;

    /// Serialize self into uniform buffer data. Writes exactly `Self::SIZE`
    /// bytes to `dst`. Assumes `dst.len()` is a multiple of `Self::ALIGN`.
    fn write(&self, dst: &mut Vec<u8>);

    /// Write the necessary number of padding bytes to `dst` so as to ensure
    /// necessary alignment, then write self to `dst`. Returns the index in
    /// `dst` where started writing self's actual data (not self's padding
    /// bytes).
    fn pad_write(&self, dst: &mut Vec<u8>) -> usize {
        while dst.len() % Self::ALIGN != 0 {
            dst.push(0);
        }
        let offset = dst.len();
        self.write(dst);
        offset
    }
}

/// Marker trait for `Std140` types which are considered "scalars".
pub trait Std140Scalar {}

/// Marker trait for `Std140` types which are considered either "scalars" or
/// "vectors".
pub trait Std140ScalarOrVector {}

// scalars

impl Std140 for bool {
    // the oreilly page was ambiguous on this, but this SO answer:
    //
    // https://stackoverflow.com/a/9419959/4957011
    //
    // clarifies that bools use 4 bytes, not 1
    const ALIGN: usize = 4;
    const SIZE: usize = 4;

    fn write(&self, dst: &mut Vec<u8>) {
        if *self {
            dst.push(1);
        } else {
            dst.push(0);
        }
    }
}

impl Std140Scalar for bool {}

impl Std140ScalarOrVector for bool {}


impl Std140 for i32 {
    const ALIGN: usize = 4;
    const SIZE: usize = 4;

    fn write(&self, dst: &mut Vec<u8>) {
        dst.extend(self.to_le_bytes());
    }
}

impl Std140Scalar for i32 {}

impl Std140ScalarOrVector for i32 {}


impl Std140 for u32 {
    const ALIGN: usize = 4;
    const SIZE: usize = 4;

    fn write(&self, dst: &mut Vec<u8>) {
        dst.extend(self.to_le_bytes());
    }
}

impl Std140Scalar for u32 {}

impl Std140ScalarOrVector for u32 {}


impl Std140 for f32 {
    const ALIGN: usize = 4;
    const SIZE: usize = 4;

    fn write(&self, dst: &mut Vec<u8>) {
        dst.extend(self.to_le_bytes());
    }
}

impl Std140Scalar for f32 {}

impl Std140ScalarOrVector for f32 {}


impl Std140 for f64 {
    const ALIGN: usize = 8;
    const SIZE: usize = 8;

    fn write(&self, dst: &mut Vec<u8>) {
        dst.extend(self.to_le_bytes());
    }
}

impl Std140ScalarOrVector for f64 {}

impl Std140Scalar for f64 {}


// two-component vectors

macro_rules! std140_vec_2 {
    ($type:ident, $x:ident, $y:ident)=>{
        impl<T: Std140 + Std140Scalar> Std140 for $type<T> {
            // "both the size and aligment are twice the size of the underlying scalar
            // type."
            const ALIGN: usize = T::SIZE * 2;
            const SIZE: usize = T::SIZE * 2;

            fn write(&self, dst: &mut Vec<u8>) {
                self.$x.write(dst);
                self.$y.write(dst);
            }
        }

        impl<T: Std140 + Std140Scalar> Std140ScalarOrVector for $type<T> {}
    };
}

std140_vec_2!(Vec2, x, y);
std140_vec_2!(Extent2, w, h);



// three and four-component vectors
//
// "both the size and alignment are four times the size of the underlying
// scalar type."

macro_rules! std140_vec_3 {
    ($type:ident, $x:ident, $y:ident, $z:ident)=>{
        impl<T: Std140 + Std140Scalar> Std140 for $type<T> {
            const ALIGN: usize = T::SIZE * 4;
            const SIZE: usize = T::SIZE * 4;

            fn write(&self, dst: &mut Vec<u8>) {
                self.$x.write(dst);
                self.$y.write(dst);
                self.$z.write(dst);
                for _ in 0..T::SIZE {
                    dst.push(0);
                }
            }
        }

        impl<T: Std140 + Std140Scalar> Std140ScalarOrVector for $type<T> {}
    };
}

std140_vec_3!(Vec3, x, y, z);
std140_vec_3!(Rgb, r, g, b);
std140_vec_3!(Extent3, w, h, d);


macro_rules! std140_vec_4 {
    ($type:ident, $x:ident, $y:ident, $z:ident, $w:ident)=>{
        impl<T: Std140 + Std140Scalar> Std140 for $type<T> {
            const ALIGN: usize = T::SIZE * 4;
            const SIZE: usize = T::SIZE * 4;

            fn write(&self, dst: &mut Vec<u8>) {
                self.$x.write(dst);
                self.$y.write(dst);
                self.$z.write(dst);
                self.$w.write(dst);
            }
        }

        impl<T: Std140 + Std140Scalar> Std140ScalarOrVector for $type<T> {}    
    };
}

std140_vec_4!(Vec4, x, y, z, w);
std140_vec_4!(Rgba, r, g, b, a);


// arrays of scalars or vectors
//
// note: I'm not sure if this correctly handles arrays of matrices or
//       structures, but it should correctly handle arrays of scalars or
//       vectors

const fn arr_elem_size(elem_type_size: usize) -> usize {
    // "the size of each element in the array will be the size of the element
    // type, rounded up to a multiple of the size of a vec4"
    if elem_type_size % 16 == 0 {
        elem_type_size
    } else {
        elem_type_size - (elem_type_size % 16) + 16
    }
}

impl<T: Std140 + Std140ScalarOrVector, const LEN: usize> Std140 for [T; LEN] {
    // "this is also the array's alignment."
    const ALIGN: usize = arr_elem_size(T::SIZE);

    // "the array's size will be this rounded-up element's size times the
    // number of elements in the array"
    const SIZE: usize = arr_elem_size(T::SIZE) * LEN;

    fn write(&self, dst: &mut Vec<u8>) {
        for elem in self {
            elem.write(dst);
            for _ in 0..arr_elem_size(T::SIZE) - T::SIZE {
                dst.push(0);
            }
        }
    }
}

// matrices
//
// we will only handle column-major matrices, and not handle arrays of matrices
//
// "same layout as an array of N vectors each with R [number of rows]
// components, where N is the total number of columns present [the odd
// phrasing is because it's trying to describe the possibility of arrays of
// matrices, which we won't actually handle]"

impl<T: Std140 + Std140Scalar> Std140 for Mat3<T> {
    const ALIGN: usize = <[Vec3<T>; 3]>::ALIGN;
    const SIZE: usize = <[Vec3<T>; 3]>::ALIGN;

    fn write(&self, dst: &mut Vec<u8>) {
        let as_array: [Vec3<T>; 3] = self.cols.clone().into_array();
        as_array.write(dst);
    }
}

impl<T: Std140 + Std140Scalar> Std140 for Mat4<T> {
    const ALIGN: usize = <[Vec4<T>; 4]>::ALIGN;
    const SIZE: usize = <[Vec4<T>; 4]>::ALIGN;

    fn write(&self, dst: &mut Vec<u8>) {
        let as_array: [Vec4<T>; 4] = self.cols.clone().into_array();
        as_array.write(dst);
    }
}

// structures
//
// we will not handle arrays of structures

/// Macro for implementing `Std140` on a struct of types which implement
/// `Std140`.
macro_rules! std140_struct {
    ($struct:ident {$(
        $field:ident: $type:ty
    ),*$(,)?})=>{
        impl $crate::std140::Std140 for $struct {
            // "structure alignment will be the alignment for the biggest
            // structure member, according to the previous rules, rounded up
            // to a multiple of the size of a vec4"
            const ALIGN: usize = {
                let mut align = 0;
                $(
                let field_align = <$type as $crate::std140::Std140>::ALIGN;
                if field_align > align {
                    align = field_align;
                }
                )*
                let rem = align % 16;
                if rem == 0 {
                    align
                } else {
                    align - rem + 16
                }
            };

            // "its size will be the space needed by its members, according to
            // the previous rules, rounded up to a multiple of the structure
            // alignment"
            const SIZE: usize = {
                let mut size = 0;
                $(
                let field_align = <$type as $crate::std140::Std140>::ALIGN;
                let field_size = <$type as $crate::std140::Std140>::SIZE;
                if size % field_align != 0 {
                    size += field_align - (size % field_align);
                }
                size += field_size;
                )*
                if size % Self::ALIGN != 0 {
                    size += Self::ALIGN - (size % Self::ALIGN);
                }
                //size
                256 // TODO LOL
            };

            fn write(&self, dst: &mut Vec<u8>) {
                let len_before = dst.len();
                $(
                <$type as $crate::std140::Std140>::pad_write(&self.$field, dst);
                )*
                while dst.len() < len_before + Self::SIZE {
                    dst.push(0);
                }
            }
        }
    };
}

pub(crate) use std140_struct;
