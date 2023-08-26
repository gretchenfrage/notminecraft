
use binschema::{
    error::Result,
    Encoder,
    Decoder,
    KnownSchema,
    Schema,
    schema,
};
use serde::{
    Serialize,
    Deserialize,
};
use vek::*;


pub fn vec3_schema<T: KnownSchema>() -> Schema {
    schema!(seq(3)(%T::schema(Default::default())))
}

pub fn vec3_encode<T: Serialize, V: Into<Vec3<T>>>(
    vec: V,
    encoder: &mut Encoder<Vec<u8>>,
) -> Result<()> {
    encoder.begin_fixed_len_seq(3)?;
    for n in vec.into() {
        encoder.begin_seq_elem()?;
        n.serialize(&mut *encoder)?;
    }
    encoder.finish_seq()
}

pub fn vec3_decode<T: for<'d> Deserialize<'d>, V: From<Vec3<T>>>(
    decoder: &mut Decoder<&[u8]>
) -> Result<V> {
    decoder.begin_fixed_len_seq(3)?;
    let mut vec = Vec3::new(None, None, None);
    for i in 0..3 {
        decoder.begin_seq_elem()?;
        vec[i] = Some(T::deserialize(&mut *decoder)?);
    }
    decoder.finish_seq()?;
    Ok(vec.map(|opt| opt.unwrap()).into())
}
