//! Machine for validating API usage and schema conformance which is used in
//! both encoding and decoding. 


use crate::{
    do_if_err::DoIfErr,
    error::{
        Result,
        error,
        ensure,
        bail,
    },
    schema::{
        Schema,
        SeqSchema,
        schema,
    },
    coder::coder_alloc::CoderStateAlloc,
};
use std::{
    write,
    writeln,
    io::Write,
    fmt::{self, Formatter, Debug},
};


/// Used to construct an (en/de)coder, and ensures that some schema is being
/// validly (en/de)coded.
pub struct CoderState<'a> {
    stack: Vec<StackFrame<'a>>,
    broken: bool,
    dbg_log: Option<DbgLog<'a>>,
}

struct DbgLog<'a> {
    write: &'a mut (dyn Write + 'a),
    indent: usize,
}

macro_rules! dbg_log {
    ($self:ident, $($t:tt)*)=>{
        if let Some(ref mut dbg_log) = $self.dbg_log {
            for _ in 0..dbg_log.indent {
                let _ = write!(dbg_log.write, "  ")
                    .map_err(|e| eprintln!("IO error in coder dbg log: {}", e));
            }
            let _ = write!(dbg_log.write, "<")
                    .map_err(|e| eprintln!("IO error in coder dbg log: {}", e));
            let _ = write!(dbg_log.write, $($t)*)
                .map_err(|e| eprintln!("IO error in coder dbg log: {}", e));
            let _ = writeln!(dbg_log.write, "/>")
                .map_err(|e| eprintln!("IO error in coder dbg log: {}", e));
        }
    };
}

macro_rules! dbg_log_push {
    ($self:ident, $($t:tt)*)=>{
        if let Some(ref mut dbg_log) = $self.dbg_log {
            for _ in 0..dbg_log.indent {
                let _ = write!(dbg_log.write, "  ")
                    .map_err(|e| eprintln!("IO error in coder dbg log: {}", e));
            }
            let _ = write!(dbg_log.write, "<")
                    .map_err(|e| eprintln!("IO error in coder dbg log: {}", e));
            let _ = write!(dbg_log.write, $($t)*)
                .map_err(|e| eprintln!("IO error in coder dbg log: {}", e));
            let _ = writeln!(dbg_log.write, ">")
                .map_err(|e| eprintln!("IO error in coder dbg log: {}", e));
            dbg_log.indent += 1;
        }
    };
}

macro_rules! dbg_log_pop {
    ($self:ident, $($t:tt)*)=>{
        if let Some(ref mut dbg_log) = $self.dbg_log {
            for _ in 0..dbg_log.indent {
                let _ = write!(dbg_log.write, "  ")
                    .map_err(|e| eprintln!("IO error in coder dbg log: {}", e));
            }
            let _ = write!(dbg_log.write, "</")
                    .map_err(|e| eprintln!("IO error in coder dbg log: {}", e));
            let _ = write!(dbg_log.write, $($t)*)
                .map_err(|e| eprintln!("IO error in coder dbg log: {}", e));
            let _ = writeln!(dbg_log.write, ">")
                .map_err(|e| eprintln!("IO error in coder dbg log: {}", e));
            dbg_log.indent -= 1;
        }
    };
}

#[derive(Debug, Clone)]
pub(super) struct StackFrame<'a> {
    schema: &'a Schema,
    api_state: ApiState,
}

#[derive(Debug, Clone)]
enum ApiState {
    /// Starting state. This element needs to be coded, and has not started
    /// being coded.
    Need,
    /// Some inner element is being coded, and finishing encoding the inner
    /// element is sufficient for this element to be considered finished
    /// encoding.
    AutoFinish,
    /// An option is being coded, but its someness is uninitialized.
    OptionUninitSomeness,
    /// A sequence is being coded, but its length is uninitialized.
    SeqUninitLen,
    /// A sequence is being coded. The corresponding `schema` must be a
    /// `Schema::Seq`.
    Seq {
        len: usize,
        /// Next element index would code.
        next: usize,
    },
    /// A tuple is being coded. The corresponding `schema` must be a
    /// `Schema::Tuple`.
    Tuple {
        /// Next element index would code.
        next: usize,
    },
    /// A struct is being coded. The corresponding `schema` must be a
    /// `schema::Struct`.
    Struct {
        /// Next field index would code.
        next: usize,
    },
    /// An enum is being coded. The corresponding `schema` must be a 
    /// `schema::Enum`.
    Enum {
        /// If None, neither the variant ord or name have been coded. If Some,
        /// that variant ord has been coded, but the variant name has not.
        variant_ord: Option<usize>,
    }
}

impl<'a> Debug for CoderState<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_fmt(format_args!("CoderState {{\n"))?;
        f.write_fmt(format_args!("    broken: {},\n", self.broken))?;
        f.write_fmt(format_args!("    stack:\n"))?;
        for (i, frame) in self.stack.iter().rev().enumerate() {
            let i = format!("{:02}", i);
            f.write_fmt(format_args!(
                "    {}. schema: {}\n",
                i, frame.schema.non_recursive_display_str(),
            ))?;
            f.write_str("    ")?;
            for _ in 0..i.len() + 2 {
                f.write_str(" ")?;
            }
            f.write_fmt(format_args!("state: {:?}\n", frame.api_state))?;
        }
        f.write_fmt(format_args!("}}"))?;
        Ok(())
    }
}


impl<'a> CoderState<'a> {
    pub fn new(
        schema: &'a Schema,
        alloc: CoderStateAlloc,
        dbg_log: Option<&'a mut (dyn Write + 'a)>,
    ) -> Self {
        let mut stack = alloc.into_stack();
        stack.push(StackFrame {
            schema,
            api_state: ApiState::Need,
        });
        CoderState {
            stack,
            broken: false,
            dbg_log: dbg_log.map(|write| DbgLog { write, indent: 0 }),
        }
    }

    pub fn is_finished(&self) -> bool {
        self.stack.is_empty() && !self.broken
    }

    pub fn is_finished_or_err(&self) -> Result<()> {
        if self.is_finished() {
            Ok(())
        } else {
            Err(error!(
                ApiUsage,
                Some(self),
                "didn't finish coding, broken = {}",
                self.broken,
            ))
        }
    }

    pub fn into_alloc(mut self) -> CoderStateAlloc {
        self.stack.clear();
        CoderStateAlloc::from_stack(self.stack)
    }
}

macro_rules! validate_top {
    ($self:ident, |$top:ident| $opt_ret:expr, $got:expr)=>{{
        ensure!(
            !$self.broken,
            ApiUsage,
            Some($self),
            "usage after IO error",
        );
        match $self.stack.iter_mut().rev().next() {
            None => bail!(ApiUsage, Some($self), "usage of finished coder"),
            Some($top) => match $opt_ret {
                Some(ret) => ret,
                None => match &$top.api_state {
                    &ApiState::AutoFinish => unreachable!("{:#?}", $self.stack),
                    &ApiState::OptionUninitSomeness => unreachable!(),
                    &ApiState::SeqUninitLen => unreachable!(),
                    &ApiState::Need => bail!(
                        SchemaNonConformance,
                        Some($self),
                        "\nneed: {:#?}\ngot: {:#?}",
                        $top.schema,
                        $got,
                    ),
                    &ApiState::Seq { .. } => bail!(
                        ApiUsage,
                        Some($self),
                        "need seq elem/finish, got {}",
                        $got,
                    ),
                    &ApiState::Tuple { .. } => bail!(
                        ApiUsage,
                        Some($self),
                        "need tuple elem/finish, got {}",
                        $got,
                    ),
                    &ApiState::Struct { .. } => bail!(
                        ApiUsage,
                        Some($self),
                        "need struct field/finish, got {}",
                        $got,
                    ),
                    &ApiState::Enum { variant_ord: None } => bail!(
                        ApiUsage,
                        Some($self),
                        "need enum variant ord, got {}",
                        $got,
                    ),
                    &ApiState::Enum { variant_ord: Some(_) } => bail!(
                        ApiUsage,
                        Some($self),
                        "need enum variant name, got {}",
                        $got,
                    ),
                },
            }
        }
    }};
}

macro_rules! validate_top_matches {
    ($self:ident, $top:pat => $ret:expr, $need:expr)=>{
        validate_top!(
            $self,
            |top| match top {
                $top => Some($ret),
                _ => None,
            },
            $need
        )
    };
}

macro_rules! validate_need_eq {
    ($self:ident, $got:expr)=>{
        validate_top!(
            $self,
            |s| if
                    matches!(&s.api_state, &ApiState::Need)
                    && s.schema == &$got
                {
                    Some(())
                } else {
                    None
                },
            format_args!("code {:?}", $got)
        )
    };
}

macro_rules! validate_need_matches {
    ($self:ident, $pat:pat => $ret:expr, $got:expr)=>{
        validate_top_matches!(
            $self,
            &mut StackFrame {
                schema: $pat,
                api_state: ApiState::Need,
            } => $ret,
            $got
        )
    };
}

macro_rules! match_or_unreachable {
    ($expr:expr, $pat:pat => $ret:expr)=>{
        match $expr {
            $pat => $ret,
            _ => unreachable!(),
        }
    };
}

macro_rules! code_simple {
    ($($m:ident($($t:tt)*),)*)=>{$(
        pub(crate) fn $m(&mut self) -> Result<()> {
            validate_need_eq!(self, schema!($($t)*));
            dbg_log!(self, "{:?}", schema!($($t)*));
            self.pop();
            Ok(())
        }
    )*};
}

impl<'a> CoderState<'a> {
    /// Unwrap top stack frame.
    fn top(&mut self) -> &mut StackFrame<'a> {
        let i = self.stack.len() - 1;
        &mut self.stack[i]
    }

    /// Push a stack frame for needing the schema. If the schema is recurse,
    /// resolve it first.
    fn push_need(&mut self, mut schema: &'a Schema) -> Result<()> {
        let mut i = self.stack.len();
        while let &Schema::Recurse(n) = schema {
            if n == 0 {
                self.broken = true;
                bail!(IllegalSchema, Some(self), "recurse of level 0");
            }
            i = i
                .checked_sub(n)
                .ok_or_else(|| error!(
                    IllegalSchema,
                    Some(self),
                    "recurse past base of stack",
                ))
                .do_if_err(|| self.broken = true)?;
            schema = self.stack[i].schema;
        }
        self.stack.push(StackFrame {
            schema,
            api_state: ApiState::Need,
        });
        Ok(())
    }

    /// Pop stack frame. If this uncovers auto finish frames, pop those too.
    fn pop(&mut self) {
        self.stack.pop().unwrap();
        while matches!(
            self.stack.iter().rev().next(),
            Some(&StackFrame { api_state: ApiState::AutoFinish, .. })
        ) {
            dbg_log_pop!(self, "auto finish");
            self.stack.pop().unwrap();
        }
    }

    /// Get the schema that needs to be coded. Fails if has already began
    /// coding that schema. Will never return `Schema::Recurse`--rather,
    /// will return the schema that recursion resolved to, or fail if it
    /// couldn't resolve.
    pub(crate) fn need(&self) -> Result<&'a Schema> {
        match self.stack.iter().rev().next() {
            Some(&StackFrame {
                schema,
                api_state: ApiState::Need,
            }) => Ok(schema),
            _ => Err(error!(
                ApiUsage, Some(self), ".need() call while not in need state"
            ))
        }
    }

    /// Mark the coder as having experienced an irrecoverable error. Any
    /// further attempts at coding is an API usage error.
    pub(crate) fn mark_broken(&mut self) {
        self.broken = true;
    }

    code_simple!(
        code_u8(u8),
        code_u16(u16),
        code_u32(u32),
        code_u64(u64),
        code_u128(u128),
        code_i8(i8),
        code_i16(i16),
        code_i32(i32),
        code_i64(i64),
        code_i128(i128),
        code_f32(f32),
        code_f64(f64),
        code_char(char),
        code_bool(bool),
        code_unit(unit),
        code_str(str),
        code_bytes(bytes),
    );

    /// Begin coding an option. If successful, this must be immediately
    /// followed with `set_option_none` or `set_option_some`, or unspecified
    /// behavior occurs. If following with `set_option_none`, that immediately
    /// finishes the option. If following with `set_option_some`, that must
    /// in turn be followed with coding the inner value, which then
    /// auto-finishes the option.
    pub(crate) fn begin_option(&mut self) -> Result<()> {
        validate_need_matches!(
            self,
            &Schema::Option(_) => (),
            "option begin"
        );
        self.top().api_state = ApiState::OptionUninitSomeness;
        Ok(())
    }

    /// Make the option be encoded as none. This must be immediately following
    /// a successful call to `begin_option`, or unspecified behavior occurs.
    /// See `begin_option`.
    pub(crate) fn set_option_none(&mut self) {
        debug_assert!(matches!(
            self.stack.iter().rev().next(),
            Some(&StackFrame {
                api_state: ApiState::OptionUninitSomeness,
                ..
            }),
        ));
        dbg_log!(self, "none");
        self.pop();
    }

    /// Make the option be encoded as some. This must be immediately following
    /// a successful call to `begin_option`, or unspecified behavior occurs.
    /// See `begin_option`.
    pub(crate) fn set_option_some(&mut self) -> Result<()> {
        debug_assert!(matches!(
            self.stack.iter().rev().next(),
            Some(&StackFrame {
                api_state: ApiState::OptionUninitSomeness,
                ..
            }),
        ));
        let inner =
            match_or_unreachable!(
                self.top(),
                &mut StackFrame {
                    schema: &Schema::Option(ref inner),
                    ..
                } => inner
            );
        self.top().api_state = ApiState::AutoFinish;
        dbg_log_push!(self, "some");
        self.push_need(inner)?;
        Ok(())
    }

    /// Begin coding a fixed len seq. This should be followed by coding `len`
    /// elements with `begin_seq_elem`, then by `finish_seq`.
    pub(crate) fn begin_fixed_len_seq(&mut self, len: usize) -> Result<()> {
        let fixed_len =
            validate_need_matches!(
                self,
                &Schema::Seq(SeqSchema {
                    len: Some(fixed_len),
                    ..
                }) => fixed_len,
                "fixed len seq begin"
            );
        ensure!(
            fixed_len == len,
            SchemaNonConformance,
            Some(self),
            "need seq len {}, got seq len {}",
            fixed_len,
            len
        );
        dbg_log_push!(self, "seq, fixed len={}", len);
        self.top().api_state =
            ApiState::Seq {
                len,
                next: 0,
            };
        Ok(())
    }

    /// Begin coding a var len seq. If successful, this must be immediately
    /// followed with `set_var_len_seq_len`, or unspecified behavior occurs.
    /// Then, that many elements should be coded with `begin_seq_elem`, then
    /// `finish_seq`.
    pub(crate) fn begin_var_len_seq(&mut self) -> Result<()> {
        validate_need_matches!(
            self,
            &Schema::Seq(SeqSchema {
                len: None,
                ..
            }) => (),
            "var len seq begin"
        );
        self.top().api_state = ApiState::SeqUninitLen;
        Ok(())
    }

    /// Provide the length of a var len seq. See `begin_var_len_seq`. This must
    /// immediately follow a successful call to `begin_var_len_seq`, or
    /// unspecified behavior occurs. See `begin_var_len_seq`.
    pub(crate) fn set_var_len_seq_len(&mut self, len: usize) {
        debug_assert!(matches!(
            self.stack.iter().rev().next(),
            Some(&StackFrame {
                api_state: ApiState::SeqUninitLen,
                ..
            }),
        ));
        dbg_log_push!(self, "seq, var len={}", len);
        self.top().api_state =
            ApiState::Seq {
                len,
                next: 0,
            };
    }

    /// Begin encoding an element in a seq. This should be followed by encoding
    /// the inner value. See `begin_seq`,
    pub(crate) fn begin_seq_elem(&mut self) -> Result<()> {
        let (schema, len, next) =
            validate_top_matches!(
                self,
                &mut StackFrame {
                    schema,
                    api_state: ApiState::Seq {
                        len,
                        ref mut next,
                    },
                } => (schema, len, next),
                "seq elem"
            );
        ensure!(
            *next < len,
            ApiUsage,
            Some(self),
            "begin seq elem at idx {}, but that is seq's declared len",
            *next
        );
        *next += 1;
        self
            .push_need(match_or_unreachable!(
                schema,
                &Schema::Seq(SeqSchema { ref inner, .. }) => &**inner
            ))?;
        Ok(())
    }

    /// Finish encoding a seq. See `begin_seq`.
    pub(crate) fn finish_seq(&mut self) -> Result<()> {
        let (len, next) =
            validate_top_matches!(
                self,
                &mut StackFrame {
                    api_state: ApiState::Seq {
                        len,
                        next,
                    },
                    ..
                } => (len, next),
                "seq finish"
            );
        debug_assert!(len <= next);
        ensure!(
            len == next,
            ApiUsage,
            Some(self),
            "finish seq of declared len {}, but only coded {} elems",
            len,
            next
        );
        dbg_log_pop!(self, "seq");
        self.pop();
        Ok(())
    }

    /// Begin coding a tuple. This should be followed by coding the
    /// elements with `begin_tuple_elem` followed by a call to `finish_tuple`.
    pub(crate) fn begin_tuple(&mut self) -> Result<()> {
        validate_need_matches!(
            self,
            &Schema::Tuple(_) => (),
            "tuple begin"
        );
        dbg_log_push!(self, "tuple");
        self.top().api_state =
            ApiState::Tuple {
                next: 0,
            };
        Ok(())
    }

    /// Begin coding an element in a tuple. This should be followed by
    /// coding the inner value. See `begin_tuple`,
    pub(crate) fn begin_tuple_elem(&mut self) -> Result<()> {
        let (schema, next) =
            validate_top_matches!(
                self,
                &mut StackFrame {
                    schema,
                    api_state: ApiState::Tuple {
                        ref mut next,
                    },
                } => (schema, next),
                "tuple elem"
            );
        let inner_schema =
            match_or_unreachable!(
                schema,
                &Schema::Tuple(ref inners) => inners
            )
            .get(*next);
        let inner_schema =
            match inner_schema {
                Some(inner_schema) => inner_schema,
                None => bail!(
                    SchemaNonConformance,
                    Some(self),
                    "begin tuple elem at idx {}, but that is the tuple's len",
                    *next,
                ),
            };
        *next += 1;
        self.push_need(inner_schema)?;
        Ok(())
    }

    /// Finish coding a tuple. See `begin_tuple`.
    pub(crate) fn finish_tuple(&mut self) -> Result<()> {
        let (schema, next) =
            validate_top_matches!(
                self,
                &mut StackFrame {
                    schema,
                    api_state: ApiState::Tuple {
                        next,
                    },
                } => (schema, next),
                "tuple finish"
            );
        let inners = 
            match_or_unreachable!(
                schema,
                &Schema::Tuple(ref inners) => inners
            );
        ensure!(
            inners.len() == next,
            SchemaNonConformance,
            Some(self),
            "finish tuple of len {}, but only encoded {} elems",
            inners.len(),
            next,
        );
        dbg_log_pop!(self, "tuple");
        self.pop();
        Ok(())
    }

    /// Begin coding a struct. This should be followed by coding the
    /// fields with `begin_struct_field` followed by a call to `finish_struct`.
    pub(crate) fn begin_struct(&mut self) -> Result<()> {
        validate_need_matches!(
            self,
            &Schema::Struct(_) => (),
            "struct begin"
        );
        dbg_log_push!(self, "struct");
        self.top().api_state =
            ApiState::Struct {
                next: 0,
            };
        Ok(())
    }

    /// Begin coding a field in a struct. This should be followed by
    /// coding the inner value. See `begin_struct`,
    pub(crate) fn begin_struct_field(&mut self, name: &str) -> Result<()> {
        let (schema, next) =
            validate_top_matches!(
                self,
                &mut StackFrame {
                    schema,
                    api_state: ApiState::Struct {
                        ref mut next,
                    },
                } => (schema, next),
                "struct field"
            );
        let field =
            match_or_unreachable!(
                schema,
                &Schema::Struct(ref fields) => fields
            )
            .get(*next);
        let field =
            match field {
                Some(field) => field,
                None => bail!(
                    SchemaNonConformance,
                    Some(self),
                    "begin struct field at idx {}, but that is the struct's len",
                    *next,
                ),
            };
        ensure!(
            &field.name == name,
            SchemaNonConformance,
            Some(self),
            "need struct field {:?}, got struct field {:?}",
            field.name,
            name,
        );
        *next += 1;
        dbg_log!(self, "begin struct field {:?}", name);
        self.push_need(&field.inner)?;
        Ok(())
    }

    /// Finish coding a struct. See `begin_struct`.
    pub(crate) fn finish_struct(&mut self) -> Result<()> {
        let (schema, next) =
            validate_top_matches!(
                self,
                &mut StackFrame {
                    schema,
                    api_state: ApiState::Struct {
                        next,
                    },
                } => (schema, next),
                "struct finish"
            );
        let fields = 
            match_or_unreachable!(
                schema,
                &Schema::Struct(ref fields) => fields
            );
        ensure!(
            fields.len() == next,
            SchemaNonConformance,
            Some(self),
            "finish struct of len {}, but only coded {} elems",
            fields.len(),
            next,
        );
        dbg_log_pop!(self, "struct");
        self.pop();
        Ok(())
    }

    /// Begin coding an enum. Returns the number of variants.
    /// 
    /// This should be followed by:
    ///
    /// - `begin_enum_variant_ord`
    /// - `begin_enum_variant_name`
    /// - encoding the inner value
    ///
    /// Which then auto-finishes the enum. At any point after successfully
    /// calling this method and before successfully calling
    /// `begin_enum_variant_name` one may call `cancel_enum` to restore the
    /// state preceeding the initial call to `begin_enum`.
    pub(crate) fn begin_enum(&mut self) -> Result<usize> {
        let num_variants =
            validate_need_matches!(
                self,
                &Schema::Enum(ref variants) => variants.len(),
                "code enum"
            );
        dbg_log_push!(self, "enum");
        self.top().api_state = ApiState::Enum { variant_ord: None };
        Ok(num_variants)
    }

    /// Provide the variant ordinal of an enum. See `begin_enum`.
    pub(crate) fn begin_enum_variant_ord(
        &mut self,
        variant_ord: usize,
    ) -> Result<()> {
        let schema =
            validate_top_matches!(
                self,
                &mut StackFrame {
                    schema,
                    api_state: ApiState::Enum { variant_ord: None },
                } => schema,
                "enum variant ord"
            );
        let num_variants =
            match_or_unreachable!(
                schema,
                &Schema::Enum(ref variants) => variants.len()
            );
        // TODO: when decoding, this is a malformed data error, not a schema
        //       non-conformance error
        ensure!(
            variant_ord < num_variants,
            SchemaNonConformance,
            Some(self),
            "begin enum with variant ordinal {}, but enum only has {} variants",
            variant_ord,
            num_variants
        );
        dbg_log!(self, "variant ord = {}", variant_ord);
        self.top().api_state =
            ApiState::Enum { variant_ord: Some(variant_ord) };
        Ok(())
    }

    /// Provide the variant name of an enum. See `begin_enum`.
    pub(crate) fn begin_enum_variant_name(
        &mut self,
        variant_name: &str,
    ) -> Result<()> {
        let (schema, variant_ord) =
            validate_top_matches!(
                self,
                &mut StackFrame {
                    schema,
                    api_state: ApiState::Enum {
                        variant_ord: Some(variant_ord),
                    },
                } => (schema, variant_ord),
                "enum variant name"
            );
        let variants =
            match_or_unreachable!(
                schema,
                &Schema::Enum(ref variants) => variants
            );
        let need_variant_name = &variants[variant_ord].name;
        ensure!(
            variant_name == need_variant_name,
            SchemaNonConformance,
            Some(self),
            "begin enum with variant name {:?}, but variant at that ordinal has name {:?}",
            variant_name,
            need_variant_name,
        );
        dbg_log!(self, "variant name = {:?}", variant_name);
        self.top().api_state = ApiState::AutoFinish;
        self.push_need(&variants[variant_ord].inner)?;
        Ok(())
    }

    /// Cancel an enum. Must only be called after a successful call to
    /// `begin_enum` and before a successful call to
    /// `begin_enum_variant_name`, or unspecified behavior occurs.
    /// See `begin_enum`.
    pub(crate) fn cancel_enum(&mut self) {
        debug_assert!(matches!(
            self.stack.iter().rev().next(),
            Some(&StackFrame {
                api_state: ApiState::Enum { .. },
                ..
            }),
        ));
        self.top().api_state = ApiState::Need;
    }
}
