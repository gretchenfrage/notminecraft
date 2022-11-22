
use graphics::{
    Renderer,
    frame_content::Mesh,
};
use mesh_data::{
    MeshData,
    MeshDiffer,
};
use chunk_data::{
    PerTileOption,
    LtiSet,
};


#[derive(Debug)]
pub struct ChunkMesh {
    mesh: Mesh,
    /*TODO*/ pub differ: MeshDiffer,
    keys: PerTileOption<u16>,
    clean: bool,
}

impl ChunkMesh {
    pub fn new(renderer: &Renderer) -> Self {
        ChunkMesh {
            mesh: Mesh {
                vertices: renderer.create_gpu_vec(),
                indices: renderer.create_gpu_vec(),
            },
            differ: MeshDiffer::new(),
            keys: PerTileOption::new(),
            clean: true,
        }
    }

    pub fn set_tile_submesh(&mut self, lti: u16, submesh_data: &MeshData) {
        self.clean = false;

        self.clear_tile_submesh(lti);

        if !submesh_data.indices.is_empty() {
            let key = self.differ.add_submesh(submesh_data);
            self.keys.set_some(lti, key as u16);
        }
    }
    
    pub fn clear_tile_submesh(&mut self, lti: u16) {
        self.clean = false;

        if let Some(&key) = self.keys.get(lti) {
            self.differ.remove_submesh(key as usize);
            self.keys.set_none(lti);
        }
    }

    pub fn patch(&mut self, renderer: &Renderer) {
        let (vertices_diff, indices_diff) = self.differ.diff();

        vertices_diff.patch(&mut self.mesh.vertices, renderer);
        indices_diff.patch(&mut self.mesh.indices, renderer);

        self.clean = true;
    }

    pub fn mesh(&self) -> &Mesh {
        assert!(self.clean);
        &self.mesh
    }
}

impl<'a, 'b> LtiSet<&'a MeshData> for &'b mut ChunkMesh {
    fn set(self, lti: u16, val: &'a MeshData) {
        self.set_tile_submesh(lti, val);
    }
}
