
mod local;
mod global;
mod chunk;


pub use crate::{
    local::{
        Ltc,
        ltc,
    },
    global::{
        Gbc,
        gbc,
    },
    chunk::{
        Chc,
        chc,
    },
};


#[test]
fn test_tile_coord_packing() {
    for i in 0..=0xffff {
        let c = Ltc(i);
        assert_eq!(
            i,
            ltc(c.x(), c.y(), c.z()).0,
        );
    }
}


#[test]
fn test_coord_splitting_joining() {
    for x in -30..30 {
        for y in 0..=0xff {
            for z in -30..30 {
                let c = gbc(x, y, z);
                assert_eq!(
                    c,
                    Gbc::from_parts(
                        c.to_chunk(),
                        c.to_local(),
                    )
                )
            }
        }
    }
}
