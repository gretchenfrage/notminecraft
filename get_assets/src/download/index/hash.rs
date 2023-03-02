
use std::{
    borrow::Borrow,
    fmt::{self, Formatter, Display},
};
use anyhow::{
    Result,
    ensure,
};


/// Pre-validated hash string.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct HashStr<S>(S);

impl<S: Borrow<str>> HashStr<S> {
    pub fn try_new(s: S) -> Result<Self> {
        validate_hash(s.borrow())?;
        Ok(HashStr(s))
    }

    /// First two characters.
    pub fn prefix(&self) -> &str {
        &self.0.borrow()[..2]
    }
}

impl<S: Borrow<str>> Display for HashStr<S> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(self.0.borrow())
    }
}

impl<S: Borrow<str>> AsRef<str> for HashStr<S> {
    fn as_ref(&self) -> &str {
        self.0.borrow()
    }
}

fn validate_hash(hash: &str) -> Result<()> {
    ensure!(hash.len() == 40, "hash {:?} wrong len", hash);
    for c in hash.chars() {
        ensure!(
            matches!(c, '0'..='9' | 'a'..='f'),
            "hash {:?} has illegal char {}",
            hash,
            c,
        )
    }
    Ok(())
}
