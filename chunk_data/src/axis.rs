
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

macro_rules! scalarlike_axis_enum {
    (
        $name:ident,
        $num_constant:ident = $num:expr,
        $per_name:ident,
        $all_constant:ident,
        ($(
            $pos:ident
                = ($int:expr)
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
            )*),
        );

        impl $name {
            pub const fn to_int(self) -> i64 {
                match self {$(
                    $name::$pos => $int,
                )*}
            }

            pub const fn from_int(int: i64) -> Option<Self> {
                match int {
                    $(
                        $int => Some($name::$pos),
                    )*
                    _ => None
                }
            }

            pub const fn neg(self) -> Self {
                // TODO: const traits when?
                match self {$(
                    $name::$pos => $name::$neg,
                )*}
            }
        }

        impl Into<i64> for $name {
            fn into(self) -> i64 {
                self.to_int()
            }
        }

        impl Into<f32> for $name {
            fn into(self) -> f32 {
                self.to_int() as f32
            }
        }

        impl TryFrom<i64> for $name {
            type Error = ();

            fn try_from(int: i64) -> Result<Self, ()> {
                Self::from_int(int).ok_or(())
            }
        }

        impl Neg for $name {
            type Output = Self;

            fn neg(self) -> Self {
                $name::neg(self)
            }
        }
    };
}

scalarlike_axis_enum!(
    Pole,
    NUM_POLES = 2,
    PerPole,
    POLES,
    (
        Neg = (-1) = -Pos,
        Pos = (1) = -Neg,
    ),
);

scalarlike_axis_enum!(
    Sign,
    NUM_SIGNS = 3,
    PerSign,
    SIGNS,
    (
        Neg = (-1) = -Pos,
        Zero = (0) = -Zero,
        Pos = (1) = -Neg,
    ),
);

impl Sign {
    pub const fn of_i64(n: i64) -> Self {
        if n > 0 {
            Sign::Pos
        } else if n < 0 {
            Sign::Neg
        } else {
            Sign::Zero
        }
    }

    pub fn of_f32(n: f32) -> Self {
        if n > 0.0 {
            Sign::Pos
        } else if n < 0.0 {
            Sign::Neg
        } else {
            Sign::Zero
        }
    }
}

impl From<Pole> for Sign {
    fn from(pole: Pole) -> Self {
        match pole {
            Pole::Pos => Sign::Pos,
            Pole::Neg => Sign::Neg,
        }
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
                        x: Sign::$pos_x.neg(),
                        y: Sign::$pos_y.neg(),
                        z: Sign::$pos_z.neg(),
                    },
                )*}
            }

            pub /* TODO const */ fn to_vec(self) -> Vec3<i64> {
                self.to_signs().map(|n| n.to_int())
            }

            pub /* ODO const */ fn from_signs(signs: Vec3<Sign>) -> Option<Self> {
                
                $(
                if signs == (Vec3 {
                    x: Sign::$pos_x,
                    y: Sign::$pos_y,
                    z: Sign::$pos_z,
                }) {
                    return Some($name::$pos);
                }
                if signs == (Vec3 {
                    x: Sign::$pos_x.neg(),
                    y: Sign::$pos_y.neg(),
                    z: Sign::$pos_z.neg(),
                }) {
                    return Some($name::$neg);
                }
                )*
                None
                /*
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
                */
            }

            pub /* TODO const */ fn from_vec(vec: Vec3<i64>) -> Option<Self> {
                if let Vec3 {
                    x: Some(x),
                    y: Some(y),
                    z: Some(z),
                } = vec.map(Sign::from_int) {
                    $name::from_signs(Vec3 { x, y, z })
                } else {
                    None
                }
            }
        }

        impl Into<Vec3<Sign>> for $name {
            fn into(self) -> Vec3<Sign> {
                self.to_signs()
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

        impl TryFrom<Vec3<Sign>> for $name {
            type Error = ();

            fn try_from(signs: Vec3<Sign>) -> Result<Self, ()> {
                Self::from_signs(signs).ok_or(())
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
            $face:ident = [$($face_vec:tt)*] = -$face_neg:ident,
        )*),
        edges = ($(
            $edge:ident = [$($edge_vec:tt)*] = -$edge_neg:ident,
        )*),
        corners = ($(
            $corner:ident = [$($corner_vec:tt)*] = -$corner_neg:ident,
        )*),
    )=>{
        veclike_axis_enum!(
            Face,
            NUM_FACES = 6,
            PerFace,
            FACES,
            ($(
                $face = [$($face_vec)*] = -$face_neg,
            )*),
        );

        veclike_axis_enum!(
            Edge,
            NUM_EDGES = 12,
            PerEdge,
            EDGES,
            ($(
                $edge = [$($edge_vec)*] = -$edge_neg,
            )*),
        );

        veclike_axis_enum!(
            Corner,
            NUM_CORNERS = 8,
            PerCorner,
            CORNERS,
            ($(
                $corner = [$($corner_vec)*] = -$corner_neg,
            )*),
        );

        veclike_axis_enum!(
            FaceEdgeCorner,
            NUM_FACES_EDGES_CORNERS = 26,
            PerFaceEdgeCorner,
            FACES_EDGES_CORNERS,
            (
                $( $face = [$($face_vec)*] = -$face_neg, )*
                $( $edge = [$($edge_vec)*] = -$edge_neg, )*
                $( $corner = [$($corner_vec)*] = -$corner_neg, )*
            ),
        );

        impl From<Face> for FaceEdgeCorner {
            fn from(face: Face) -> Self {
                match face {$(
                    Face::$face => FaceEdgeCorner::$face,
                    Face::$face_neg => FaceEdgeCorner::$face_neg,
                )*}
            }
        }

        impl From<Edge> for FaceEdgeCorner {
            fn from(edge: Edge) -> Self {
                match edge {$(
                    Edge::$edge => FaceEdgeCorner::$edge,
                    Edge::$edge_neg => FaceEdgeCorner::$edge_neg,
                )*}
            }
        }

        impl From<Corner> for FaceEdgeCorner {
            fn from(corner: Corner) -> Self {
                match corner {$(
                    Corner::$corner => FaceEdgeCorner::$corner,
                    Corner::$corner_neg => FaceEdgeCorner::$corner_neg,
                )*}
            }
        }
    };
}

fec_system!(
    faces = (
        PosX = [Pos, Zero, Zero] = -NegX,
        PosY = [Zero, Pos, Zero] = -NegY,
        PosZ = [Zero, Zero, Pos] = -NegZ,
    ),
    edges = (
        PosXPosY = [Pos, Pos, Zero] = -NegXNegY,
        PosXNegY = [Pos, Neg, Zero] = -NegXPosY,
        PosYPosZ = [Zero, Pos, Pos] = -NegYNegZ,
        PosYNegZ = [Zero, Pos, Neg] = -NegYPosZ,
        PosXPosZ = [Pos, Zero, Pos] = -NegXNegZ,
        PosXNegZ = [Pos, Zero, Neg] = -NegXPosZ,
    ),
    corners = (
        PosXPosYPosZ = [Pos, Pos, Pos] = -NegXNegYNegZ,
        NegXPosYPosZ = [Neg, Pos, Pos] = -PosXNegYNegZ,
        PosXNegYPosZ = [Pos, Neg, Pos] = -NegXPosYNegZ,
        PosXPosYNegZ = [Pos, Pos, Neg] = -NegXNegYPosZ,
    ),
);

impl Face {
    pub const fn to_axis_pole(self) -> (Axis, Pole) {
        match self {
            Face::PosX => (Axis::X, Pole::Pos),
            Face::NegX => (Axis::X, Pole::Neg),
            Face::PosY => (Axis::Y, Pole::Pos),
            Face::NegY => (Axis::Y, Pole::Neg),
            Face::PosZ => (Axis::Z, Pole::Pos),
            Face::NegZ => (Axis::Z, Pole::Neg),
        }
    }

    pub const fn from_axis_pole(axis: Axis, pole: Pole) -> Self {
        match (axis, pole) {
            (Axis::X, Pole::Pos) => Face::PosX,
            (Axis::X, Pole::Neg) => Face::NegX,
            (Axis::Y, Pole::Pos) => Face::PosY,
            (Axis::Y, Pole::Neg) => Face::NegY,
            (Axis::Z, Pole::Pos) => Face::PosZ,
            (Axis::Z, Pole::Neg) => Face::NegZ,
        }
    }
}

impl Edge {
    pub const fn to_zero_pole_pole(self) -> (Axis, Pole, Pole) {
        match self {
            Edge::PosXPosY => (Axis::Z, Pole::Pos, Pole::Pos),
            Edge::NegXNegY => (Axis::Z, Pole::Neg, Pole::Neg),
            Edge::PosXNegY => (Axis::Z, Pole::Pos, Pole::Neg),
            Edge::NegXPosY => (Axis::Z, Pole::Neg, Pole::Pos),
            Edge::PosYPosZ => (Axis::X, Pole::Pos, Pole::Pos),
            Edge::NegYNegZ => (Axis::X, Pole::Neg, Pole::Neg),
            Edge::PosYNegZ => (Axis::X, Pole::Pos, Pole::Neg),
            Edge::NegYPosZ => (Axis::X, Pole::Neg, Pole::Pos),
            Edge::PosXPosZ => (Axis::Y, Pole::Pos, Pole::Pos),
            Edge::NegXNegZ => (Axis::Y, Pole::Neg, Pole::Neg),
            Edge::PosXNegZ => (Axis::Y, Pole::Pos, Pole::Neg),
            Edge::NegXPosZ => (Axis::Y, Pole::Neg, Pole::Pos),
        }
    }

    pub const fn from_zero_pole_pole(
        zero: Axis,
        pole1: Pole,
        pole2: Pole,
    ) -> Self {
        match (zero, pole1, pole2) {
            (Axis::Z, Pole::Pos, Pole::Pos) => Edge::PosXPosY,
            (Axis::Z, Pole::Neg, Pole::Neg) => Edge::NegXNegY,
            (Axis::Z, Pole::Pos, Pole::Neg) => Edge::PosXNegY,
            (Axis::Z, Pole::Neg, Pole::Pos) => Edge::NegXPosY,
            (Axis::X, Pole::Pos, Pole::Pos) => Edge::PosYPosZ,
            (Axis::X, Pole::Neg, Pole::Neg) => Edge::NegYNegZ,
            (Axis::X, Pole::Pos, Pole::Neg) => Edge::PosYNegZ,
            (Axis::X, Pole::Neg, Pole::Pos) => Edge::NegYPosZ,
            (Axis::Y, Pole::Pos, Pole::Pos) => Edge::PosXPosZ,
            (Axis::Y, Pole::Neg, Pole::Neg) => Edge::NegXNegZ,
            (Axis::Y, Pole::Pos, Pole::Neg) => Edge::PosXNegZ,
            (Axis::Y, Pole::Neg, Pole::Pos) => Edge::NegXPosZ,
        }
    }
}

impl Corner {
    pub const fn to_poles(self) -> Vec3<Pole> {
        match self {
            Corner::PosXPosYPosZ => Vec3 { x: Pole::Pos, y: Pole::Pos, z: Pole::Pos },
            Corner::NegXNegYNegZ => Vec3 { x: Pole::Neg, y: Pole::Neg, z: Pole::Neg },
            Corner::NegXPosYPosZ => Vec3 { x: Pole::Neg, y: Pole::Pos, z: Pole::Pos },
            Corner::PosXNegYNegZ => Vec3 { x: Pole::Pos, y: Pole::Neg, z: Pole::Neg },
            Corner::PosXNegYPosZ => Vec3 { x: Pole::Pos, y: Pole::Neg, z: Pole::Pos },
            Corner::NegXPosYNegZ => Vec3 { x: Pole::Neg, y: Pole::Pos, z: Pole::Neg },
            Corner::PosXPosYNegZ => Vec3 { x: Pole::Pos, y: Pole::Pos, z: Pole::Neg },
            Corner::NegXNegYPosZ => Vec3 { x: Pole::Neg, y: Pole::Neg, z: Pole::Pos },
        }
    }

    pub const fn from_poles(poles: Vec3<Pole>) -> Self {
        match poles {
            Vec3 { x: Pole::Pos, y: Pole::Pos, z: Pole::Pos } => Corner::PosXPosYPosZ,
            Vec3 { x: Pole::Neg, y: Pole::Neg, z: Pole::Neg } => Corner::NegXNegYNegZ,
            Vec3 { x: Pole::Neg, y: Pole::Pos, z: Pole::Pos } => Corner::NegXPosYPosZ,
            Vec3 { x: Pole::Pos, y: Pole::Neg, z: Pole::Neg } => Corner::PosXNegYNegZ,
            Vec3 { x: Pole::Pos, y: Pole::Neg, z: Pole::Pos } => Corner::PosXNegYPosZ,
            Vec3 { x: Pole::Neg, y: Pole::Pos, z: Pole::Neg } => Corner::NegXPosYNegZ,
            Vec3 { x: Pole::Pos, y: Pole::Pos, z: Pole::Neg } => Corner::PosXPosYNegZ,
            Vec3 { x: Pole::Neg, y: Pole::Neg, z: Pole::Pos } => Corner::NegXNegYPosZ,
        }
    }
}

impl Into<Vec3<Pole>> for Corner {
    fn into(self) -> Vec3<Pole> {
        self.to_poles()
    }
}

impl From<Vec3<Pole>> for Corner {
    fn from(poles: Vec3<Pole>) -> Self {
        Self::from_poles(poles)
    }
}
