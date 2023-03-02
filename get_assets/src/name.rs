//! Asset names are sequences of name parts separated by forward slashes.

use std::{
    path::{
        Path,
        PathBuf,
        Component,
    },
    borrow::Borrow,
    fmt::{self, Formatter, Display},
};
use anyhow::{
    Result,
    ensure,
    bail,
};


const NAME_SEPARATOR: char = '/';


/// Pre-validated asset name.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct AssetName<S>(S);

impl<S: Borrow<str>> AssetName<S> {
    pub fn try_new(s: S) -> Result<Self> {
        validate_name(s.borrow())?;
        Ok(AssetName(s))
    }

    /// Guarantees made by having been validated:
    /// - won't panic
    /// - at least 1 part
    /// - all parts will parse as `std::path::Component::Normal`
    pub fn parts<'a>(&'a self) -> impl Iterator<Item=&'a str> + 'a
    {
        self.0.borrow().split(NAME_SEPARATOR)
    }

    /// Convert to relative path.
    pub fn to_path(&self) -> PathBuf {
        let mut path = PathBuf::new();
        path.extend(self.parts());
        path
    }
}

impl<S: Borrow<str>> Display for AssetName<S> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(self.0.borrow())
    }
}

fn validate_name_part(part: &str) -> Result<()> {
    let mut comps = Path::new(part).components();
    match comps.next() {
        Some(Component::Normal(os_str)) => ensure!(
            os_str.to_str().is_some(),
            "invalid asset name part {:?}, path-parsed as non-utf8 os string",
            part,
        ),
        Some(_) => bail!(
            "invalid asset name part {:?}, path-parsed as non-Component::Normal",
            part,
        ),
        None => bail!(
            "invalid asset name part {:?}, path-parsed as empty component sequence",
            part,
        ),
    }
    ensure!(
        comps.next().is_none(),
        "invalid asset name part {:?}, path-parsed as multiple components",
        part,
    );
    Ok(())
}

fn validate_name(name: &str) -> Result<()> {
    let mut num_parts = 0;
    for name in name.split(NAME_SEPARATOR) {
        num_parts += 1;
        validate_name_part(name)?;
    }
    ensure!(
        num_parts > 0,
        "invalid asset name {:?}, has 0 parts",
        name,
    );
    Ok(())
}
