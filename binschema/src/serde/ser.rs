
use crate::{
    error::{
        Error,
        Result,
        error,
        bail,
    },
    schema::{
        Schema,
        SeqSchema,
    },
    Encoder,
};
use std::{
    io::Write,
    fmt::Display,
};
use serde::ser::{
    Serialize,
    Serializer,
    SerializeSeq,
    SerializeTuple,
    SerializeTupleStruct,
    SerializeTupleVariant,
    SerializeMap,
    SerializeStruct,
    SerializeStructVariant,
};

impl serde::ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::other(msg.to_string())
    }
}

impl<'a, 'b, W: Write> Encoder<'a, 'b, W> {
    fn serialize_seq_like<'c>(
        &'c mut self,
        got_len: Option<usize>,
    ) -> Result<SeqLikeSerializer<'a, 'b, 'c, W>>
    {
        let seq_like =
            match self.need()? {
                &Schema::Seq(SeqSchema { len: Some(len), .. }) => {
                    self.begin_fixed_len_seq(got_len.unwrap_or(len))?;
                    SeqLike::Seq
                },
                &Schema::Seq(SeqSchema { len: None, .. }) => {
                    let len = got_len
                        .ok_or_else(|| error!(
                            Other,
                            Some(self.coder_state()),
                            "serialize var len seq without specifying len",
                        ))?;
                    self.begin_var_len_seq(len)?;
                    SeqLike::Seq
                },
                &Schema::Tuple(_) => {
                    self.begin_tuple()?;
                    SeqLike::Tuple
                },
                &Schema::Unit => {
                    self.encode_unit()?;
                    SeqLike::Unit
                }
                schema => bail!(
                    SchemaNonConformance,
                    Some(self.coder_state()),
                    "need {:?}, got seq-like",
                    schema,
                ),
            };
        Ok(SeqLikeSerializer {
            encoder: self,
            seq_like,
        })
    }
}

macro_rules! leaf_methods {
    ($(
        $serialize:ident($type:ty), $encode:ident;
    )*)=>{$(
        fn $serialize(self, v: $type) -> Result<()> {
            self.$encode(v)
        }
    )*};
}

impl<'a, 'b, 'c, W: Write> Serializer for &'c mut Encoder<'a, 'b, W> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = SeqLikeSerializer<'a, 'b, 'c, W>;
    type SerializeTuple = SeqLikeSerializer<'a, 'b, 'c, W>;
    type SerializeTupleStruct = SeqLikeSerializer<'a, 'b, 'c, W>;
    type SerializeTupleVariant = SeqLikeSerializer<'a, 'b, 'c, W>;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    leaf_methods!(
        serialize_bool(bool), encode_bool;
        serialize_i8(i8), encode_i8;
        serialize_i16(i16), encode_i16;
        serialize_i32(i32), encode_i32;
        serialize_i64(i64), encode_i64;
        serialize_i128(i128), encode_i128;
        serialize_u8(u8), encode_u8;
        serialize_u16(u16), encode_u16;
        serialize_u32(u32), encode_u32;
        serialize_u64(u64), encode_u64;
        serialize_u128(u128), encode_u128;
        serialize_f32(f32), encode_f32;
        serialize_f64(f64), encode_f64;
        serialize_char(char), encode_char;
        serialize_str(&str), encode_str;
        serialize_bytes(&[u8]), encode_bytes;
    );

    fn serialize_none(self) -> Result<()> {
        self.encode_none()
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.begin_some()?;
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<()> {
        self.encode_unit()
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        self.encode_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        self.begin_enum(variant_index as usize, variant)?;
        self.encode_unit()
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.begin_enum(variant_index as usize, variant)?;
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        self.serialize_seq_like(len)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq_like(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_seq_like(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.begin_enum(variant_index as usize, variant)?;
        self.serialize_seq_like(Some(len))
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self> {
        let len = len
            .ok_or_else(|| Error::other("serialize_map with None len"))?;
        self.begin_var_len_seq(len)?;
        Ok(self)
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self> {
        self.begin_struct()?;
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self> {
        self.begin_enum(variant_index as usize, variant)?;
        self.begin_struct()?;
        Ok(self)
    }

    fn is_human_readable(&self) -> bool { false }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum SeqLike {
    Seq,
    Tuple,
    Unit,
}

pub struct SeqLikeSerializer<'a, 'b, 'c, W> {
    encoder: &'c mut Encoder<'a, 'b, W>,
    seq_like: SeqLike,
}

impl<'a, 'b, 'c, W: Write> SeqLikeSerializer<'a, 'b, 'c, W> {
    fn inner_serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        match self.seq_like {
            SeqLike::Seq => self.encoder.begin_seq_elem()?,
            SeqLike::Tuple => self.encoder.begin_tuple_elem()?,
            SeqLike::Unit => bail!(
                Other,
                Some(self.encoder.coder_state()),
                "serialize element to unit as seq-like",
            ),
        }
        value.serialize(&mut *self.encoder)
    }

    fn inner_end(self) -> Result<()> {
        match self.seq_like {
            SeqLike::Seq => self.encoder.finish_seq(),
            SeqLike::Tuple => self.encoder.finish_tuple(),
            SeqLike::Unit => Ok(()),
        }
    }
}

impl<'a, 'b, 'c, W: Write> SerializeSeq for SeqLikeSerializer<'a, 'b, 'c, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.inner_serialize_element(value)
    }

    fn end(self) -> Result<()> {
        self.inner_end()
    }
}

impl<'a, 'b, 'c, W: Write> SerializeTuple for SeqLikeSerializer<'a, 'b, 'c, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.inner_serialize_element(value)
    }

    fn end(self) -> Result<()> {
        self.inner_end()
    }
}

impl<'a, 'b, 'c, W: Write> SerializeTupleStruct for SeqLikeSerializer<'a, 'b, 'c, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.inner_serialize_element(value)
    }

    fn end(self) -> Result<()> {
        self.inner_end()
    }
}

impl<'a, 'b, 'c, W: Write> SerializeTupleVariant for SeqLikeSerializer<'a, 'b, 'c, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.inner_serialize_element(value)
    }

    fn end(self) -> Result<()> {
        self.inner_end()
    }
}

impl<'a, 'b, 'c, W: Write> SerializeMap for &'c mut Encoder<'a, 'b, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.begin_seq_elem()?;
        self.begin_tuple()?;
        self.begin_tuple_elem()?;
        key.serialize(&mut **self)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.begin_tuple_elem()?;
        value.serialize(&mut **self)?;
        self.finish_tuple()
    }

    fn end(self) -> Result<()> {
        self.finish_seq()
    }
}

impl<'a, 'b, 'c, W: Write> SerializeStruct for &'c mut Encoder<'a, 'b, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.begin_struct_field(key)?;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.finish_struct()
    }

    fn skip_field(&mut self, key: &'static str) -> Result<()> {
        self.begin_struct_field(key)?;
        self.encode_none()
    }
}

impl<'a, 'b, 'c, W: Write> SerializeStructVariant for &'c mut Encoder<'a, 'b, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.begin_struct_field(key)?;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.finish_struct()
    }

    fn skip_field(&mut self, key: &'static str) -> Result<()> {
        self.begin_struct_field(key)?;
        self.encode_none()
    }
}

