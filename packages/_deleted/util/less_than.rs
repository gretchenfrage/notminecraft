
use crate::{
    game_data::*,
    game_binschema::*,
};
use binschema::{
    *,
    error as binschema_error,
};
use std::sync::Arc;
use anyhow::*;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct UsizeLessThan<const LT: usize>(usize);

impl<const LT: usize> UsizeLessThan<LT> {
    pub fn new(n: usize) -> Self {
        assert!(n < LT);
        UsizeLessThan(n)
    }

    pub fn get(self) -> usize {
        self.0
    }
}

impl<const LT: usize> Into<usize> for UsizeLessThan<LT> {
    fn into(self) -> usize {
        self.0
    }
}

impl<const LT: usize> TryFrom<usize> for UsizeLessThan<LT> {
    type Error = Error;

    fn try_from(n: usize) -> Result<Self> {
        ensure!(n < LT, "{} not < {}", n, LT);
        Ok(UsizeLessThan(n))
    }
}

impl<const LT: usize> GameBinschema for UsizeLessThan<LT> {
    fn schema(game: &Arc<GameData>) -> Schema {
        <usize as GameBinschema>::schema(game)
    }

    fn encode(
        &self,
        encoder: &mut Encoder<'_, '_, Vec<u8>>,
        game: &Arc<GameData>,
    ) -> binschema_error::Result<()> {
        <usize as GameBinschema>::encode(&self.0, encoder, game)
    }

    fn decode(
        decoder: &mut Decoder<'_, '_, &[u8]>,
        game: &Arc<GameData>,
    ) -> binschema_error::Result<Self> {
        let n = <usize as GameBinschema>::decode(decoder, game)?;
        Self::try_from(n)
            .map_err(binschema_error::Error::other)
    }
}
