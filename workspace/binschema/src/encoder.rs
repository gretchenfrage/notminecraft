
use crate::{
    error::{
        Error,
        Result,
    },
    do_if_err::DoIfErr,
    coder::coder::CoderState,
    var_len::{
        write_var_len_uint,
        write_var_len_sint,
        write_ord,
    },
    schema::Schema,
};
use std::io::Write;


/// Encodes a value to a `std::io::Write` comforming to a schema.
pub struct Encoder<'a, 'b, W> {
    state: &'b mut CoderState<'a>,
    write: &'b mut W,
}

impl<'a, 'b, W> Encoder<'a, 'b, W> {
    pub fn new(state: &'b mut CoderState<'a>, write: &'b mut W) -> Self {
        Encoder { state, write }
    }

    /// Get the schema that needs to be encoded. Fails if has already began
    /// encoding that schema. Will never return `Schema::Recurse`--rather,
    /// will return the schema that recursion resolved to, or fail if it
    /// couldn't resolve.
    ///
    /// This is usually not necessary to use, and usage may be seen as a
    /// weakening of defensive programming.
    pub fn need(&self) -> Result<&'a Schema> {
        self.state.need()
    }

    pub fn coder_state(&self) -> &CoderState<'a> {
        &*self.state
    }
}

macro_rules! encode_le_bytes {
    ($($m:ident($t:ident) $c:ident,)*)=>{$(
        pub fn $m(&mut self, n: $t) -> Result<()> {
            self.state.$c()?;
            self.write(&n.to_le_bytes())?;
            Ok(())
        }
    )*};
}

macro_rules! encode_var_len_uint {
    ($($m:ident($t:ident) $c:ident,)*)=>{$(
        pub fn $m(&mut self, n: $t) -> Result<()> {
            self.state.$c()?;
            write_var_len_uint(&mut self.write, n as u128)
                .do_if_err(|| self.state.mark_broken())?;
            Ok(())
        }
    )*};
}

macro_rules! encode_var_len_sint {
    ($($m:ident($t:ident) $c:ident,)*)=>{$(
        pub fn $m(&mut self, n: $t) -> Result<()> {
            self.state.$c()?;
            write_var_len_sint(&mut self.write, n as i128)
                .do_if_err(|| self.state.mark_broken())?;
            Ok(())
        }
    )*};
}

impl<'a, 'b, W: Write> Encoder<'a, 'b, W> {
    fn write(&mut self, b: &[u8]) -> Result<()> {
        self.write
            .write_all(b)
            .map_err(Error::from)
            .do_if_err(|| self.state.mark_broken())
    }

    encode_le_bytes!(
        encode_u8(u8) code_u8,
        encode_u16(u16) code_u16,
        encode_i8(i8) code_i8,
        encode_i16(i16) code_i16,
        encode_f32(f32) code_f32,
        encode_f64(f64) code_f64,
    );

    encode_var_len_uint!(
        encode_u32(u32) code_u32,
        encode_u64(u64) code_u64,
        encode_u128(u128) code_u128,
        encode_char(char) code_char,
    );

    encode_var_len_sint!(
        encode_i32(i32) code_i32,
        encode_i64(i64) code_i64,
        encode_i128(i128) code_i128,
    );

    pub fn encode_bool(&mut self, b: bool) -> Result<()> {
        self.state.code_bool()?;
        self.write(&[b as u8])?;
        Ok(())
    }

    pub fn encode_unit(&mut self) -> Result<()> {
        self.state.code_unit()?;
        Ok(())
    }

    pub fn encode_str(&mut self, s: &str) -> Result<()> {
        self.state.code_str()?;
        write_var_len_uint(&mut self.write, s.len() as u128)
            .do_if_err(|| self.state.mark_broken())?;
        self.write(s.as_bytes())?;
        Ok(())
    }

    pub fn encode_bytes(&mut self, s: &[u8]) -> Result<()> {
        self.state.code_bytes()?;
        write_var_len_uint(&mut self.write, s.len() as u128)
            .do_if_err(|| self.state.mark_broken())?;
        self.write(s)?;
        Ok(())
    }

    /// Completely encode an option none value.
    pub fn encode_none(&mut self) -> Result<()> {
        self.state.begin_option()?;
        self.state.set_option_none();
        self.write(&[0])?;
        Ok(())
    }

    /// Begin encoding an option some value. This should be followed by
    /// encoding the inner value, which then auto-finishes the option.
    pub fn begin_some(&mut self) -> Result<()> {
        self.state.begin_option()?;
        self.state.set_option_some()?;
        self.write(&[1])?;
        Ok(())
    }

    /// Begin encoding a fixed len seq. This should be followed by encoding
    /// `len` elements with `begin_seq_elem` followed by a call to
    /// `finish_seq`.
    pub fn begin_fixed_len_seq(&mut self, len: usize) -> Result<()> {
        self.state.begin_fixed_len_seq(len)?;
        Ok(())
    }

    /// Begin encoding a var len seq. This should be followed by encoding `len`
    /// elements with `begin_seq_elem` followed by a call to `finish_seq`.
    pub fn begin_var_len_seq(&mut self, len: usize) -> Result<()> {
        self.state.begin_var_len_seq()?;
        self.state.set_var_len_seq_len(len);
        write_var_len_uint(&mut self.write, len as u128)
            .do_if_err(|| self.state.mark_broken())?;
        Ok(())
    }
    
    /// Begin encoding an element in a seq. This should be followed by encoding
    /// the inner value. See `begin_seq`.
    pub fn begin_seq_elem(&mut self) -> Result<()> {
        self.state.begin_seq_elem()?;
        Ok(())
    }

    /// Finish encoding a seq. See `begin_seq`.
    pub fn finish_seq(&mut self) -> Result<()> {
        self.state.finish_seq()?;
        Ok(())
    }

    /// Begin encoding a tuple. This should be followed by encoding the
    /// elements with `begin_tuple_elem` followed by a call to `finish_tuple`.
    pub fn begin_tuple(&mut self) -> Result<()> {
        self.state.begin_tuple()?;
        Ok(())
    }

    /// Begin encoding an element in a tuple. This should be followed by
    /// encoding the inner value. See `begin_tuple`,
    pub fn begin_tuple_elem(&mut self) -> Result<()> {
        self.state.begin_tuple_elem()?;
        Ok(())
    }

    /// Finish encoding a tuple. See `begin_tuple`.
    pub fn finish_tuple(&mut self) -> Result<()> {
        self.state.finish_tuple()?;
        Ok(())
    }

    /// Begin encoding a struct. This should be followed by encoding the
    /// fields with `begin_struct_field` followed by a call to `finish_struct`.
    pub fn begin_struct(&mut self) -> Result<()> {
        self.state.begin_struct()?;
        Ok(())
    }

    /// Begin encoding a field in a struct. This should be followed by
    /// encoding the inner value. See `begin_struct`,
    pub fn begin_struct_field(&mut self, name: &str) -> Result<()> {
        self.state.begin_struct_field(name)?;
        Ok(())
    }

    /// Finish encoding a struct. See `begin_struct`.
    pub fn finish_struct(&mut self) -> Result<()> {
        self.state.finish_struct()?;
        Ok(())
    }

    /// Begin encoding an enum. This should be followed by encoding the inner
    /// value, which then auto-finishes the enum.
    pub fn begin_enum(
        &mut self,
        variant_ord: usize,
        variant_name: &str,
    ) -> Result<()> {
        // as a single all-or-nothing state change (via cancel if non-immediate
        // fail):
        //
        // - begin enum
        // - begin enum variant ord
        // - begin enum variant name
        let num_variants = self.state.begin_enum()?;
        self.state
            .begin_enum_variant_ord(variant_ord)
            .do_if_err(|| self.state.cancel_enum())?;
        self.state
            .begin_enum_variant_name(variant_name)
            .do_if_err(|| self.state.cancel_enum())?;

        write_ord(&mut self.write, variant_ord, num_variants)
            .do_if_err(|| self.state.mark_broken())?;
        Ok(())
    }
}