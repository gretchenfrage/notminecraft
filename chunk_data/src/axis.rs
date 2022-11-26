
use std::{
    ops::{
        Index,
        IndexMut,
        Neg,
    },
    convert::TryFrom,
};
use vek::*;


macro_rules! axis_enum {
    (
        $name:ident,
        $num_constant:ident = $num:expr,
        $per_name:ident,
        $all_constant:ident,
        ($(
            $pos:ident = [$pos_x:expr, $pos_y:expr, $pos_z:expr],
            $neg:ident = [$neg_x:expr, $neg_y:expr, $neg_z:expr],
        )*),
    )=>{
        #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
        #[repr(u8)]
        pub enum $name {$(
            $pos,
            $neg,
        )*}

        pub const $num_constant: usize = $num;

        #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub struct $per_name<T>(pub [T; $num_constant]);

        pub const $all_constant: $per_name<$name> = $per_name([$(
            $name::$pos,
            $name::$neg,
        )*]);

        impl $name {
            pub const fn to_vec(self) -> Vec3<i64> {
                match self {$(
                    $name::$pos => Vec3 { x: $pos_x, y: $pos_y, z: $pos_z },
                    $name::$neg => Vec3 { x: $neg_x, y: $neg_y, z: $neg_z },
                )*}
            }

            pub const fn from_vec(vec: Vec3<i64>) -> Option<Self> {
                match vec {
                    $(
                        Vec3 {
                            x: $pos_x, y: $pos_y, z: $pos_z
                        } => Some($name::$pos),
                        Vec3 {
                            x: $neg_x, y: $neg_y, z: $neg_z
                        } => Some($name::$neg),
                    )*
                    _ => None
                }
            }
        }

        impl Into<Vec3<i64>> for $name {
            fn into(self) -> Vec3<i64> {
                self.to_vec()
            }
        }

        impl Into<Vec3<f32>> for $name {
            fn into(self) -> Vec3<f32> {
                self.to_vec().map(|n| n as f32)
            }
        }

        impl TryFrom<Vec3<i64>> for $name {
            type Error = ();

            fn try_from(vec: Vec3<i64>) -> Result<Self, ()> {
                Self::from_vec(vec).ok_or(())
            }
        }

        impl<T> Index<$name> for $per_name<T> {
            type Output = T;

            fn index(&self, i: $name) -> &Self::Output {
                &self.0[i as usize]
            }
        }

        impl<T> IndexMut<$name> for $per_name<T> {
            fn index_mut(&mut self, i: $name) -> &mut Self::Output {
                &mut self.0[i as usize]
            }
        }

        impl Neg for $name {
            type Output = Self;

            fn neg(self) -> Self {
                match self {$(
                    $name::$pos => $name::$neg,
                    $name::$neg => $name::$pos,
                )*}
            }
        }

        impl<T: Clone> $per_name<T> {
            pub fn repeat(val: T) -> Self {
                $per_name([$(
                    #[allow(non_snake_case)]
                    {
                        let $pos = ();
                        let _ = $pos;
                        val.clone()
                    },
                    val.clone(),
                )*])
            }

            pub fn map<B, F>(self, f: F) -> $per_name<B>
            where
                F: FnMut(T) -> B,
            {
                $per_name(self.0.map(f))
            }
        }

        impl<T> IntoIterator for $per_name<T> {
            type Item = T;
            type IntoIter = <[T; $num_constant] as IntoIterator>::IntoIter;

            fn into_iter(self) -> Self::IntoIter {
                self.0.into_iter()
            }
        }
    };
}

macro_rules! fec_system {
    (
        faces = ($(
            $face:ident = [$($face_vec:tt)*],
        )*),
        edges = ($(
            $edge:ident = [$($edge_vec:tt)*],
        )*),
        corners = ($(
            $corner:ident = [$($corner_vec:tt)*],
        )*),
    )=>{
        axis_enum!(
            Face,
            NUM_FACES = 6,
            PerFace,
            FACES,
            ($(
                $face = [$($face_vec)*],
            )*),
        );

        axis_enum!(
            Edge,
            NUM_EDGES = 12,
            PerEdge,
            EDGES,
            ($(
                $edge = [$($edge_vec)*],
            )*),
        );

        axis_enum!(
            Corner,
            NUM_CORNERS = 8,
            PerCorner,
            CORNERS,
            ($(
                $corner = [$($corner_vec)*],
            )*),
        );

        axis_enum!(
            FaceEdgeCorner,
            NUM_FACES_EDGES_CORNERS = 26,
            PerFaceEdgeCorner,
            FACES_EDGES_CORNERS,
            (
                $( $face = [$($face_vec)*], )*
                $( $edge = [$($edge_vec)*], )*
                $( $corner = [$($corner_vec)*], )*
            ),
        );
    };
}

fec_system!(
    faces = (
        PosX = [ 1,  0,  0],
        NegX = [-1,  0,  0],

        PosY = [ 0,  1,  0],
        NegY = [ 0, -1,  0],

        PosZ = [ 0,  0,  1],
        NegZ = [ 0,  0, -1],
    ),
    edges = (
        PosXPosY = [  1,  1,  0],
        NegXNegY = [ -1, -1,  0],

        PosXNegY = [  1, -1,  0],
        NegXPosY = [ -1,  1,  0],

        PosYPosZ = [  0,  1,  1],
        NegYNegZ = [  0, -1, -1],

        PosYNegZ = [  0,  1, -1],
        NegYPosZ = [  0, -1,  1],

        PosXPosZ = [  1,  0,  1],
        NegXNegZ = [ -1,  0, -1],

        PosXNegZ = [  1,  0, -1],
        NegXPosZ = [ -1,  0,  1],      
    ),
    corners = (
        PosXPosYPosZ = [  1,  1,  1],
        NegXNegYNegZ = [ -1, -1, -1],

        NegXPosYPosZ = [ -1,  1,  1],
        PosXNegYNegZ = [  1, -1, -1],

        PosXNegYPosZ = [  1, -1,  1],
        NegXPosYNegZ = [ -1,  1, -1],

        PosXPosYNegZ = [  1,  1, -1],
        NegXNegYPosZ = [ -1, -1,  1],
    ),
);
