//! Handling of variable-length integer logic.


use std::io::{
    Read,
    Write,
    Result,
    Error,
    ErrorKind,
};


macro_rules! ensure {
    ($c:expr, $($e:tt)*)=>{
        if !$c {
            return Err(Error::new(
                ErrorKind::Other,
                format!($($e)*),
            ));
        }
    };
}


/// Number of bytes needed to encode an ordinal based on max ordinal
/// value.
fn ord_byte_len(max_ord: usize) -> usize {
    let mut mask = !0;
    let mut bytes = 0;

    while (mask & max_ord) != 0 {
        mask <<= 8;
        bytes += 1;
    }

    bytes
}

/// Write an enum ordinal. Assumes `ord` < `num_variants`.
pub fn write_ord<W>(
    write: &mut W,
    ord: usize,
    num_variants: usize,
) -> Result<()>
where
    W: Write,
{
    debug_assert!(ord < num_variants, "enum ord out of bounds");
    // if the ord is greater than 2^64... congratulations, future man, on
    // having several dozen exabytes of RAM. you get a free bug.
    let all_bytes = u64::to_le_bytes(ord as _);
    let byte_len = ord_byte_len(num_variants - 1);
    let used_bytes = &all_bytes[..byte_len];
    write.write_all(used_bytes)
}

/// Read an enum ordinal.
pub fn read_ord<R>(
    read: &mut R,
    num_variants: usize,
) -> Result<usize>
where
    R: Read,
{
    ensure!(num_variants > 0, "malformed data, presence of uninhabited enum");
    let mut all_bytes = [0; 8];
    let byte_len = ord_byte_len(num_variants - 1);
    let used_bytes = &mut all_bytes[..byte_len];
    read.read_exact(used_bytes)?;
    let ord = u64::from_le_bytes(all_bytes);
    ensure!(
        ord < num_variants as u64,
        "malformed data, enum ordinal {} out of range 0..{}",
        ord,
        num_variants,
    );
    // we know ord to be a valid usize, because it is less than num_variants,
    // which we get by getting the length of the enum vector, thus making it
    // valid usize
    Ok(ord as usize)
}

const MORE_BIT: u8  = 0b10000000;
const LO_7_BITS: u8 = 0b01111111;

/// Write a variable length unsigned int.
pub fn write_var_len_uint<W>(
    write: &mut W,
    mut n: u128,
) -> Result<()>
where
    W: Write,
{
    let mut more = true;
    while more {
        let curr_7_bits = (n & (LO_7_BITS as u128)) as u8;
        n >>= 7;
        more = n != 0;
        let curr_byte = ((more as u8) << 7) | curr_7_bits;
        write.write_all(&[curr_byte])?;
    }
    Ok(())
}

/// Read a variable length unsigned int.
pub fn read_var_len_uint<R>(
    read: &mut R,
) -> Result<u128>
where
    R: Read,
{
    let mut n: u128 = 0;
    let mut shift = 0;
    let mut more = true;
    while more {
        ensure!(
            shift < 128,
            "malformed data: too many bytes in var len uint",
        );

        let mut buf = [0];
        read.read_exact(&mut buf)?;
        let [curr_byte] = buf;

        n |= ((curr_byte & LO_7_BITS) as u128) << shift;
        shift += 7;
        more = (curr_byte & MORE_BIT) != 0;
    }
    Ok(n)
}


const ENCODED_SIGN_BIT: u8 = 0b01000000;
const LO_6_BITS: u8        = 0b00111111;

/// Write a variable length signed int.
pub fn write_var_len_sint<W>(
    write: &mut W,
    mut n: i128,
) -> Result<()>
where
    W: Write,
{
    let neg = n < 0;
    if neg {
        n = !n;
    }
    let curr_7_bits =
        ((neg as u8) << 6)
        | (n & (LO_6_BITS as i128)) as u8;
    n >>= 6;
    let mut more = n != 0;
    let curr_byte = ((more as u8) << 7) | curr_7_bits;
    write.write_all(&[curr_byte])?;

    while more {
        let curr_7_bits = (n & (LO_7_BITS as i128)) as u8;
        n >>= 7;
        more = n != 0;
        let curr_byte = ((more as u8) << 7) | curr_7_bits;
        write.write_all(&[curr_byte])?;
    }

    Ok(())
}

/// Read a variable length signed int.
pub fn read_var_len_sint<R>(
    read: &mut R,
) -> Result<i128>
where
    R: Read,
{
    let mut n: i128 = 0;
    
    let mut buf = [0];
    read.read_exact(&mut buf)?;
    let [curr_byte] = buf;

    let neg = (curr_byte & ENCODED_SIGN_BIT) != 0;
    n |= (curr_byte & LO_6_BITS) as i128;
    let mut more = (curr_byte & MORE_BIT) != 0;
    let mut shift = 6;

    while more {
        // TODO: should use crate-specific error types
        ensure!(
            shift < 128,
            "malformed data: too many bytes in var len sint",
        );

        let mut buf = [0];
        read.read_exact(&mut buf)?;
        let [curr_byte] = buf;

        n |= ((curr_byte & LO_7_BITS) as i128) << shift;
        shift += 7;
        more = (curr_byte & MORE_BIT) != 0;
    }

    if neg {
        n = !n;
    }
    
    Ok(n)
}

#[test]
fn test_var_len_uint() {
    let mut buf = Vec::new();
    for n in 0..2 << 10 {
        buf.clear();
        write_var_len_uint(&mut buf, n).unwrap();
        let n2 = read_var_len_uint(&mut buf.as_slice()).unwrap();
        assert_eq!(n, n2);
        //println!("{} encoded in {} bytes", n, buf.len());
    }
}

#[test]
fn test_var_len_pos_sint() {
    let mut buf = Vec::new();
    for n in 0..2 << 10 {
        buf.clear();
        write_var_len_sint(&mut buf, n).unwrap();
        let n2 = read_var_len_sint(&mut buf.as_slice()).unwrap();
        assert_eq!(n, n2);
        //println!("{} encoded in {} bytes", n, buf.len());
    }
}

#[test]
fn test_var_len_neg_sint() {
    let mut buf = Vec::new();
    for n in 0..2 << 10 {
        let n = -n;
        buf.clear();
        write_var_len_sint(&mut buf, n).unwrap();
        let n2 = read_var_len_sint(&mut buf.as_slice()).unwrap();
        assert_eq!(n, n2);
        //println!("{} encoded in {} bytes", n, buf.len());
    }
}
