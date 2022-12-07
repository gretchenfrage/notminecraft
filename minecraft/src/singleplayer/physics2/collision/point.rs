
use super::{
    super::{
        aa_box::AaBox,
        world_geometry::WorldGeometry,
    },
    CollisionObject,
    Collision,
};
use chunk_data::{
    AXES,
    Face,
    Pole,
    Sign,
    PerAxis,
};
use vek::*;

/*
#[derive(Debug, Copy, Clone)]
pub struct Hitscanner {
    min_dt: f32,
    max_dt: f32,
    vel: Vec3<f32>,

    dt: f32,
    pos: Vec3<f32>,
    face: Option<Face>,
}

impl Hitscanner {

}*/


#[derive(Debug, Copy, Clone)]
pub struct PointCollisionObject;

impl CollisionObject for PointCollisionObject {
    fn first_collision<W: WorldGeometry>(
        &self,
        min_dt: f32,
        max_dt: f32,
        pos: Vec3<f32>,
        vel: Vec3<f32>,
        world_geometry: &W,
    ) -> Option<Collision<W::BarrierId>>
    {
        let vel_poles: PerAxis<Option<Pole>> = AXES
            .map(|axis| Pole::from_sign(Sign::of_f32(vel[axis as usize])));

        let mut pos = pos + min_dt * vel;
        let mut gtc = pos.map(|n| n.floor() as i64);
        let mut dt = min_dt;

        while dt < max_dt {
            world_geometry.tile_geometry(gtc, |aa_box, world_obj_id| {
                for axis in AXES {
                    let vel_pole =
                        match vel_poles[axis] {
                            Some(pole) => pole,
                            None => continue,
                        };


                }
            });
        }

        unimplemented!()
    }
}
