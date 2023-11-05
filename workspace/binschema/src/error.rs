//! Error types.

use crate::coder::coder::CoderState;
use std::fmt::{self, Formatter, Display};


pub type Result<I> = std::result::Result<I, Error>; 

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    error: Box<dyn std::error::Error + Send + Sync>,
    coder_state: Option<String>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ErrorKind {
    /// Underlying IO error.
    ///
    /// This corresponds with the (en/de)coder being left in a **"broken"**
    /// state that rejects further coding API calls.
    Io,

    /// (Only when decoding) the bytes being decoded are not a valid message
    /// for the given schema.
    ///
    /// This corresponds with the (en/de)coder being left in a **"broken"**
    /// state that rejects further coding API calls.
    MalformedData,

    /// The shape of the data the user of this library tried to (en/de)code is
    /// not valid for the given schema.
    ///
    /// This corresponds with the (en/de)coder being left in an **unchanged**
    /// state.
    SchemaNonConformance,

    /// The program or its environment is not capable of (en/de)coding what
    /// may be an otherwise valid message.
    ///
    /// Often, this may actually be indicative of malformed data.
    ///
    /// This corresponds with the (en/de)coder being left in a **"broken"**
    /// state that rejects further coding API calls.
    PlatformLimits,

    /// The schema itself is illegal. This kind of error can be front-runned by
    /// validating the schema.
    ///
    /// This corresponds with the (en/de)coder being left in a **"broken"**
    /// state that rejects further coding API calls.
    IllegalSchema,

    /// The user of this library performed a sequence of API calls that would
    /// never be valid.
    ///
    /// This corresponds with the (en/de)coder being left in an **unchanged**
    /// state.
    ApiUsage,

    /// Some "other" error type. Encoders and decoders will not themselves
    /// produce this. It could represent the failure of a higher-level
    /// invariant, such as a map with duplicate keys.
    Other,
}

impl Error {
    pub fn new<E>(
        kind: ErrorKind,
        error: E,
        coder_state: Option<&CoderState>,
    ) -> Self
    where
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        Error {
            kind,
            error: error.into(),
            coder_state: coder_state.map(|state| format!("{:?}", state)),
        }
    }

    pub fn other<E>(error: E) -> Self
    where
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        Self::new(ErrorKind::Other, error, None)
    }

    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    pub fn inner(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
        &*self.error
    }

    pub fn inner_mut(&mut self) -> &mut (dyn std::error::Error + Send + Sync + 'static) {
        &mut *self.error
    }

    pub fn into_inner(self) -> Box<dyn std::error::Error + Send + Sync + 'static> {
        self.error
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::new(ErrorKind::Io, error, None)
    }
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(match *self {
            ErrorKind::Io => "IO error",
            ErrorKind::MalformedData => "malformed data",
            ErrorKind::SchemaNonConformance => "schema non-comformance error",
            ErrorKind::PlatformLimits => "platform limits or malformed data",
            ErrorKind::IllegalSchema => "illegal schema",
            ErrorKind::ApiUsage => "API usage error",
            ErrorKind::Other => "unknown error",
        })
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.kind, f)?;
        f.write_str(", ")?;
        Display::fmt(&self.error, f)?;
        if let Some(ref coder_state) = self.coder_state {
            f.write_str("\nstate: ")?;
            f.write_str(coder_state)?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.inner())
    }
}


macro_rules! error {
    ($k:ident, $coder_state:expr, $($e:tt)*)=>{
        $crate::error::Error::new(
            $crate::error::ErrorKind::$k,
            format!($($e)*),
            $coder_state,
        )
    };
}

macro_rules! bail {
    ($($e:tt)*)=>{ return Err(error!($($e)*)) };
}

macro_rules! ensure {
    ($c:expr, $($e:tt)*)=>{
        if !$c {
            bail!($($e)*);
        }
    };
}

pub(crate) use error;
pub(crate) use bail;
pub(crate) use ensure;
