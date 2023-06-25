
use crate::{
    asset::consts::*,
	game_data::GameData,
	singleplayer::tile_meshing::mesh_simple_face,
	util::hex_color::hex_color,
};
use chunk_data::{
	FACES,
	Face,
	PerChunk,
	ChunkBlocks,
	Getter,
};
use mesh_data::MeshData;
use vek::*;


pub fn mesh_grass_tile(
    mesh_buf: &mut MeshData,
    gtc: Vec3<i64>,
    getter: &Getter,
    tile_blocks: &PerChunk<ChunkBlocks>,
    game: &GameData,
) {
	for face in FACES {
        let grass_color = hex_color(0x74b44aff) / hex_color(0x969696ff);
        let (tex_index, color) = match face {
            Face::PosY => (BTI_GRASS_TOP, grass_color),
            Face::NegY => (BTI_DIRT, Rgba::white()),
            _ => (BTI_GRASS_SIDE, Rgba::white()),
        };
        mesh_simple_face(
            mesh_buf,
            face,
            tex_index,
            color,
            gtc,
            getter,
            tile_blocks,
            game,
        );
    }
}