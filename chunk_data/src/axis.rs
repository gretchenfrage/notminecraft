
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
        $doc:literal,
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
        #[doc = $doc]
        pub enum $name {$(
            $variant,
        )*}

        #[doc = concat!("Number of variants of `", stringify!($name), "`.")]
        pub const $num_constant: usize = $num;

        #[doc = concat!("Array of `T` for each `", stringify!($name), "`.")]
        #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub struct $per_name<T>(pub [T; $num_constant]);

        #[doc = concat!("All variants of `", stringify!($name), "`.")]
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
    "{X, Y, Z}",
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

impl Axis {
    pub fn other_axes(self) -> [Axis; 2] {
        match self {
            Axis::X => [Axis::Y, Axis::Z],
            Axis::Y => [Axis::Z, Axis::X],
            Axis::Z => [Axis::X, Axis::Y],
        }
    }
}

macro_rules! scalarlike_axis_enum {
    (
        $doc:literal,
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
            $doc,
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
    "Positive or negative.",
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
    "Positive, negative, or zero.",
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

impl Pole {
    pub const fn from_sign(sign: Sign) -> Option<Self> {
        match sign {
            Sign::Pos => Some(Pole::Pos),
            Sign::Neg => Some(Pole::Neg),
            Sign::Zero => None,
        }
    }
}

impl TryFrom<Sign> for Pole {
    type Error = ();

    fn try_from(sign: Sign) -> Result<Self, ()> {
        Pole::from_sign(sign).ok_or(())
    }
}

macro_rules! veclike_axis_enum {
    (
        $doc:literal,
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
            $doc,
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
            "Any of 6 directions going straight along an axis, like axis-aligned cube faces.",
            Face,
            NUM_FACES = 6,
            PerFace,
            FACES,
            ($(
                $face = [$($face_vec)*] = -$face_neg,
            )*),
        );

        veclike_axis_enum!(
            "Any of 12 directions going diagonally between 2 axes, like axis-aligned unit cube edges.",
            Edge,
            NUM_EDGES = 12,
            PerEdge,
            EDGES,
            ($(
                $edge = [$($edge_vec)*] = -$edge_neg,
            )*),
        );

        veclike_axis_enum!(
            "Any of 8 directions going diagonally between all 3 axes, like axis-aligned unit cube corners.",
            Corner,
            NUM_CORNERS = 8,
            PerCorner,
            CORNERS,
            ($(
                $corner = [$($corner_vec)*] = -$corner_neg,
            )*),
        );

        veclike_axis_enum!(
            "Any of 26 Faces, Edges, or Corners. A non-zero 3-vector in which all components are 0, 1, or -1.",
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

    pub const fn to_axis(self) -> Axis {
        self.to_axis_pole().0
    }

    pub const fn to_pole(self) -> Pole {
        self.to_axis_pole().1
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

    pub const fn to_edges(self) -> [Edge; 4] {
        match self {
            Face::PosX => [Edge::PosXNegY, Edge::PosXPosY, Edge::PosXNegZ, Edge::PosXPosZ],
            Face::NegX => [Edge::NegXNegY, Edge::NegXPosY, Edge::NegXNegZ, Edge::NegXPosZ],
            Face::PosY => [Edge::NegXPosY, Edge::PosXPosY, Edge::PosYNegZ, Edge::PosYPosZ],
            Face::NegY => [Edge::NegXNegY, Edge::PosXNegY, Edge::NegYNegZ, Edge::NegYPosZ],
            Face::PosZ => [Edge::NegXPosZ, Edge::PosXPosZ, Edge::NegYPosZ, Edge::PosYPosZ],
            Face::NegZ => [Edge::NegXNegZ, Edge::PosXNegZ, Edge::NegYNegZ, Edge::PosYNegZ],
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

    pub const fn to_corners(self) -> [Corner; 2] {
        match self {
            Edge::PosXPosY => [Corner::PosXPosYNegZ, Corner::PosXPosYPosZ],
            Edge::NegXNegY => [Corner::NegXNegYNegZ, Corner::NegXNegYPosZ],
            Edge::PosXNegY => [Corner::PosXNegYNegZ, Corner::PosXNegYPosZ],
            Edge::NegXPosY => [Corner::NegXPosYNegZ, Corner::NegXPosYPosZ],
            Edge::PosYPosZ => [Corner::NegXPosYPosZ, Corner::PosXPosYPosZ],
            Edge::NegYNegZ => [Corner::NegXNegYNegZ, Corner::PosXNegYNegZ],
            Edge::PosYNegZ => [Corner::NegXPosYNegZ, Corner::PosXPosYNegZ],
            Edge::NegYPosZ => [Corner::NegXNegYPosZ, Corner::PosXNegYPosZ],
            Edge::PosXPosZ => [Corner::PosXNegYPosZ, Corner::PosXPosYPosZ],
            Edge::NegXNegZ => [Corner::NegXNegYNegZ, Corner::NegXPosYNegZ],
            Edge::PosXNegZ => [Corner::PosXNegYNegZ, Corner::PosXPosYNegZ],
            Edge::NegXPosZ => [Corner::NegXNegYPosZ, Corner::NegXPosYPosZ],
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
