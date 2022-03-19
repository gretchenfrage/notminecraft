//! Serialization of data into vertex buffers and declaration their structure.

use std::mem::size_of;
use vek::*;
use wgpu::{
    VertexFormat,
    VertexAttribute,
};


pub mod glsl_types {
    macro_rules! glsl_types {
        ($($name:ident),*$(,)?)=>{
            $(
            #[allow(non_camel_case_types)]
            pub enum $name {}
            )*
        };
    }

    glsl_types! {
        vec2,
        vec3,
        vec4,

        uvec2,
        uvec3,
        uvec4,
        
        ivec2,
        ivec3,
        ivec4,

        dvec2,
        dvec3,
        dvec4,

        float,
        uint,
        int,
        double,
    }
}

pub trait AttributeData<GlslType> {
    const FORMAT: VertexFormat;
    const SIZE: usize;

    fn write(&self, dst: &mut Vec<u8>);
}

// skipping 16 bit floats

// scalars
macro_rules! attr_scalar {
    ($glsl_type:ident, $rust_type:ty, $format:ident)=>{
        impl AttributeData<glsl_types::$glsl_type> for $rust_type {
            const FORMAT: VertexFormat = VertexFormat::$format;
            const SIZE: usize = size_of::<$rust_type>();

            fn write(&self, dst: &mut Vec<u8>) {
                dst.extend(self.to_le_bytes());
            }
        }
    };
}
attr_scalar!(float, f32, Float32);
attr_scalar!(uint, u32, Uint32);
attr_scalar!(int, i32, Sint32);
attr_scalar!(double, f64, Float64);

// vectors
macro_rules! attr_vec {
    (
        $glsl_type:ident,
        ($($vek_type:ident),*$(,)?),
        $comp_type:ty,
        $format:ident,
        $num_comps:expr $(,)?
    )=>{
        $(
        impl AttributeData<glsl_types::$glsl_type> for $vek_type<$comp_type> {
            const FORMAT: VertexFormat = VertexFormat::$format;
            const SIZE: usize = size_of::<$comp_type>() * $num_comps;

            fn write(&self, dst: &mut Vec<u8>) {
                for comp in self.into_array() {
                    dst.extend(comp.to_le_bytes());
                }
            }
        }
        )*
    };
}

// 2-vectors of (8|16|32)-bit (signed|unsigned) ints
attr_vec!(uvec2, (Vec2, Extent2), u8, Uint8x2, 2);
attr_vec!(ivec2, (Vec2, Extent2), i8, Sint8x2, 2);
attr_vec!(uvec2, (Vec2, Extent2), u16, Uint16x2, 2);
attr_vec!(ivec2, (Vec2, Extent2), i16, Sint16x2, 2);
attr_vec!(uvec2, (Vec2, Extent2), u32, Uint32x2, 2);
attr_vec!(ivec2, (Vec2, Extent2), i32, Sint32x2, 2);

// 2-vectors of normalized (8|16)-bit (signed|unsigned) ints
attr_vec!(vec2, (Vec2, Extent2), u8, Unorm8x2, 2);
attr_vec!(vec2, (Vec2, Extent2), i8, Snorm8x2, 2);
attr_vec!(vec2, (Vec2, Extent2), u16, Unorm16x2, 2);
attr_vec!(vec2, (Vec2, Extent2), i16, Snorm16x2, 2);

// 2-vectors of (32|64)-bit floats
attr_vec!(vec2, (Vec2, Extent2), f32, Float32x2, 2);
attr_vec!(dvec2, (Vec2, Extent2), f64, Float64x2, 2);

// 3-vectors of 32-bit (signed|unsigned) ints
attr_vec!(uvec3, (Vec3, Extent3, Rgb), u32, Uint32x3, 3);
attr_vec!(ivec3, (Vec3, Extent3, Rgb), i32, Sint32x3, 3);

// 3-vectors of (32|64)-bit floats
attr_vec!(vec3, (Vec3, Extent3, Rgb), f32, Float32x3, 3);
attr_vec!(dvec3, (Vec3, Extent3, Rgb), f64, Float64x3, 3);

// 4-vectors of (8|16|32)-bit (signed|unsigned) ints
attr_vec!(uvec4, (Vec4, Rgba), u8, Uint8x4, 4);
attr_vec!(ivec4, (Vec4, Rgba), i8, Sint8x4, 4);
attr_vec!(uvec4, (Vec4, Rgba), u16, Uint16x4, 4);
attr_vec!(ivec4, (Vec4, Rgba), i16, Sint16x4, 4);
attr_vec!(uvec4, (Vec4, Rgba), u32, Uint32x4, 4);
attr_vec!(ivec4, (Vec4, Rgba), i32, Sint32x4, 4);

// 4-vectors of normalized (8|16)-bit (signed|unsigned) ints 
attr_vec!(vec4, (Vec4, Rgba), u8, Unorm8x4, 4);
attr_vec!(vec4, (Vec4, Rgba), i8, Snorm8x4, 4);
attr_vec!(vec4, (Vec4, Rgba), u16, Unorm16x4, 4);
attr_vec!(vec4, (Vec4, Rgba), i16, Snorm16x4, 4);

// 4-vectors of (32|64)-bit floats
attr_vec!(vec4, (Vec4, Rgba), f32, Float32x4, 4);
attr_vec!(dvec4, (Vec4, Rgba), f64, Float64x4, 4);


pub trait VertexStruct {
    const SIZE: usize;
    const ATTRIBUTES: &'static [VertexAttribute];

    fn write(&self, dst: &mut Vec<u8>);
}

macro_rules! vertex_struct {
    ($name:ident {$(
        ($field:ident: $rust_type:ty) (layout(location=$location:expr) in $glsl_type:ident)
    ),*$(,)?})=>{
        impl $crate::vertex::VertexStruct for $name {
            const SIZE: usize =
                0
                $( + <$rust_type as $crate::vertex::AttributeData<$crate::vertex::glsl_types::$glsl_type>>::SIZE)*;
            #[allow(unused_assignments)]
            const ATTRIBUTES: &'static [wgpu::VertexAttribute] = &{
                let mut attrs = [$(
                    wgpu::VertexAttribute {
                        format: <$rust_type as $crate::vertex::AttributeData<$crate::vertex::glsl_types::$glsl_type>>::FORMAT,
                        offset: !0,
                        shader_location: $location,
                    },
                )*];
                let mut offset = 0;
                let mut index = 0;
                $(
                attrs[index].offset = offset as u64;
                offset += <$rust_type as $crate::vertex::AttributeData<$crate::vertex::glsl_types::$glsl_type>>::SIZE;
                index += 1;
                )*
                attrs
            };

            fn write(&self, dst: &mut Vec<u8>) {
                $(
                <$rust_type as $crate::vertex::AttributeData<$crate::vertex::glsl_types::$glsl_type>>::write(
                    &self.$field,
                    dst,
                );
                )*
            }
        }
    };
}

pub(crate) use vertex_struct;
