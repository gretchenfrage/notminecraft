//! Dynamic representation of data within the serialized data model, analogous
//! to `serde_json::Value`.

use crate::{
    error::Result,
    schema::{
        Schema,
        ScalarType,
        SeqSchema,
    },
    Encoder,
    Decoder,
};
use std::io::{
    Write,
    Read,
};


#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Value {
    Scalar(ScalarValue),
    Str(String),
    Bytes(Vec<u8>),
    Unit,
    Option(Option<Box<Value>>),
    FixedLenSeq(Vec<Value>),
    VarLenSeq(Vec<Value>),
    Tuple(Vec<Value>),
    Struct(Vec<StructValueField>),
    Enum(EnumValue),
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub enum ScalarValue {
    U8(u8), U16(u16), U32(u32), U64(u64), U128(u128),
    I8(i8), I16(i16), I32(i32), I64(i64), I128(i128),
    F32(f32), F64(f64),
    Char(char),
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct StructValueField {
    pub name: String,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct EnumValue {
    pub variant_ord: usize,
    pub variant_name: String,
    pub value: Box<Value>,
}


impl Value {
    pub fn encode_to<W: Write>(&self, e: &mut Encoder<W>) -> Result<()> {
        match self {
            &Value::Scalar(s) => s.encode_to(e),
            &Value::Str(ref s) => e.encode_str(s),
            &Value::Bytes(ref b) => e.encode_bytes(b),
            &Value::Unit => e.encode_unit(),
            &Value::Option(None) => e.encode_none(),
            &Value::Option(Some(ref value)) => {
                e.begin_some()?;
                value.encode_to(e)
            }
            &Value::FixedLenSeq(ref elems) => {
                e.begin_fixed_len_seq(elems.len())?;
                for elem in elems {
                    e.begin_seq_elem()?;
                    elem.encode_to(e)?;
                }
                e.finish_seq()
            }
            &Value::VarLenSeq(ref elems) => {
                e.begin_var_len_seq(elems.len())?;
                for elem in elems {
                    e.begin_seq_elem()?;
                    elem.encode_to(e)?;
                }
                e.finish_seq()
            }
            &Value::Tuple(ref elems) => {
                e.begin_tuple()?;
                for elem in elems {
                    e.begin_tuple_elem()?;
                    elem.encode_to(e)?;
                }
                e.finish_tuple()
            }
            &Value::Struct(ref fields) => {
                e.begin_struct()?;
                for field in fields {
                    e.begin_struct_field(&field.name)?;
                    field.value.encode_to(e)?;
                }
                e.finish_struct()
            }
            &Value::Enum(EnumValue {
                variant_ord,
                ref variant_name,
                ref value,
            }) => {
                e.begin_enum(variant_ord, variant_name)?;
                value.encode_to(e)
            }
        }
    }

    pub fn decode_from<R: Read>(d: &mut Decoder<R>) -> Result<Self> {
        Ok(match d.need()? {
            &Schema::Scalar(scalar_type) =>
                Value::Scalar(ScalarValue::decode_from(d, scalar_type)?),
            &Schema::Str => Value::Str(d.decode_str()?),
            &Schema::Bytes => Value::Bytes(d.decode_bytes()?),
            &Schema::Unit => {
                d.decode_unit()?;
                Value::Unit
            }
            &Schema::Option(_) => {
                if d.begin_option()? {
                    let inner = Value::decode_from(d)?;
                    Value::Option(Some(Box::new(inner)))
                } else {
                    Value::Option(None)
                }
            }
            &Schema::Seq(SeqSchema {
                len: Some(len),
                inner: _,
            }) => {
                d.begin_fixed_len_seq(len)?;
                let mut elems = Vec::with_capacity(len);
                for _ in 0..len {
                    d.begin_seq_elem()?;
                    elems.push(Value::decode_from(d,)?);
                }
                d.finish_seq()?;
                Value::FixedLenSeq(elems)
            }
            &Schema::Seq(SeqSchema {
                len: None,
                inner: _,
            }) => {
                let len = d.begin_var_len_seq()?;
                let mut elems = Vec::with_capacity(len);
                for _ in 0..len {
                    d.begin_seq_elem()?;
                    elems.push(Value::decode_from(d)?);
                }
                d.finish_seq()?;
                Value::VarLenSeq(elems)
            }
            &Schema::Tuple(ref inner_schemas) => {
                d.begin_tuple()?;
                let mut elems = Vec::with_capacity(inner_schemas.len());
                for _ in 0..inner_schemas.len() {
                    d.begin_tuple_elem()?;
                    elems.push(Value::decode_from(d)?);
                }
                d.finish_tuple()?;
                Value::Tuple(elems)
            }
            &Schema::Struct(ref schema_fields) => {
                d.begin_struct()?;
                let mut fields = Vec::with_capacity(schema_fields.len());
                for schema_field in schema_fields {
                    d.begin_struct_field(&schema_field.name)?;
                    fields.push(StructValueField {
                        name: schema_field.name.clone(),
                        value: Value::decode_from(d)?,
                    });
                }
                d.finish_struct()?;
                Value::Struct(fields)
            }
            &Schema::Enum(ref variants) => {
                let variant_ord = d.begin_enum()?;
                // TODO: avoid panicking
                let variant = &variants[variant_ord];
                d.begin_enum_variant(&variant.name)?;
                let inner = Value::decode_from(d)?;
                Value::Enum(EnumValue {
                    variant_ord,
                    variant_name: variant.name.clone(),
                    value: Box::new(inner),
                })
            }
            &Schema::Recurse(_) => unreachable!(), 
        })
    }
}

impl ScalarValue {
    pub fn encode_to<W: Write>(self, e: &mut Encoder<W>) -> Result<()> {
        match self {
            ScalarValue::U8(n) => e.encode_u8(n),
            ScalarValue::U16(n) => e.encode_u16(n),
            ScalarValue::U32(n) => e.encode_u32(n),
            ScalarValue::U64(n) => e.encode_u64(n),
            ScalarValue::U128(n) => e.encode_u128(n),
            ScalarValue::I8(n) => e.encode_i8(n),
            ScalarValue::I16(n) => e.encode_i16(n),
            ScalarValue::I32(n) => e.encode_i32(n),
            ScalarValue::I64(n) => e.encode_i64(n),
            ScalarValue::I128(n) => e.encode_i128(n),
            ScalarValue::F32(n) => e.encode_f32(n),
            ScalarValue::F64(n) => e.encode_f64(n),
            ScalarValue::Char(c) => e.encode_char(c),
            ScalarValue::Bool(b) => e.encode_bool(b),
        }
    }

    pub fn decode_from<R: Read>(
        d: &mut Decoder<R>,
        scalar_type: ScalarType,
    ) -> Result<Self> {
        Ok(match scalar_type {
            ScalarType::U8 => ScalarValue::U8(d.decode_u8()?),
            ScalarType::U16 => ScalarValue::U16(d.decode_u16()?),
            ScalarType::U32 => ScalarValue::U32(d.decode_u32()?),
            ScalarType::U64 => ScalarValue::U64(d.decode_u64()?),
            ScalarType::U128 => ScalarValue::U128(d.decode_u128()?),
            ScalarType::I8 => ScalarValue::I8(d.decode_i8()?),
            ScalarType::I16 => ScalarValue::I16(d.decode_i16()?),
            ScalarType::I32 => ScalarValue::I32(d.decode_i32()?),
            ScalarType::I64 => ScalarValue::I64(d.decode_i64()?),
            ScalarType::I128 => ScalarValue::I128(d.decode_i128()?),
            ScalarType::F32 => ScalarValue::F32(d.decode_f32()?),
            ScalarType::F64 => ScalarValue::F64(d.decode_f64()?),
            ScalarType::Char => ScalarValue::Char(d.decode_char()?),
            ScalarType::Bool => ScalarValue::Bool(d.decode_bool()?),
        })
    }
}
