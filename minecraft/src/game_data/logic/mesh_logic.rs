
use chunk_data::*;


#[derive(Debug)]
pub enum BlockMeshLogic {
    NoMesh,
    FullCube(BlockMeshLogicFullCube),
}

#[derive(Debug, Copy, Clone)]
pub struct BlockMeshLogicFullCube {
    pub tex_indices: PerFace<usize>,
    pub transparent: bool,
    pub rgb_u8_meta: bool,
}

impl BlockMeshLogic {
    pub fn basic_cube(tex_index: usize) -> Self {
        BlockMeshLogic::FullCube(BlockMeshLogicFullCube {
            tex_indices: PerFace::repeat(tex_index),
            transparent: false,
            rgb_u8_meta: false,
        })
    }

    pub fn basic_cube_faces(tex_indices: PerFace<usize>) -> Self {
        BlockMeshLogic::FullCube(BlockMeshLogicFullCube {
            tex_indices,
            transparent: false,
            rgb_u8_meta: false,
        })
    }

    pub fn basic_cube_transparent(tex_index: usize) -> Self {
        BlockMeshLogic::FullCube(BlockMeshLogicFullCube {
            tex_indices: PerFace::repeat(tex_index),
            transparent: true,
            rgb_u8_meta: false,
        })
    }

    pub fn basic_cube_rgb_u8_meta(tex_index: usize) -> Self {
        BlockMeshLogic::FullCube(BlockMeshLogicFullCube {
            tex_indices: PerFace::repeat(tex_index),
            transparent: false,
            rgb_u8_meta: true,
        })
    }

    pub fn obscures(&self, _face: Face) -> bool {
        match self {
            &BlockMeshLogic::NoMesh => false,
            &BlockMeshLogic::FullCube(mesh_logic) => !mesh_logic.transparent,
        }
    }
}
