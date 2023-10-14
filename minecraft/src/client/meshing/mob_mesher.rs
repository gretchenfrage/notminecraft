
use graphics::prelude::*;
use mesh_data::*;
use chunk_data::*;
use vek::*;
use std::ops::Deref;


/// Utility for making mob body part meshes.
#[derive(Debug)]
pub struct MobMesher<G> {
    pub gpu_vec_ctx: G,
    pub tex_size: Extent2<u32>,
}

impl<G> MobMesher<G>
where
    G: Deref,
    G::Target: GpuVecContext
{
    pub fn make_part(
        &self,
        offset: impl Into<Vec2<u32>>,
        extent: impl Into<Extent3<u32>>,
        origin_frac: impl Into<Vec3<f32>>,
    ) -> Mesh {
        // convert stuff into vectors
        let offset = offset.into();
        let extent = Vec3::<u32>::from(extent.into());
        let origin_frac = origin_frac.into();

        // convert stuff into floats
        let tex_size = self.tex_size.map(|n| n as f32);
        let offset = offset.map(|n| n as f32);
        let extent = extent.map(|n| n as f32);

        // compose mesh from faces
        let mut mesh = MeshData::new();
        let scaled_extent = Vec3::from(extent);
        let origin_adjust = -(extent * origin_frac);
        for face in FACES {
            let (face_start, face_extents) = face.quad_start_extents();
            let pos_start = face_start
                .to_poles()
                .zip(scaled_extent)
                .map(|(pole, n)| match pole {
                    Pole::Neg => 0.0,
                    Pole::Pos => n,
                }) + origin_adjust;
            let [pos_ext_1, pos_ext_2] = face_extents
                .map(|ext_face| {
                    let (ext_axis, ext_pole) = ext_face.to_axis_pole();
                    let n = PerAxis::from(scaled_extent)[ext_axis] * ext_pole.to_int() as f32;
                    ext_axis.to_vec(n)
                });
            let tex_start =
                (offset + Vec2::from(match face {
                    Face::PosX => [0.0, extent.z],
                    Face::NegX => [extent.z + extent.x, extent.z],
                    Face::PosY => [extent.z, 0.0],
                    Face::NegY => [extent.z + extent.x, 0.0],
                    Face::PosZ => [extent.z, extent.z],
                    Face::NegZ => [extent.z * 2.0 + extent.x, extent.z],
                })) / tex_size;
            let tex_extent = Vec2::from(face
                .to_axis()
                .other_axes()
                .map(
                    |axis| PerAxis::from(extent)[axis]
                )) / tex_size;

            mesh.add_quad(&Quad {
                pos_start,
                pos_ext_1: pos_ext_1.into(),
                pos_ext_2: pos_ext_2.into(),
                tex_start,
                tex_extent: tex_extent.into(),
                vert_colors: [Rgba::white(); 4],
                tex_index: 0,
            });
        }

        // upload
        mesh.upload(&*self.gpu_vec_ctx)
    }
}
