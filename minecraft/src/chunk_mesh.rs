
use graphics::{
    Renderer,
    GpuVecContext,
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
    mesh: Option<Mesh>,
    differ: MeshDiffer,
    keys: PerTileOption<u16>,
    clean: bool,
}

impl ChunkMesh {
    pub fn new() -> Self {
        ChunkMesh {
            mesh: None,
            differ: MeshDiffer::new(),
            keys: PerTileOption::new(),
            clean: false,
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

    pub fn patch<T>(&mut self, gpu_vec_context: &T)
    where
        T: GpuVecContext,
    {
        let (vertices_diff, indices_diff) = self.differ.diff();

        if self.mesh.is_none() {
            self.mesh = Some(Mesh {
                vertices: gpu_vec_context.create_gpu_vec(),
                indices: gpu_vec_context.create_gpu_vec(),
            });
        }
        let mesh = self.mesh.as_mut().unwrap();

        vertices_diff.patch(&mut mesh.vertices, gpu_vec_context);
        indices_diff.patch(&mut mesh.indices, gpu_vec_context);

        self.clean = true;
    }

    pub fn mesh(&self) -> &Mesh {
        assert!(self.clean);
        self.mesh.as_ref().unwrap()
    }
}

impl<'a, 'b> LtiSet<&'a MeshData> for &'b mut ChunkMesh {
    fn set(self, lti: u16, val: &'a MeshData) {
        self.set_tile_submesh(lti, val);
    }
}
