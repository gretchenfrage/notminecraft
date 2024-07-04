
use crate::game_binschema::GameBinschema;
use vek::*;


#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, GameBinschema)]
pub enum EntityKind {
    Steve,
    Pig,
}

#[derive(Debug, Clone, GameBinschema)]
pub struct SteveEntityState {
    pub vel: Vec3<f32>,
    pub name: String,
}

#[derive(Debug, Copy, Clone, GameBinschema)]
pub struct PigEntityState {
    pub vel: Vec3<f32>,
    pub color: Rgb<f32>,
}
