//! Glue between this library and serde. Makes `&mut Encoder` implement
//! `serde::Serializer`. Some notes on how translations occur:
//!
//! - unit structs and unit variants are encoded simply as unit
//! - newtype structs and newtype variants are encoded simply as the inner
//!   value
//! - tuple structs and tuple variants are encoded simply as tuple
//! - upon encoding a seq, uses the `.need()` function to determine whether the
//!   schema expects a fixed len or var len seq, which implies the associated
//!   warning
//! - a map is encoded as a var len seq of (key, value) tuples
//! - when asked to "skip a struct field", it tries encoding a none value for
//!   that field

pub mod ser;
pub mod de;
