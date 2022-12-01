
use crate::game_data::{
	BTI_LOG_SIDE,
	BTI_LOG_TOP,
	BlockMeshLogic,
};
use chunk_data::{
	FACES,
	Axis,
};


pub fn log_mesh_logic() -> BlockMeshLogic {
	BlockMeshLogic::BasicCubeFaces(FACES.map(|face|
		match face.to_axis() {
	        Axis::Y => BTI_LOG_TOP,
	        _ => BTI_LOG_SIDE,
	    }
	))
}
