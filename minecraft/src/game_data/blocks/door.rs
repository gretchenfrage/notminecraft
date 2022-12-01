
use chunk_data::Face;


#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct DoorMeta {
    pub part: DoorPart,
    pub dir: DoorDir,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum DoorPart {
    Upper,
    Lower,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum DoorDir {
    PosX,
    NegX,
    PosZ,
    NegZ,
}

impl DoorDir {
    pub fn to_face(self) -> Face {
        match self {
            DoorDir::PosX => Face::PosX,
            DoorDir::NegX => Face::NegX,
            DoorDir::PosZ => Face::PosZ,
            DoorDir::NegZ => Face::NegZ,
        }
    }
}

#[test]
fn door_is_inline() {
    assert!(std::mem::size_of::<DoorMeta>() <= 2);
}
