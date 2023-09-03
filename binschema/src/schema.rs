//! Data types for representing a schema, and the macro for constructing them
//! with syntactic sugar.

use crate::{
    error::*,
    Encoder,
    Decoder,
};
use std::{
    fmt::Write,
    io,
};


/// Description of how raw binary data encodes less tedious structures of
/// semantic primitives.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Schema {
    /// Some scalar data type.
    Scalar(ScalarType),
    /// Utf8 string.
    Str,
    /// Byte string.
    Bytes,
    /// Unit (0 bytes).
    Unit,
    /// Option (some or none).
    Option(Box<Schema>),
    /// Homogenous sequence. May be fixed or variable length.
    Seq(SeqSchema),
    /// Heterogenous fixed-length sequence.
    Tuple(Vec<Schema>),
    /// Sequence fields with names and ordinals.
    Struct(Vec<StructSchemaField>),
    /// Tagged union of variants with names and ordinals.
    Enum(Vec<EnumSchemaVariant>),
    /// Recurse type. This allows schema to be self-referential.
    ///
    /// Represents a reference to the type n layers above self in the schema
    /// tree. So for eg, a binary search tree could be represented as:
    ///
    /// ```
    /// use binschema::{Schema, ScalarType};
    ///
    /// Schema::Enum(vec![
    ///     ("Branch", Schema::Struct(vec![
    ///         ("left", Schema::Recurse(2)).into(),
    ///         ("right", Schema::Recurse(2)).into(),
    ///     ])).into(),
    ///     ("Leaf", Schema::Scalar(ScalarType::I32)).into(),
    /// ]);
    /// ```
    ///
    /// `Recurse(0)` would recurse to itself, but it is illegal, as attempting
    /// to resolve leads to an infinite loop.
    Recurse(usize),
}

struct ParentNode<'a> {
    line: usize,
    idx: usize,
    next: Option<&'a ParentNode<'a>>,
}

impl Schema {
    /// A magic number, chosen at random and committed into the source code, which
    /// is recommended to be included before encoding a schema itself somewhere,
    /// and which the developers of binschema should change whenever the schema schema
    /// changes. This allows schema validity checks to be future compatible for changes
    /// to the metaschema, as well as decreases the chance of an schema validity check
    /// passing for data which wasn't encoded with binschema at all.
    pub fn schema_schema_magic_bytes() -> [u8; 4] {
        [0xfe, 0x56, 0x6e, 0x71]
    }

    /// The schema for transcoding a schema itself.
    pub fn schema_schema() -> Schema {
        schema!(
            enum {
                Scalar(enum {
                    U8(unit),
                    U16(unit),
                    U32(unit),
                    U64(unit),
                    U128(unit),
                    I8(unit),
                    I16(unit),
                    I32(unit),
                    I64(unit),
                    I128(unit),
                    F32(unit),
                    F64(unit),
                    Char(unit),
                    Bool(unit),
                }),
                Str(unit),
                Bytes(unit),
                Unit(unit),
                Option(recurse(1)),
                Seq(struct {
                    (len: option(u64)),
                    (inner: recurse(2)),
                }),
                Tuple(seq(varlen)(recurse(2))),
                Struct(seq(varlen)(struct {
                    (name: str),
                    (inner: recurse(3)),
                })),
                Enum(seq(varlen)(struct {
                    (name: str),
                    (inner: recurse(3)),
                })),
                Recurse(u64),
            }
        )
    }

    /// Encode this schema itself according to the schema schema.
    pub fn encode_schema<W: io::Write>(&self, encoder: &mut Encoder<W>) -> Result<()> {
        match self {
            &Schema::Scalar(st) => {
                encoder.begin_enum(0, "Scalar")?;
                let (st_ord, st_name) = match st {
                    ScalarType::U8 => (0, "U8"),
                    ScalarType::U16 => (1, "U16"),
                    ScalarType::U32 => (2, "U32"),
                    ScalarType::U64 => (3, "U64"),
                    ScalarType::U128 => (4, "U128"),
                    ScalarType::I8 => (5, "I8"),
                    ScalarType::I16 => (6, "I16"),
                    ScalarType::I32 => (7, "I32"),
                    ScalarType::I64 => (8, "I64"),
                    ScalarType::I128 => (9, "I128"),
                    ScalarType::F32 => (10, "F32"),
                    ScalarType::F64 => (11, "F64"),
                    ScalarType::Char => (12, "Char"),
                    ScalarType::Bool => (13, "Bool"),
                };
                encoder.begin_enum(st_ord, st_name)?;
                encoder.encode_unit()
            }
            &Schema::Str => {
                encoder.begin_enum(1, "Str")?;
                encoder.encode_unit()
            }
            &Schema::Bytes => {
                encoder.begin_enum(2, "Bytes")?;
                encoder.encode_unit()
            }
            &Schema::Unit => {
                encoder.begin_enum(3, "Unit")?;
                encoder.encode_unit()
            }
            &Schema::Option(ref inner) => {
                encoder.begin_enum(4, "Option")?;
                inner.encode_schema(encoder)
            }
            &Schema::Seq(SeqSchema { len, ref inner }) => {
                encoder.begin_enum(5, "Seq")?;
                encoder.begin_struct()?;
                encoder.begin_struct_field("len")?;
                if let Some(len) = len {
                    encoder.begin_some()?;
                    encoder.encode_u64(len as u64)?;
                } else {
                    encoder.encode_none()?;
                }
                encoder.begin_struct_field("inner")?;
                inner.encode_schema(encoder)?;
                encoder.finish_struct()
            }
            &Schema::Tuple(ref inners) => {
                encoder.begin_enum(6, "Tuple")?;
                encoder.begin_var_len_seq(inners.len())?;
                for inner in inners {
                    encoder.begin_seq_elem()?;
                    inner.encode_schema(encoder)?;
                }
                encoder.finish_seq()
            }
            &Schema::Struct(ref fields) => {
                encoder.begin_enum(7, "Struct")?;
                encoder.begin_var_len_seq(fields.len())?;
                for field in fields {
                    encoder.begin_seq_elem()?;
                    encoder.begin_struct()?;
                    encoder.begin_struct_field("name")?;
                    encoder.encode_str(&field.name)?;
                    encoder.begin_struct_field("inner")?;
                    field.inner.encode_schema(encoder)?;
                    encoder.finish_struct()?;
                }
                encoder.finish_seq()
            }
            &Schema::Enum(ref variants) => {
                encoder.begin_enum(8, "Enum")?;
                encoder.begin_var_len_seq(variants.len())?;
                for variant in variants {
                    encoder.begin_seq_elem()?;
                    encoder.begin_struct()?;
                    encoder.begin_struct_field("name")?;
                    encoder.encode_str(&variant.name)?;
                    encoder.begin_struct_field("inner")?;
                    variant.inner.encode_schema(encoder)?;
                    encoder.finish_struct()?;
                }
                encoder.finish_seq()
            }
            &Schema::Recurse(n) => {
                encoder.begin_enum(9, "Recurse")?;
                encoder.encode_u64(n as u64)
            }
        }
    }

    /// Decode this schema itself according to the schema schema.
    pub fn decode_schema<R: io::Read>(decoder: &mut Decoder<R>) -> Result<Self> {

        fn decode_usize<R: io::Read>(decoder: &mut Decoder<R>) -> Result<usize> {
            decoder.decode_u64()
                .and_then(|n| usize::try_from(n)
                    .map_err(|e| Error::new(
                        ErrorKind::PlatformLimits,
                        e,
                        Some(decoder.coder_state()),
                    )))
        }

        Ok(match decoder.begin_enum()? {
            0 => {
                decoder.begin_enum_variant("Scalar")?;
                let (st, st_name) = match decoder.begin_enum()? {
                    0 => (ScalarType::U8, "U8"),
                    1 => (ScalarType::U16, "U16"),
                    2 => (ScalarType::U32, "U32"),
                    3 => (ScalarType::U64, "U64"),
                    4 => (ScalarType::U128, "U128"),
                    5 => (ScalarType::I8, "I8"),
                    6 => (ScalarType::I16, "I16"),
                    7 => (ScalarType::I32, "I32"),
                    8 => (ScalarType::I64, "I64"),
                    9 => (ScalarType::I128, "I128"),
                    10 => (ScalarType::F32, "F32"),
                    11 => (ScalarType::F64, "F64"),
                    12 => (ScalarType::Char, "Char"),
                    13 => (ScalarType::Bool, "Bool"),
                    _ => panic!(
                        "unexpected enum ordinal decoding schema itself \
                        (this indicates a basic usage error rather than merely bad data)"
                    ),
                };
                decoder.begin_enum_variant(st_name)?;
                decoder.decode_unit()?;
                Schema::Scalar(st)
            }
            1 => {
                decoder.begin_enum_variant("Str")?;
                decoder.decode_unit()?;
                Schema::Str
            }
            2 => {
                decoder.begin_enum_variant("Bytes")?;
                decoder.decode_unit()?;
                Schema::Unit
            }
            3 => {
                decoder.begin_enum_variant("Unit")?;
                decoder.decode_unit()?;
                Schema::Unit
            }
            4 => {
                decoder.begin_enum_variant("Option")?;
                let inner = Schema::decode_schema(decoder)?;
                Schema::Option(Box::new(inner))
            }
            5 => {
                decoder.begin_enum_variant("Seq")?;
                decoder.begin_struct()?;
                decoder.begin_struct_field("len")?;
                let len = if decoder.begin_option()? {
                    Some(decode_usize(decoder)?)
                } else {
                    None
                };
                decoder.begin_struct_field("inner")?;
                let inner = Schema::decode_schema(decoder)?;
                decoder.finish_struct()?;
                Schema::Seq(SeqSchema { len, inner: Box::new(inner) })
            }
            6 => {
                decoder.begin_enum_variant("Tuple")?;
                let mut fields = Vec::new();
                for _ in 0..decoder.begin_var_len_seq()? {
                    decoder.begin_seq_elem()?;
                    fields.push(Schema::decode_schema(decoder)?);
                }
                decoder.finish_seq()?;
                Schema::Tuple(fields)
            }
            7 => {
                decoder.begin_enum_variant("Struct")?;
                let mut fields = Vec::new();
                for _ in 0..decoder.begin_var_len_seq()? {
                    decoder.begin_seq_elem()?;
                    decoder.begin_struct()?;
                    fields.push(StructSchemaField {
                        name: {
                            decoder.begin_struct_field("name")?;
                            decoder.decode_str()?
                        },
                        inner: {
                            decoder.begin_struct_field("inner")?;
                            Schema::decode_schema(decoder)?
                        },
                    });
                    decoder.finish_struct()?;
                }
                decoder.finish_seq()?;
                Schema::Struct(fields)
            }
            8 => {
                decoder.begin_enum_variant("Enum")?;
                let mut variants = Vec::new();
                for _ in 0..decoder.begin_var_len_seq()? {
                    decoder.begin_seq_elem()?;
                    decoder.begin_struct()?;
                    variants.push(EnumSchemaVariant {
                        name: {
                            decoder.begin_struct_field("name")?;
                            decoder.decode_str()?
                        },
                        inner: {
                            decoder.begin_struct_field("inner")?;
                            Schema::decode_schema(decoder)?
                        },
                    });
                    decoder.finish_struct()?;
                }
                decoder.finish_seq()?;
                Schema::Enum(variants)
            }
            9 => {
                decoder.begin_enum_variant("Recurse")?;
                Schema::Recurse(decode_usize(decoder)?)
            }
            _ => panic!(
                "unexpected enum ordinal decoding schema itself \
                (this indicates a basic usage error rather than merely bad data)"
            ),
        })
    }

    pub(crate) fn non_recursive_display_str(&self) -> &'static str {
        match self {
            Schema::Scalar(st) => st.display_str(),
            Schema::Str => "str",
            Schema::Bytes => "bytes",
            Schema::Unit => "unit",
            Schema::Option(_) => "option(..)",
            Schema::Seq(_) => "seq(..)(..)",
            Schema::Tuple(_)=> "tuple {..}",
            Schema::Struct(_) => "struct {..}",
            Schema::Enum(_) => "enum {..}",
            Schema::Recurse(_) => "recurse(_)",
        }
    }

    fn inner_pretty_fmt(
        &self,
        lines: &mut Vec<String>,
        indent: u32,
        parents: Option<&ParentNode>,
    ) {
        let mut line = String::new();
        for _ in 0..indent {
            line.push_str("    ");
        }
        line.push_str("- ");
        match self {
            &Schema::Scalar(st) => {
                line.push_str(st.display_str());
                lines.push(line);
            }
            &Schema::Str => {
                line.push_str("str");
                lines.push(line);
            }
            &Schema::Bytes => {
                line.push_str("bytes");
                lines.push(line);
            }
            &Schema::Unit => {
                line.push_str("unit");
                lines.push(line);
            }
            &Schema::Option(ref inner) => {
                line.push_str("option:");
                let child_parents = ParentNode {
                    line: lines.len(),
                    idx: line.len(),
                    next: parents,
                };
                lines.push(line);
                inner.inner_pretty_fmt(lines, indent + 1, Some(&child_parents));
            }
            &Schema::Seq(SeqSchema { len, ref inner }) => {
                line.push_str("seq (");
                if let Some(fixed_len) = len {
                    write!(&mut line, "length = {}", fixed_len).unwrap();
                } else {
                    line.push_str("variable length");
                }
                line.push_str("):");
                let child_parents = ParentNode {
                    line: lines.len(),
                    idx: line.len(),
                    next: parents,
                };
                lines.push(line);
                inner.inner_pretty_fmt(lines, indent + 1, Some(&child_parents));
            }
            &Schema::Tuple(ref inners) => {
                line.push_str("tuple");
                let child_parents = ParentNode {
                    line: lines.len(),
                    idx: line.len(),
                    next: parents,
                };
                lines.push(line);
                for (i, inner) in inners.iter().enumerate() {
                    let mut line = String::new();
                    for _ in 0..indent {
                        line.push_str("    ");
                    }
                    line.push_str("  ");
                    write!(&mut line, "element {}:", i).unwrap();
                    lines.push(line);
                    inner.inner_pretty_fmt(lines, indent + 1, Some(&child_parents));
                }
            }
            &Schema::Struct(ref fields) => {
                line.push_str("struct");
                let child_parents = ParentNode {
                    line: lines.len(),
                    idx: line.len(),
                    next: parents,
                };
                lines.push(line);
                for (i, field) in fields.iter().enumerate() {
                    let mut line = String::new();
                    for _ in 0..indent {
                        line.push_str("    ");
                    }
                    line.push_str("  ");
                    write!(&mut line, "field {} (name = {:?}):", i, field.name).unwrap();
                    lines.push(line);
                    field.inner.inner_pretty_fmt(lines, indent + 1, Some(&child_parents));
                }
            }
            &Schema::Enum(ref variants) => {
                line.push_str("enum");
                let child_parents = ParentNode {
                    line: lines.len(),
                    idx: line.len(),
                    next: parents,
                };
                lines.push(line);
                for (i, variant) in variants.iter().enumerate() {
                    let mut line = String::new();
                    for _ in 0..indent {
                        line.push_str("    ");
                    }
                    line.push_str("  ");
                    write!(&mut line, "variant {} (name = {:?}):", i, variant.name).unwrap();
                    lines.push(line);
                    variant.inner.inner_pretty_fmt(lines, indent + 1, Some(&child_parents));
                }
            }
            &Schema::Recurse(level) => {
                write!(&mut line, "recurse (level = {})", level).unwrap();
                let start_line = lines.len();
                let start_idx = line.len() + 1;
                lines.push(line);
                if level == 0 {
                    return;
                }
                let mut curr = parents;
                for _ in 0..level - 1 {
                    if curr.is_none() {
                        let l = lines.len() - 1;
                        lines[l].push_str(" --> [beyond root]");
                        return;
                    } else {
                        curr = curr.unwrap().next;
                    }
                }
                if curr.is_none() {
                    let l = lines.len() - 1;
                    lines[l].push_str(" --> [beyond root]");
                    return;
                }
                let dst_line = curr.unwrap().line;
                let dst_idx = curr.unwrap().idx + 1;
                let mut bar_idx = (dst_line..=start_line)
                    .map(|l| lines[l].len())
                    .max()
                    .unwrap() + 1;
                bar_idx = usize::max(bar_idx, dst_idx + 2);
                bar_idx = usize::max(bar_idx, start_idx + 1);
                while lines[start_line].len() < start_idx {
                    lines[start_line].push_str(" ");
                }
                for i in start_idx..=bar_idx {
                    if i < bar_idx {
                        lines[start_line].push_str("-")
                    } else {
                        lines[start_line].push_str("/");
                    }
                }
                for line in dst_line + 1..=start_line - 1 {
                    while lines[line].len() < bar_idx {
                        lines[line].push_str(" ");
                    }
                    if lines[line].len() == bar_idx {
                        lines[line].push_str("|");
                    }
                }
                while lines[dst_line].len() < dst_idx {
                    lines[dst_line].push_str(" ");
                }
                for i in dst_idx..=bar_idx {
                    if i == lines[dst_line].len() {
                        if i == dst_idx {
                            lines[dst_line].push_str("<");
                        } else if i < bar_idx {
                            lines[dst_line].push_str("-");
                        } else {
                            lines[dst_line].push_str("\\");
                        }
                    }
                }
            }
        }
    }

    pub fn pretty_fmt(&self) -> String {
        let mut lines = Vec::new();
        self.inner_pretty_fmt(&mut lines, 0, None);
        lines.join("\n")
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ScalarType {
    /// Encoded as-is.
    U8,
    /// Encoded little-endian.
    U16,
    /// Encoded var len.
    U32,
    /// Encoded var len.
    U64,
    /// Encoded var len.
    U128,
    /// Encoded as-is.
    I8,
    /// Encoded little-endian.
    I16,
    /// Encoded var len.
    I32,
    /// Encoded var len.
    I64,
    /// Encoded var len.
    I128,
    /// Encoded little-endian.
    F32,
    /// Encoded little-endian.
    F64,
    Char,
    /// Encoded as 1 byte, 0 or 1.
    Bool,
}

impl ScalarType {
    fn display_str(self) -> &'static str {
        match self {
            ScalarType::U8 => "u8",
            ScalarType::U16 => "u16",
            ScalarType::U32 => "u32",
            ScalarType::U64 => "u64",
            ScalarType::U128 => "u128",
            ScalarType::I8 => "i8",
            ScalarType::I16 => "i16",
            ScalarType::I32 => "i32",
            ScalarType::I64 => "i64",
            ScalarType::I128 => "i128",
            ScalarType::F32 => "f32",
            ScalarType::F64 => "f64",
            ScalarType::Char => "char",
            ScalarType::Bool => "bool",
        }
    }
}

/// Value in `Schema::Seq`.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SeqSchema {
    pub len: Option<usize>,
    pub inner: Box<Schema>,
}

/// Item in `Schema::Struct`.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct StructSchemaField {
    pub name: String,
    pub inner: Schema,
}

impl<S: Into<String>> From<(S, Schema)> for StructSchemaField {
    fn from((name, inner): (S, Schema)) -> Self {
        StructSchemaField {
            name: name.into(),
            inner,
        }
    }
}

/// Item in `Schema::Enum`. 
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct EnumSchemaVariant {
    pub name: String,
    pub inner: Schema,
}

impl<S: Into<String>> From<(S, Schema)> for EnumSchemaVariant {
    fn from((name, inner): (S, Schema)) -> Self {
        EnumSchemaVariant {
            name: name.into(),
            inner,
        }
    }
}

#[macro_export]
macro_rules! schema {
    (u8)=>{ $crate::Schema::Scalar($crate::ScalarType::U8) };
    (u16)=>{ $crate::Schema::Scalar($crate::ScalarType::U16) };
    (u32)=>{ $crate::Schema::Scalar($crate::ScalarType::U32) };
    (u64)=>{ $crate::Schema::Scalar($crate::ScalarType::U64) };
    (u128)=>{ $crate::Schema::Scalar($crate::ScalarType::U128) };
    (i8)=>{ $crate::Schema::Scalar($crate::ScalarType::I8) };
    (i16)=>{ $crate::Schema::Scalar($crate::ScalarType::I16) };
    (i32)=>{ $crate::Schema::Scalar($crate::ScalarType::I32) };
    (i64)=>{ $crate::Schema::Scalar($crate::ScalarType::I64) };
    (i128)=>{ $crate::Schema::Scalar($crate::ScalarType::I128) };
    (f32)=>{ $crate::Schema::Scalar($crate::ScalarType::F32) };
    (f64)=>{ $crate::Schema::Scalar($crate::ScalarType::F64) };
    (char)=>{ $crate::Schema::Scalar($crate::ScalarType::Char) };
    (bool)=>{ $crate::Schema::Scalar($crate::ScalarType::Bool) };
    (str)=>{ $crate::Schema::Str };
    (bytes)=>{ $crate::Schema::Bytes };
    (unit)=>{ $crate::Schema::Unit };
    (option($($inner:tt)*))=>{ $crate::Schema::Option(::std::boxed::Box::new($crate::schema!($($inner)*))) };
    (seq(varlen)($($inner:tt)*))=>{ $crate::Schema::Seq($crate::SeqSchema { len: ::core::option::Option::None, inner: ::std::boxed::Box::new($crate::schema!($($inner)*)) }) };
    (seq($len:expr)($($inner:tt)*))=>{ $crate::Schema::Seq($crate::SeqSchema { len: ::core::option::Option::Some($len), inner: ::std::boxed::Box::new($crate::schema!($($inner)*)) }) };
    (tuple { $(($($item:tt)*)),*$(,)? })=>{ $crate::Schema::Tuple(::std::vec![$( $crate::schema!($($item)*), )*]) };
    (struct { $(($name:ident: $($field:tt)*)),*$(,)? })=>{ $crate::Schema::Struct(::std::vec![$( $crate::StructSchemaField { name: ::std::string::String::from(::core::stringify!($name)), inner: $crate::schema!($($field)*) }, )*]) };
    (enum { $($name:ident($($variant:tt)*)),*$(,)? })=>{ $crate::Schema::Enum(::std::vec![$( $crate::EnumSchemaVariant { name: ::std::string::String::from(::core::stringify!($name)), inner: $crate::schema!($($variant)*) }, )*]) };
    (recurse($n:expr))=>{ $crate::Schema::Recurse($n) };
    (%$schema:expr)=>{ $schema };
}

pub use schema;
