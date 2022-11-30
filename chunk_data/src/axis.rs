
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
            $variant:ident,
        )*),
    )=>{
        #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
        #[repr(u8)]
        pub enum $name {$(
            $variant,
        )*}

        pub const $num_constant: usize = $num;

        #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub struct $per_name<T>(pub [T; $num_constant]);

        pub const $all_constant: $per_name<$name> = $per_name([$(
            $name::$variant,
        )*]);

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

        impl<T: Clone> $per_name<T> {
            pub fn repeat(val: T) -> Self {
                $per_name([$(
                    #[allow(non_snake_case)]
                    {
                        let $variant = ();
                        let _ = $variant;
                        val.clone()
                    },
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

axis_enum!(
    Axis,
    NUM_AXES = 3,
    PerAxis,
    AXES,
    (
        X,
        Y,
        Z,
    ),
);

axis_enum!(
    Pole,
    NUM_POLES = 2,
    PerPole,
    POLES,
    (
        Neg,
        Pos,
    ),
);

impl Pole {
    pub const fn to_int(self) -> i64 {
        match self {
            Pole::Neg => -1,
            Pole::Pos => 1,
        }
    }

    pub const fn from_int(int: i64) -> Option<Self> {
        match int {
            -1 => Some(Pole::Neg),
            1 => Some(Pole::Pos),
            _ => None,
        }
    }
}

impl Into<i64> for Pole {
    fn into(self) -> i64 {
        self.to_int()
    }
}

impl Into<f32> for Pole {
    fn into(self) -> f32 {
        self.to_int() as f32
    }
}

impl TryFrom<i64> for Pole {
    type Error = ();

    fn try_from(int: i64) -> Result<Self, ()> {
        Self::from_int(int).ok_or(())
    }
}

impl Neg for Pole {
    type Output = Self;

    fn neg(self) -> Self {
        match self {
            Pole::Pos => Pole::Neg,
            Pole::Neg => Pole::Pos,
        }
    }
}

axis_enum!(
    Sign,
    NUM_SIGNS = 3,
    PerSign,
    SIGNS,
    (
        Neg,
        Zero,
        Pos,
    ),
);

impl Sign {
    pub const fn to_int(self) -> i64 {
        match self {
            Sign::Neg => -1,
            Sign::Zero => 0,
            Sign::Pos => 1,
        }
    }

    pub const fn of_int(n: i64) -> Self {
        if n > 0 {
            Sign::Pos
        } else if n < 0 {
            Sign::Neg
        } else {
            Sign::Zero
        }
    }

    pub const fn of_float(n: f32) -> Self {
        if n > 0.0 {
            Sign::Pos
        } else if n < 0.0 {
            Sign::Neg
        } else {
            Sign::Zero
        }
    }

    pub const fn from_int(n: i64) -> Option<Self> {
        match n {
            -1 => Some(Sign::Neg),
            0 => Some(Sign::Zero),
            1 => Some(Sign::Pos),
            _ => None,
        }
    }

    pub const fn neg(self) -> Self {
        // TODO: this only exists because const traits aren't stable yet
        match self {
            Sign::Neg => Sign::Pos,
            Sign::Zero => Sign::Zero,
            Sign::Pos => Sign::Neg,
        }
    }
}

impl Into<i64> for Sign {
    fn into(self) -> i64 {
        self.to_int()
    }
}

impl Into<f32> for Sign {
    fn into(self) -> f32 {
        self.to_int() as f32
    }
}

impl Neg for Sign {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Sign::neg(self)
    }
}

macro_rules! veclike_axis_enum {
    (
        $name:ident,
        $num_constant:ident = $num:expr,
        $per_name:ident,
        $all_constant:ident,
        ($(
            $pos:ident
                = [$pos_x:ident, $pos_y:ident, $pos_z:ident]
                = -$neg:ident,
        )*),
    )=>{
        axis_enum!(
            $name,
            $num_constant = $num,
            $per_name,
            $all_constant,
            ($(
                $pos,
                $neg,
            )*),
        );

        impl $name {
            pub const fn to_signs(self) -> Vec3<Sign> {
                match self {$(
                    $name::$pos => Vec3 {
                        x: Sign::$pos_x,
                        y: Sign::$pos_y,
                        z: Sign::$pos_z,
                    },
                    $name::$neg => Vec3 {
                        x: -Sign::$pos_x,
                        y: -Sign::$pos_y,
                        z: -Sign::$pos_z,
                    },
                )*}
            }

            pub const fn to_vec(self) -> Vec3<i64> {
                self.to_signs().map(|n| n.to_int())
            }

            pub const fn from_signs(signs: Vec3<Sign>) -> Option<Self> {
                match vec {
                    $(
                        Vec3 {
                            x: Sign::$pos_x,
                            y: Sign::$pos_y,
                            z: Sign::$pos_z,
                        } => Some($name::$pos),
                        Vec3 {
                            x: Sign::$pos_x.neg(),
                            y: Sign::$pos_y.neg(),
                            z: Sign::$pos_z.neg(),
                        } => Some($name::$neg),
                    )*
                    _ => None
                }
            }

            pub const fn from_vec(vec: Vec3<i64>) -> Option<Self> {
                if let 
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

        impl Neg for $name {
            type Output = Self;

            fn neg(self) -> Self {
                match self {$(
                    $name::$pos => $name::$neg,
                    $name::$neg => $name::$pos,
                )*}
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
        veclike_axis_enum!(
            Face,
            NUM_FACES = 6,
            PerFace,
            FACES,
            ($(
                $face = [$($face_vec)*],
            )*),
        );

        veclike_axis_enum!(
            Edge,
            NUM_EDGES = 12,
            PerEdge,
            EDGES,
            ($(
                $edge = [$($edge_vec)*],
            )*),
        );

        veclike_axis_enum!(
            Corner,
            NUM_CORNERS = 8,
            PerCorner,
            CORNERS,
            ($(
                $corner = [$($corner_vec)*],
            )*),
        );

        veclike_axis_enum!(
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
