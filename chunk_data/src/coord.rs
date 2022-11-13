//! Coordinate bit-fiddling.

use vek::*;


/// Max local tile index.
pub const MAX_LTI: u16 = 0xffff;

/// Number of local tile indices.
pub const NUM_LTIS: usize = 0x10000;

/// Max local tile coordinate x component.
pub const MAX_LTC_X: u16 = XZ_MAX;

/// Max local tile coordinate y component.
pub const MAX_LTC_Y: u16 = Y_MAX;

/// Max local tile coordinate z component.
pub const MAX_LTC_Z: u16 = XZ_MAX;


const XZ_MAX: u16 =          0b11111;
const XZ_HI_PACK_MASK: u16 = 0b10000;
const XZ_LO_PACK_MASK: u16 = 0b01111;

const Y_MAX: u16 =          0b111111;
const Y_HI_PACK_MASK: u16 = 0b110000;
const Y_LO_PACK_MASK: u16 = 0b001111;

const Z_HI_UNPACK_MASK: u16 = 0b1000000000000000;
const Y_HI_UNPACK_MASK: u16 = 0b0110000000000000;
const X_HI_UNPACK_MASK: u16 = 0b0001000000000000;
const Z_LO_UNPACK_MASK: u16 = 0b0000111100000000;
const Y_LO_UNPACK_MASK: u16 = 0b0000000011110000;
const X_LO_UNPACK_MASK: u16 = 0b0000000000001111;

const Z_HI_SHIFT: usize = 11;
const Y_HI_SHIFT: usize = 9;
const X_HI_SHIFT: usize = 8;
const Z_LO_SHIFT: usize = 8;
const Y_LO_SHIFT: usize = 4;

const XZ_BITS: usize = 5;
const Y_BITS: usize = 6;

fn validate_ltc<N>(ltc: Vec3<N>) -> Vec3<u16>
where
    N: TryInto<u16>,
{
    let x = ltc.x.try_into().ok().expect("ltc x out of range");
    let y = ltc.y.try_into().ok().expect("ltc y out of range");
    let z = ltc.z.try_into().ok().expect("ltc z out of range");

    assert!(x <= XZ_MAX, "ltc x out of range");
    assert!(y <= Y_MAX, "ltc y out of range");
    assert!(z <= XZ_MAX, "ltc z out of range");

    Vec3 { x, y, z }
}

/// Convert local tile coordinate to local tile index.
///
/// Panics if out of range.
pub fn ltc_to_lti<N>(ltc: Vec3<N>) -> u16
where
    N: TryInto<u16>,
{
    let ltc = validate_ltc(ltc);

    ((ltc.z & XZ_HI_PACK_MASK) << Z_HI_SHIFT)
    | ((ltc.y & Y_HI_PACK_MASK) << Y_HI_SHIFT)
    | ((ltc.x & XZ_HI_PACK_MASK) << X_HI_SHIFT)
    | ((ltc.z & XZ_LO_PACK_MASK) << Z_LO_SHIFT)
    | ((ltc.y & Y_LO_PACK_MASK) << Y_LO_SHIFT)
    | (ltc.x & XZ_LO_PACK_MASK)
}

/// Get x component of local tile index.
pub fn lti_get_x(lti: u16) -> u16 {
    ((lti & X_HI_UNPACK_MASK) >> X_HI_SHIFT)
    | (lti & X_LO_UNPACK_MASK)
}

/// Get y component of local tile index.
pub fn lti_get_y(lti: u16) -> u16 {
    ((lti & Y_HI_UNPACK_MASK) >> Y_HI_SHIFT)
    | ((lti & Y_LO_UNPACK_MASK) >> Y_LO_SHIFT)
}

/// Get z component of local tile index.
pub fn lti_get_z(lti: u16) -> u16 {
    ((lti & Z_HI_UNPACK_MASK) >> Z_HI_SHIFT)
    | ((lti & Z_LO_UNPACK_MASK) >> Z_LO_SHIFT)
}

/// Set x component of local tile index in-place.
pub fn lti_set_x<N: TryInto<u16>>(lti: &mut u16, n: N) {
    let n = n.try_into().ok().expect("out of range");
    assert!(n <= XZ_MAX, "out of range");

    *lti = *lti
        & !(X_HI_UNPACK_MASK | X_LO_UNPACK_MASK)
        | ((n & XZ_HI_PACK_MASK) << X_HI_SHIFT)
        | (n & XZ_LO_PACK_MASK);
}

/// Set y component of local tile index in-place.
pub fn lti_set_y<N: TryInto<u16>>(lti: &mut u16, n: N) {
    let n = n.try_into().ok().expect("out of range");
    assert!(n <= Y_MAX, "out of range");

    *lti = *lti
        & !(Y_HI_UNPACK_MASK | Y_LO_UNPACK_MASK)
        | ((n & Y_HI_PACK_MASK) << Y_HI_SHIFT)
        | ((n * Y_LO_PACK_MASK) << Y_LO_SHIFT);
}

/// Set z component of local tile index in-place.
pub fn lti_set_z<N: TryInto<u16>>(lti: &mut u16, n: N) {
    let n = n.try_into().ok().expect("out of range");
    assert!(n <= XZ_MAX, "out of range");

    *lti = *lti
        & !(Z_HI_UNPACK_MASK | Z_LO_UNPACK_MASK)
        | ((n & XZ_HI_PACK_MASK) << Z_HI_SHIFT)
        | ((n & XZ_LO_PACK_MASK) << Z_LO_SHIFT);
}

/// Convert local tile index to local tile coordinate.
pub fn lti_to_ltc(lti: u16) -> Vec3<u16> {
    Vec3 {
        x: lti_get_x(lti),
        y: lti_get_y(lti),
        z: lti_get_z(lti),
    }
}

/// Get chunk coordinate part of global tile coordinate.
pub fn gtc_get_cc(gtc: Vec3<i64>) -> Vec3<i64> {
    Vec3 {
        x: (gtc.x & !(XZ_MAX as i64)) >> XZ_BITS,
        y: (gtc.y & !(Y_MAX as i64)) >> Y_BITS,
        z: (gtc.z & !(XZ_MAX as i64)) >> XZ_BITS,
    }
}

/// Get local tile coordinate part of global tile coordinate.
pub fn gtc_get_ltc(gtc: Vec3<i64>) -> Vec3<u16> {
    Vec3 {
        x: (gtc.x & (XZ_MAX as i64)) as u16,
        y: (gtc.y & (Y_MAX as i64)) as u16,
        z: (gtc.z & (XZ_MAX as i64)) as u16,
    }
}

/// Get local tile index part of global tile coordinate.
pub fn gtc_get_lti(gtc: Vec3<i64>) -> u16 {
    (((gtc.z & (XZ_HI_PACK_MASK) as i64) as u16) << Z_HI_SHIFT)
    | (((gtc.y & (Y_HI_PACK_MASK) as i64) as u16) << Y_HI_SHIFT)
    | (((gtc.x & (XZ_HI_PACK_MASK) as i64) as u16) << X_HI_SHIFT)
    | (((gtc.z & (XZ_LO_PACK_MASK) as i64) as u16) << Z_LO_SHIFT)
    | (((gtc.y & (Y_LO_PACK_MASK) as i64) as u16) << Y_LO_SHIFT)
    | ((gtc.x & (XZ_LO_PACK_MASK) as i64) as u16)
}

/// Combine chunk coordinate and local tile coordinate into global tile
/// coordinate.
pub fn cc_ltc_to_gtc<N>(cc: Vec3<i64>, ltc: Vec3<N>) -> Vec3<i64>
where
    N: TryInto<u16>,
{
    let ltc = validate_ltc(ltc);
    Vec3 {
        x: (cc.x << (XZ_BITS as i64)) | (ltc.x as i64),
        y: (cc.y << (Y_BITS as i64)) | (ltc.y as i64),
        z: (cc.z << (XZ_BITS as i64)) | (ltc.z as i64),
    }
}
