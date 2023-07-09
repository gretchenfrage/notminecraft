//! This serialization system is designed around the idea that that a _schema_,
//! a specification for what values are permitted and how they're encoded as
//! raw bytes, is a data structure that can be manipulated programmatically
//! at runtime and itself serialized. This can be used to achieve
//! bincode-levels of efficiency, protobufs levels of validation, and JSON
//! levels of easy debugging. For example, one could arrange a key/value store
//! such that the store contains, on-disk, the serialized schemas for the keys
//! and the values. Or, an RPC protocol could be designed such that, upon
//! initialization, the server sends down its list of endpoints and the
//! serialized schemas for their parameters and return types.
//!
//! Typical usage pattern:
//!
//! - create `CoderStateAlloc`
//! - to encode (serialize) a value:
//!     1. combine `&Schema` and `CoderStateAlloc` into `CoderState`
//!     2. combine `&mut CoderState` and `&mut W` where `W: Write` into `Encoder`
//!     3. pass `&mut Encoder` and `&`value into procedure for encoding value
//!     4. on `CoderState`, call `.is_finished_or_err()?` to guarantee that
//!        valid schema-comformant data was fully written to `W`
//!     5. convert `CoderState` back into `CoderStateAlloc` so it can be reused
//! - to decode (deserialize) a value:
//!     1. combine `&Schema` and `CoderStateAlloc` into `CoderState`
//!     2. combine `&mut CoderState` and `&mut R` where `R: Read` into `Decoder`
//!     3. pass `&mut Decoder` into procedure for decoding value
//!     4. on `CoderState`, call `.is_finished_or_err()?` to guarantee that
//!        valid schema-comformant data was fully read from `R`, and no more
//!     5. convert `CoderState` back into `CoderStateAlloc` so it can be reused
//!
//! The data model supports:
//!
//! - `u8` through `u128`, `i8` through `i128`(32 bits and above are encoded
//!    variable length)
//! - `f32` and `f64`, `char`, `bool`
//! - utf8 string, byte string
//! - option
//! - fixed length array, variable length array
//! - tuple (just values back-to-back)
//! - struct (just values back-to-back, but at schema-time the fields have 
//!   names)
//! - enum, as in rust-style enum, as in tagged union, as in "one of"
//! - recursing up in the schema, so as to support recursive schema types like
//!   trees


pub mod error;
pub mod value;

mod schema;
mod known_schema;
mod do_if_err;
mod var_len;
mod coder;
mod encoder;
mod decoder;
mod serde;

pub use crate::{
    coder::{
        coder::CoderState,
        coder_alloc::CoderStateAlloc,
    },
    encoder::Encoder,
    decoder::Decoder,
    known_schema::{
        KnownSchema,
        RecurseStack,
    },
    schema::{
        Schema,
        ScalarType,
        SeqSchema,
        StructSchemaField,
        EnumSchemaVariant,
    },
};
pub use binschema_derive::KnownSchema;


use crate::error::Result;
use std::io::{Write, Read};
use ::serde::{Serialize, Deserialize};


pub fn serde_to_writer<T, W>(val: &T, write: &mut W) -> Result<()>
where
    T: KnownSchema + Serialize,
    W: Write,
{
    let schema = T::schema(Default::default());
    let coder_alloc = CoderStateAlloc::new();
    let mut coder = CoderState::new(&schema, coder_alloc, None);
    let mut encoder = Encoder::new(&mut coder, write);
    val.serialize(&mut encoder)
}

pub fn serde_to_vec<T>(val: &T) -> Result<Vec<u8>>
where
    T: KnownSchema + Serialize,
{
    let mut vec = Vec::new();
    serde_to_writer(val, &mut vec)?;
    Ok(vec)
}

pub fn serde_from_reader<T, R>(read: &mut R) -> Result<T>
where
    T: KnownSchema + for<'d> Deserialize<'d>,
    R: Read,
{
    let schema = T::schema(Default::default());
    let coder_alloc = CoderStateAlloc::new();
    let mut coder = CoderState::new(&schema, coder_alloc, None);
    let mut decoder = Decoder::new(&mut coder, read);
    T::deserialize(&mut decoder)
}

pub fn serde_from_slice<T>(mut slice: &[u8]) -> Result<T>
where
    T: KnownSchema + for<'d> Deserialize<'d>,
{
    serde_from_reader(&mut slice)
}
