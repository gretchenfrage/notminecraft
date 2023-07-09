
use crate::{
    error::{
        Error,
        Result,
        error,
        bail,
        ensure,
    },
    schema::{
        Schema,
        SeqSchema,
        ScalarType,
        StructSchemaField,
        EnumSchemaVariant,
    },
    Decoder,
};
use std::{
    io::Read,
    fmt::Display,
};
use serde::de::{
    value::{
        StrDeserializer,
        UsizeDeserializer,
    },
    Deserializer,
    Visitor,
    SeqAccess,
    MapAccess,
    EnumAccess,
    VariantAccess,
    DeserializeSeed,
};

impl serde::de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::other(msg.to_string())
    }
}

impl<'a, 'b, R: Read> Decoder<'a, 'b, R> {
    fn deserialize_seq_like<'d, V: Visitor<'d>>(
        &mut self,
        v: V,
        got_len: Option<usize>,
    ) -> Result<V::Value> {
        let (len, seq_like) =
            match self.need()? {
                &Schema::Seq(SeqSchema { len: Some(len), .. }) => {
                    self.begin_fixed_len_seq(len)?;
                    (len, SeqLike::Seq)
                },
                &Schema::Seq(SeqSchema { len: None, .. }) => {
                    let len = self.begin_var_len_seq()?;
                    (len, SeqLike::Seq)
                },
                &Schema::Tuple(ref inner) => {
                    self.begin_tuple()?;
                    (inner.len(), SeqLike::Tuple)
                }
                &Schema::Unit => {
                    self.decode_unit()?;
                    (0, SeqLike::Unit)
                }
                schema => bail!(
                    SchemaNonConformance,
                    Some(self.coder_state()),
                    "need {:?}, got seq-like",
                    schema,
                ),
            };
        if let Some(got_len) = got_len {
            ensure!(
                len == got_len,
                SchemaNonConformance,
                Some(self.coder_state()),
                "need seq-like len {}, got seq-like len {}",
                len,
                got_len,
            );
        }
        v.visit_seq(SeqDecoder {
            decoder: self,
            remaining: len,
            seq_like,
        })
    }

    fn inner_deserialize_struct<'d, V, N>(
        &mut self,
        fields: &[N],
        v: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'d>,
        N: AsName,
    {
        self.begin_struct()?;
        v.visit_map(StructDecoder {
            decoder: self,
            remaining: fields,
        })
    }

    fn inner_deserialize_enum<'d, V, N>(
        &mut self,
        variants: &[N],
        v: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'d>,
        N: AsName,
    {
        let ord = self.begin_enum()?;
        let name = variants
            .get(ord)
            .ok_or_else(|| error!(
                SchemaNonConformance,
                Some(self.coder_state()),
                "decoded ord {}, but only {} variants provided to deserialize_enum",
                ord,
                variants.len(),
            ))?;
        self.begin_enum_variant(name.as_name())?;
        v.visit_enum(EnumDecoder {
            decoder: self,
            ord,
        })
    }
}

macro_rules! leaf_methods {
    ($(
        $deserialize:ident, $visit:ident, $decode:ident;
    )*)=>{$(
        fn $deserialize<V: Visitor<'d>>(self, v: V) -> Result<V::Value> {
            v.$visit(self.$decode()?)
        }
    )*};
}

impl<'a, 'b, 'c, 'd, R: Read> Deserializer<'d> for &'c mut Decoder<'a, 'b, R> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'d>>(self, v: V) -> Result<V::Value> {
        match self.need()? {
            &Schema::Scalar(st) => match st {
                ScalarType::U8 => self.deserialize_u8(v),
                ScalarType::U16 => self.deserialize_u16(v),
                ScalarType::U32 => self.deserialize_u32(v),
                ScalarType::U64 => self.deserialize_u64(v),
                ScalarType::U128 => self.deserialize_u128(v),
                ScalarType::I8 => self.deserialize_i8(v),
                ScalarType::I16 => self.deserialize_i16(v),
                ScalarType::I32 => self.deserialize_i32(v),
                ScalarType::I64 => self.deserialize_i64(v),
                ScalarType::I128 => self.deserialize_i128(v),
                ScalarType::F32 => self.deserialize_f32(v),
                ScalarType::F64 => self.deserialize_f64(v),
                ScalarType::Char => self.deserialize_char(v),
                ScalarType::Bool => self.deserialize_bool(v),
            }
            &Schema::Str => self.deserialize_str(v),
            &Schema::Bytes => self.deserialize_bytes(v),
            &Schema::Unit => self.deserialize_unit(v),
            &Schema::Option(_) => self.deserialize_option(v),
            &Schema::Seq(_) => self.deserialize_seq_like(v, None),
            &Schema::Tuple(_) => self.deserialize_seq_like(v, None),
            &Schema::Struct(
                ref fields,
            ) => self.inner_deserialize_struct(fields, v),
            &Schema::Enum(
                ref variants,
            ) => self.inner_deserialize_enum(variants, v),
            &Schema::Recurse(_) => unreachable!(),
        }
    }

    leaf_methods!(
        deserialize_bool, visit_bool, decode_bool;
        deserialize_i8, visit_i8, decode_i8;
        deserialize_i16, visit_i16, decode_i16;
        deserialize_i32, visit_i32, decode_i32;
        deserialize_i64, visit_i64, decode_i64;
        deserialize_i128, visit_i128, decode_i128;
        deserialize_u8, visit_u8, decode_u8;
        deserialize_u16, visit_u16, decode_u16;
        deserialize_u32, visit_u32, decode_u32;
        deserialize_u64, visit_u64, decode_u64;
        deserialize_u128, visit_u128, decode_u128;
        deserialize_f32, visit_f32, decode_f32;
        deserialize_f64, visit_f64, decode_f64;
        deserialize_char, visit_char, decode_char;
        deserialize_str, visit_string, decode_str;
        deserialize_string, visit_string, decode_str;
        deserialize_bytes, visit_byte_buf, decode_bytes;
        deserialize_byte_buf, visit_byte_buf, decode_bytes;
    );

    fn deserialize_option<V: Visitor<'d>>(self, v: V) -> Result<V::Value> {
        match self.begin_option()? {
            false => v.visit_none(),
            true => v.visit_some(self)
        }
    }

    fn deserialize_unit<V: Visitor<'d>>(self, v: V) -> Result<V::Value> {
        self.decode_unit()?;
        v.visit_unit()
    }

    fn deserialize_unit_struct<V: Visitor<'d>>(
        self,
        _name: &'static str,
        v: V,
    ) -> Result<V::Value> {
        self.decode_unit()?;
        v.visit_unit()
    }

    fn deserialize_newtype_struct<V: Visitor<'d>>(
        self,
        _name: &'static str,
        v: V,
    ) -> Result<V::Value> {
        v.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'d>>(self, v: V) -> Result<V::Value> {
        self.deserialize_seq_like(v, None)
    }

    fn deserialize_tuple<V: Visitor<'d>>(
        self,
        got_len: usize,
        v: V,
    ) -> Result<V::Value> {
        self.deserialize_seq_like(v, Some(got_len))
    }

    fn deserialize_tuple_struct<V: Visitor<'d>>(
        self,
        _name: &'static str,
        got_len: usize,
        v: V,
    ) -> Result<V::Value> {
        self.deserialize_seq_like(v, Some(got_len))
    }

    fn deserialize_map<V: Visitor<'d>>(self, v: V) -> Result<V::Value> {
        let len = self.begin_var_len_seq()?;
        v.visit_map(MapDecoder {
            decoder: self,
            remaining: len,
        })
    }

    fn deserialize_struct<V: Visitor<'d>>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        v: V,
    ) -> Result<V::Value> {
        self.inner_deserialize_struct(fields, v)
    }

    fn deserialize_enum<V: Visitor<'d>>(
        self,
        _name: &'static str,
        variants: &'static [&'static str],
        v: V,
    ) -> Result<V::Value> {
        self.inner_deserialize_enum(variants, v)
    }

    fn deserialize_identifier<V: Visitor<'d>>(self, _v: V) -> Result<V::Value> {
        Err(error!(
            ApiUsage,
            Some(self.coder_state()),
            "deserialize_identifier directly on Decoder",
        ))
    }

    fn deserialize_ignored_any<V: Visitor<'d>>(self, v: V) -> Result<V::Value> {
        self.deserialize_any(v)
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum SeqLike {
    Seq,
    Tuple,
    Unit,
}

struct SeqDecoder<'a, 'b, 'c, R> {
    decoder: &'c mut Decoder<'a, 'b, R>,
    remaining: usize,
    seq_like: SeqLike,
}

impl<'a, 'b, 'c, 'd, R: Read> SeqAccess<'d> for SeqDecoder<'a, 'b, 'c, R> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'d>,
    {
        if self.remaining > 0 {
            self.remaining -= 1;

            match self.seq_like {
                SeqLike::Seq => self.decoder.begin_seq_elem()?,
                SeqLike::Tuple => self.decoder.begin_tuple_elem()?,
                SeqLike::Unit => bail!(
                    Other,
                    Some(self.decoder.coder_state()),
                    "deserialize element from unit as seq-like",
                ),
            };
            let value = seed.deserialize(&mut *self.decoder)?;

            if self.remaining == 0 {
                match self.seq_like {
                    SeqLike::Seq => self.decoder.finish_seq()?,
                    SeqLike::Tuple => self.decoder.finish_tuple()?,
                    SeqLike::Unit => unreachable!(),
                }
            }

            Ok(Some(value))
        } else {
            Ok(None)
        }
    }
}

struct MapDecoder<'a, 'b, 'c, R> {
    decoder: &'c mut Decoder<'a, 'b, R>,
    remaining: usize,
}

impl<'a, 'b, 'c, 'd, R: Read> MapAccess<'d> for MapDecoder<'a, 'b, 'c, R> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'d>,
    {
        if self.remaining > 0 {
            self.remaining -= 1;

            self.decoder.begin_seq_elem()?;
            self.decoder.begin_tuple()?;
            self.decoder.begin_tuple_elem()?;
            Ok(Some(seed.deserialize(&mut *self.decoder)?))
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'d>,
    {
        self.decoder.begin_tuple_elem()?;
        let value = seed.deserialize(&mut *self.decoder)?;
        self.decoder.finish_tuple()?;

        if self.remaining == 0 {
            self.decoder.finish_seq()?;
        }

        Ok(value)
    }
}

trait AsName {
    fn as_name(&self) -> &str;
}

impl AsName for &'static str {
    fn as_name(&self) -> &str { *self }
}

impl AsName for StructSchemaField {
    fn as_name(&self) -> &str { &self.name }
}

impl AsName for EnumSchemaVariant {
    fn as_name(&self) -> &str { &self.name }
}

struct StructDecoder<'a, 'b, 'c, 'n, R, N> {
    decoder: &'c mut Decoder<'a, 'b, R>,
    remaining: &'n [N],
}

impl<
    'a, 'b, 'c, 'n, 'd,
    R: Read,
    N: AsName,
> MapAccess<'d> for StructDecoder<'a, 'b, 'c, 'n, R, N>
{
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'d>,
    {
        if let Some(next) = self.remaining.get(0) {
            self.remaining = &self.remaining[1..];

            self.decoder.begin_struct_field(next.as_name())?;
            Ok(Some(seed.deserialize(
                StrDeserializer::<Error>::new(next.as_name())
            )?))
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'d>,
    {
        let value = seed.deserialize(&mut *self.decoder)?;

        if self.remaining.is_empty() {
            self.decoder.finish_struct()?;
        }

        Ok(value)
    }
}

struct EnumDecoder<'a, 'b, 'c, R> {
    ord: usize,
    decoder: &'c mut Decoder<'a, 'b, R>,
}

impl<'a, 'b, 'c, 'd, R: Read> EnumAccess<'d> for EnumDecoder<'a, 'b, 'c, R> {
    type Error = Error;
    type Variant = &'c mut Decoder<'a, 'b, R>;

    fn variant_seed<V: DeserializeSeed<'d>>(
        self,
        seed: V,
    ) -> Result<(V::Value, Self::Variant)>
    {
        Ok((
            seed.deserialize(UsizeDeserializer::<Error>::new(self.ord))?,
            self.decoder,
        ))
    }
}

impl<'a, 'b, 'c, 'd, R: Read> VariantAccess<'d> for &'c mut Decoder<'a, 'b, R> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        self.decode_unit()
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'d>,
    {
        seed.deserialize(self)
    }

    fn tuple_variant<V: Visitor<'d>>(
        self,
        len: usize,
        visitor: V,
    ) -> Result<V::Value> {
        self.deserialize_tuple(len, visitor)
    }

    fn struct_variant<V: Visitor<'d>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        self.inner_deserialize_struct(fields, visitor)
    }
}
