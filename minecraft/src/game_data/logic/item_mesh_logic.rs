
use chunk_data::*;


#[derive(Debug)]
pub enum ItemMeshLogic {
    FullCube {
        top_tex_index: usize,
        left_tex_index: usize,
        right_tex_index: usize,
    }
}

impl ItemMeshLogic {
    pub fn basic_cube(tex_index: usize) -> Self {
        ItemMeshLogic::FullCube {
            top_tex_index: tex_index,
            left_tex_index: tex_index,
            right_tex_index: tex_index,
        }
    }
}
