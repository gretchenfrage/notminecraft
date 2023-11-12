
use crate::{
    do_if_err::DoIfErr,
    error::{
        Result,
        Error,
        error,
        bail,
    },
    coder::coder::CoderState,
    var_len::{
        read_var_len_uint,
        read_var_len_sint,
        read_ord,
    },
    schema::Schema,
};
use std::{
    mem::{
        size_of,
        take,
    },
    io::Read,
    borrow::BorrowMut,
    iter::repeat,
};


/// Decodes a value from a `std::io::Read` comforming to a schema.
pub struct Decoder<'a, 'b, R> {
    state: &'b mut CoderState<'a>,
    read: &'b mut R,
}

impl<'a, 'b, R> Decoder<'a, 'b, R> {
    pub fn new(state: &'b mut CoderState<'a>, read: &'b mut R) -> Self {
        Decoder { state, read }
    }

    /// Get the schema that needs to be decoded. Fails if has already began
    /// decoding that schema. Will never return `Schema::Recurse`--rather,
    /// will return the schema that recursion resolved to, or fail if it
    /// couldn't resolve.
    ///
    /// This is usually not necessary to do. It's mainly for debugging.
    pub fn need(&self) -> Result<&'a Schema> {
        self.state.need()
    }

    pub fn coder_state(&self) -> &CoderState<'a> {
        &*self.state
    }
}

macro_rules! decode_le_bytes {
    ($($m:ident($t:ident) $c:ident,)*)=>{$(
        pub fn $m(&mut self) -> Result<$t> {
            self.state.$c()?;
            let buf = self
                .read([0; size_of::<$t>()])
                .do_if_err(|| self.state.mark_broken())?;
            Ok($t::from_le_bytes(buf))
        }
    )*};
}

macro_rules! decode_var_len_uint {
    ($($m:ident($t:ident) $c:ident,)*)=>{$(
        pub fn $m(&mut self) -> Result<$t> {
            self.state.$c()?;
            read_var_len_uint(&mut self.read)
                .map_err(Error::from)
                .and_then(|n| $t::try_from(n)
                    .map_err(|_| error!(
                        MalformedData,
                        Some(self.coder_state()),
                        concat!(
                            "{} out of range for a ",
                            stringify!($t),
                        ),
                        n,
                    )))
                .do_if_err(|| self.state.mark_broken())
        }
    )*};
}

macro_rules! decode_var_len_sint {
    ($($m:ident($t:ident) $c:ident,)*)=>{$(
        pub fn $m(&mut self) -> Result<$t> {
            self.state.$c()?;
            read_var_len_sint(&mut self.read)
                .map_err(Error::from)
                .and_then(|n| $t::try_from(n)
                    .map_err(|_| error!(
                        MalformedData,
                        Some(self.coder_state()),
                        concat!(
                            "{} out of range for a ",
                            stringify!($t),
                        ),
                        n,
                    )))
                .do_if_err(|| self.state.mark_broken())
        }
    )*};
}

impl<'a, 'b, R: Read> Decoder<'a, 'b, R> {
    fn read<B: BorrowMut<[u8]>>(&mut self, mut buf: B) -> Result<B> {
        self.read
            .read_exact(buf.borrow_mut())
            .do_if_err(|| self.state.mark_broken())?;
        Ok(buf)
    }

    /// Read a varlen-encoded usize.
    fn read_len(&mut self) -> Result<usize> {
        read_var_len_uint(&mut self.read)
            .map_err(Error::from)
            .and_then(|n| usize::try_from(n)
                .map_err(|_| error!(
                    PlatformLimits,
                    Some(self.coder_state()),
                    "{} out of range for a usize",
                    n,
                )))
            .do_if_err(|| self.state.mark_broken())
    }

    decode_le_bytes!(
        decode_u8(u8) code_u8,
        decode_u16(u16) code_u16,
        decode_i8(i8) code_i8,
        decode_i16(i16) code_i16,
        decode_f32(f32) code_f32,
        decode_f64(f64) code_f64,
    );

    decode_var_len_uint!(
        decode_u32(u32) code_u32,
        decode_u64(u64) code_u64,
        decode_u128(u128) code_u128,
    );

    decode_var_len_sint!(
        decode_i32(i32) code_i32,
        decode_i64(i64) code_i64,
        decode_i128(i128) code_i128,
    );

    pub fn decode_char(&mut self) -> Result<char> {
        self.state.code_char()?;
        let n = read_var_len_uint(&mut self.read)
            .map_err(Error::from)
            .and_then(|n| u32::try_from(n)
                .map_err(|_| error!(
                    MalformedData,
                    Some(self.coder_state()),
                    "{} out of range for a char",
                    n,
                )))
            .do_if_err(|| self.state.mark_broken())?;
        char::from_u32(n)
            .ok_or_else(|| error!(
                MalformedData,
                Some(self.coder_state()),
                "{} is not a valid char",
                n
            ))
    }

    pub fn decode_bool(&mut self) -> Result<bool> {
        self.state.code_bool()?;
        let [n] = self.read([0])?;
        match n {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(error!(
                MalformedData,
                Some(self.coder_state()),
                "{} is not a valid bool",
                n,
            )),
        }
    }

    pub fn decode_unit(&mut self) -> Result<()> {
        self.state.code_unit()?;
        Ok(())
    }

    /// Clear `buf` and decode a str into it.
    pub fn decode_str_into(&mut self, buf: &mut String) -> Result<()> {
        // always clear the buf, for consistency
        buf.clear();

        self.state.code_str()?;
        let len = self.read_len()?;

        // do a little switcharoo to get ownership of raw Vec<u8> buf
        //
        // this is fine because String::default() won't make any allocs unless
        // characters are actually added to it.
        let mut bbuf = take(buf).into_bytes();

        // TODO: protection against malicious payloads

        // try to read all the bytes in
        // on error, make sure to return the buffer
        bbuf.reserve(len);
        bbuf.extend(repeat(0).take(len));
        if let Err(e) = self.read.read_exact(&mut bbuf) {
            self.state.mark_broken();
            bbuf.clear();
            *buf = String::from_utf8(bbuf).unwrap();
            return Err(e.into());
        }

        // try to convert to utf8
        // on error, make sure to return the buffer
        match String::from_utf8(bbuf) {
            Ok(s) => {
                *buf = s;
                Ok(())
            }
            Err(e) => {
                self.state.mark_broken();
                let mut bbuf = e.into_bytes();
                bbuf.clear();
                *buf = String::from_utf8(bbuf).unwrap();
                Err(error!(
                    MalformedData, Some(self.coder_state()), "non UTF8 str bytes",
                ))
            }
        }
    }

    /// Decode a str into a new alloc.
    pub fn decode_str(&mut self) -> Result<String> {
        let mut buf = String::new();
        self.decode_str_into(&mut buf)?;
        Ok(buf)
    }

    /// Clear `buf` and decode a bytes into it.
    pub fn decode_bytes_into(&mut self, buf: &mut Vec<u8>) -> Result<()> {
        // always clear the buf, for consistency
        buf.clear();

        self.state.code_bytes()?;
        let len = self.read_len().do_if_err(|| self.state.mark_broken())?;
        buf.reserve(len);
        buf.extend(repeat(0).take(len));
        self.read.read_exact(buf).do_if_err(|| self.state.mark_broken())?;
        Ok(())
    }

    /// Decode a bytes into a new alloc.
    pub fn decode_bytes(&mut self) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        self.decode_bytes_into(&mut buf)?;
        Ok(buf)
    }
    
    /// Begin decoding an option. If returns false, option is none, and finishes
    /// decoding immediately. If returns true, option is some, in which case
    /// this should be followed by decoding the inner value, when then
    /// auto-finishes the option.
    pub fn begin_option(&mut self) -> Result<bool> {
        self.state.begin_option()?;
        let [n] = self.read([0])?;
        let is_some =
            match n {
                0 => false,
                1 => true,
                _ => bail!(
                    MalformedData,
                    Some(self.coder_state()),
                    "{} is not a valid option someness",
                    n,
                ),
            };
        if is_some {
            self.state.set_option_some()?;
        } else {
            self.state.set_option_none();
        }
        Ok(is_some)
    }

    /// Begin decoding a fixed len seq. This should be followed by decoding
    /// `len` elements with `begin_seq_elem` followed by a call to
    /// `finish_seq`.
    pub fn begin_fixed_len_seq(&mut self, len: usize) -> Result<()> {
        self.state.begin_fixed_len_seq(len)?;
        Ok(())
    }

    /// Begin decoding a var len seq. Returns the length. This should be
    /// followed by decoding `len` elements with `begin_seq_elem` followed by
    /// a call to `finish_seq`.
    pub fn begin_var_len_seq(&mut self) -> Result<usize> {
        self.state.begin_var_len_seq()?;
        let len = self.read_len()?;
        self.state.set_var_len_seq_len(len);
        Ok(len)
    }

    /// Begin decoding an element in a seq. This should be followed by decoding
    /// the inner value. See `begin_fixed_len_seq` or `begin_var_len_seq`.
    pub fn begin_seq_elem(&mut self) -> Result<()> {
        self.state.begin_seq_elem()?;
        Ok(())
    }

    /// Finish decoding a seq. See `begin_fixed_len_seq` or
    /// `begin_var_len_seq`.
    pub fn finish_seq(&mut self) -> Result<()> {
        self.state.finish_seq()?;
        Ok(())
    }
    
    /// Begin decoding a tuple. This should be followed by decoding the
    /// elements with `begin_tuple_elem` followed by a call to `finish_tuple`.
    pub fn begin_tuple(&mut self) -> Result<()> {
        self.state.begin_tuple()?;
        Ok(())
    }

    /// Begin decoding an element in a tuple. This should be followed by
    /// decoding the inner value. See `begin_tuple`,
    pub fn begin_tuple_elem(&mut self) -> Result<()> {
        self.state.begin_tuple_elem()?;
        Ok(())
    }

    /// Finish decoding a tuple. See `begin_tuple`.
    pub fn finish_tuple(&mut self) -> Result<()> {
        self.state.finish_tuple()?;
        Ok(())
    }

    /// Begin decoding a struct. This should be followed by decoding the
    /// fields with `begin_struct_field` followed by a call to `finish_struct`.
    pub fn begin_struct(&mut self) -> Result<()> {
        self.state.begin_struct()?;
        Ok(())
    }

    /// Begin decoding a field in a struct. This should be followed by
    /// decoding the inner value. See `begin_struct`,
    pub fn begin_struct_field(&mut self, name: &str) -> Result<()> {
        self.state.begin_struct_field(name)?;
        Ok(())
    }

    /// Finish decoding a struct. See `begin_struct`.
    pub fn finish_struct(&mut self) -> Result<()> {
        self.state.finish_struct()?;
        Ok(())
    }

    /// Begin decoding an enum. Returns the variant ordinal. This should be
    /// followed by `begin_enum_variant`, then decoding the inner value, which
    /// then auto-finishes the enum.
    pub fn begin_enum(&mut self) -> Result<usize> {
        let num_variants = self.state.begin_enum()?;
        let variant_ord = read_ord(&mut self.read, num_variants)
            .do_if_err(|| self.state.mark_broken())?;
        self.state
            .begin_enum_variant_ord(variant_ord)
            .do_if_err(|| self.state.mark_broken())?;
        Ok(variant_ord)
    }

    /// Provide the name of the enum variant. See `begin_enum`.
    pub fn begin_enum_variant(&mut self, name: &str) -> Result<()> {
        self.state.begin_enum_variant_name(name)?;
        Ok(())
    }
}
