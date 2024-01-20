//! Asynchronously generating graphical meshes for chunks.


use crate::util_abort_handle::AbortGuard;
use graphics::frame_content::Mesh;
use mesh_data::MeshDiffer;
use chunk_data::*;


/// State for a chunk in terms of it being meshed. After a chunk is added to the client, the
/// uploading of the chunk's mesh to the GPU is done asynchronously in the background.
pub enum ChunkMeshState {
    /// Chunk is still being meshed.
    Meshing(ChunkMeshing),
    /// Chunk has been meshed.
    Meshed(ChunkMeshed),
}

/// Variant of `ChunkMeshState` for when chunk is still being meshed.
pub struct ChunkMeshing {
    /// Abort guard for the request to mesh the chunk.
    pub abort_guard: AbortGuard,
    /// Whether each tile potentially had its mesh changed since the request to mesh the chunk was
    /// submitted.
    pub tile_dirty: PerTileBool,
    /// List of tiles set to true in tile_dirty.
    pub dirty_tiles: Vec<u16>,
}

/// Variant of `ChunkMeshState` for when chunk has been meshed.
pub struct ChunkMeshed {
    /// The GPU mesh
    pub mesh: Mesh,
    /// The mesh differ, which organizes the internal structure of mesh in a way that allows'
    /// variable-length submeshes to be efficiently added and removed, and tracks the buffered
    /// GPU mesh writes involved in such.
    pub differ: MeshDiffer,
    /// For every tile with a non-empty mesh, its submesh key in the mesh differ.
    pub submesh_keys: PerTileOption<u16>,
    /// Whether there are mesh edits buffered in the mesh differ.
    pub dirty: bool,
}
