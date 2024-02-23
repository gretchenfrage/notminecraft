/// `usize` with type-level exclusive maximum.

use crate::{
    game_data::*,
    game_binschema::*,
};
use binschema::{
    *,
    error as binschema_error,
};
use std::{
    sync::Arc,
    io::Cursor,
};
use anyhow::*;


/// `usize` with type-level exclusive maximum.
///
/// Binschema-transcodes with the smallest schema the bound allows.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct UsizeLt<const LT: usize>(usize);

impl<const LT: usize> UsizeLt<LT> {
    /// Construct, panicking if `!(n < LT)`.
    pub fn new(n: usize) -> Self {
        assert!(n < LT);
        UsizeLt(n)
    }

    /// Get as a usize.
    pub fn get(self) -> usize {
        self.0
    }

    /// Use to index into an array with type-guaranteed safety.
    pub fn idx<T>(self, array: &[T; LT]) -> &T {
        unsafe { array.get_unchecked(self.0) }
    }

    /// Use to mutably index into an array with type-guaranteed safety.
    pub fn idx_mut<T>(self, array: &mut [T; LT]) -> &mut T {
        unsafe { array.get_unchecked_mut(self.0) }
    }
}

impl<const LT: usize> Into<usize> for UsizeLt<LT> {
    fn into(self) -> usize {
        self.0
    }
}

impl<const LT: usize> TryFrom<usize> for UsizeLt<LT> {
    type Error = Error;

    fn try_from(n: usize) -> Result<Self> {
        ensure!(n < LT, "{} not < {}", n, LT);
        Ok(UsizeLt(n))
    }
}

impl<const LT: usize> GameBinschema for UsizeLt<LT> {
    fn schema(game: &Arc<GameData>) -> Schema {
        if (u8::MAX as usize) < LT {
            <u8 as GameBinschema>::schema(game)
        } else if (u16::MAX as usize) < LT {
            <u16 as GameBinschema>::schema(game)
        } else if (u32::MAX as usize) < LT {
            <u32 as GameBinschema>::schema(game)
        } else if (u64::MAX as usize) < LT {
            <u64 as GameBinschema>::schema(game)
        } else {
            <usize as GameBinschema>::schema(game)
        }
    }

    fn encode(
        &self,
        encoder: &mut Encoder<'_, '_, Vec<u8>>,
        game: &Arc<GameData>,
    ) -> binschema_error::Result<()> {
        if (u8::MAX as usize) < LT {
            <u8 as GameBinschema>::encode(&(self.0 as u8), encoder, game)
        } else if (u16::MAX as usize) < LT {
            <u16 as GameBinschema>::encode(&(self.0 as u16), encoder, game)
        } else if (u32::MAX as usize) < LT {
            <u32 as GameBinschema>::encode(&(self.0 as u32), encoder, game)
        } else if (u64::MAX as usize) < LT {
            <u64 as GameBinschema>::encode(&(self.0 as u64), encoder, game)
        } else {
            <usize as GameBinschema>::encode(&self.0, encoder, game)
        }
    }

    fn decode(
        decoder: &mut Decoder<'_, '_, Cursor<&[u8]>>,
        game: &Arc<GameData>,
    ) -> binschema_error::Result<Self> {
        let n = if (u8::MAX as usize) < LT {
            <u8 as GameBinschema>::decode(decoder, game)? as usize
        } else if (u16::MAX as usize) < LT {
            <u16 as GameBinschema>::decode(decoder, game)? as usize
        } else if (u32::MAX as usize) < LT {
            usize::try_from(<u32 as GameBinschema>::decode(decoder, game)?)
                .map_err(binschema_error::Error::other)?
        } else if (u64::MAX as usize) < LT {
            usize::try_from(<u64 as GameBinschema>::decode(decoder, game)?)
                .map_err(binschema_error::Error::other)?
        } else {
            <usize as GameBinschema>::decode(decoder, game)?
        };
        Self::try_from(n).map_err(binschema_error::Error::other)
    }
}
