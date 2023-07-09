//! Data types for representing a schema, and the macro for constructing them
//! with syntactic sugar.

use crate::serde_to_writer;
use serde::{
    Serialize,
    Deserialize,
};
use sha2::{
    Digest,
    Sha256,
};
use std::fmt::Write;


/// Description of how raw binary data encodes less tedious structures of
/// semantic primitives.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
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
    
    pub fn sha256(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        serde_to_writer(self, &mut hasher).unwrap();
        hasher.finalize().into()
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct SeqSchema {
    pub len: Option<usize>,
    pub inner: Box<Schema>,
}

/// Item in `Schema::Struct`.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
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
