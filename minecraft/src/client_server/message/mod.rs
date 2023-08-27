//! Messages sent between client and server.

pub mod transcode_vek;


use self::transcode_vek::*;
use crate::game_data::GameData;
use binschema::{
    error::Result,
    *,
};
use chunk_data::*;
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use vek::*;


/// Message sent from client to server.
#[derive(Debug)]
pub enum UpMessage {
    SetTileBlock(UpMessageSetTileBlock),
}

impl UpMessage {
    pub fn schema() -> Schema {
        schema!(
            enum {
                SetTileBlock(%UpMessageSetTileBlock::schema()),
            }
        )
    }

    pub fn encode(&self, encoder: &mut Encoder<Vec<u8>>) -> Result<()> {
        match self {
            &UpMessage::SetTileBlock(ref inner) => {
                encoder.begin_enum(0, "SetTileBlock")?;
                inner.encode(encoder)
            }
        }
    }

    pub fn decode(decoder: &mut Decoder<&[u8]>) -> Result<Self> {
        Ok(match decoder.begin_enum()? {
            0 => {
                decoder.begin_enum_variant("SetTileBlock")?;
                UpMessage::SetTileBlock(UpMessageSetTileBlock::decode(decoder)?)
            }
            _ => unreachable!()
        })
    }
}

#[derive(Debug)]
pub struct UpMessageSetTileBlock {
    pub gtc: Vec3<i64>,
    pub bid: RawBlockId,
}

impl UpMessageSetTileBlock {
    pub fn schema() -> Schema {
        schema!(
            struct {
                (gtc: %vec3_schema::<i64>()),
                (bid: u16),
            }
        )
    }

    pub fn encode(&self, encoder: &mut Encoder<Vec<u8>>) -> Result<()> {
        encoder.begin_struct()?;
        encoder.begin_struct_field("gtc")?;
        vec3_encode::<i64, _>(self.gtc, &mut *encoder)?;
        encoder.begin_struct_field("bid")?;
        encoder.encode_u16(self.bid.0)?;
        encoder.finish_struct()
    }

    pub fn decode(decoder: &mut Decoder<&[u8]>) -> Result<Self> {
        decoder.begin_struct()?;
        let value = UpMessageSetTileBlock {
            gtc: {
                decoder.begin_struct_field("gtc")?;
                vec3_decode(&mut *decoder)?
            },
            bid: {
                decoder.begin_struct_field("bid")?;
                RawBlockId(decoder.decode_u16()?)
            },
        };
        decoder.finish_struct()?;
        Ok(value)
    }
}


/// Message sent from server to client.
#[derive(Debug)]
pub enum DownMessage {
    LoadChunk(DownMessageLoadChunk),
    SetTileBlock(DownMessageSetTileBlock),
}

impl DownMessage {
    pub fn schema() -> Schema {
        schema!(
            enum {
                LoadChunk(%DownMessageLoadChunk::schema()),
                SetTileBlock(%DownMessageSetTileBlock::schema()),
            }
        )
    }

    pub fn encode(&self, encoder: &mut Encoder<Vec<u8>>) -> Result<()> {
        match self {
            &DownMessage::LoadChunk(ref inner) => {
                encoder.begin_enum(0, "LoadChunk")?;
                inner.encode(encoder)
            }
            &DownMessage::SetTileBlock(ref inner) => {
                encoder.begin_enum(1, "SetTileBlock")?;
                inner.encode(encoder)
            }
        }
    }

    pub fn decode(decoder: &mut Decoder<&[u8]>, game: &Arc<GameData>) -> Result<Self> {
        Ok(match decoder.begin_enum()? {
            0 => {
                decoder.begin_enum_variant("LoadChunk")?;
                DownMessage::LoadChunk(DownMessageLoadChunk::decode(decoder, game)?)
            }
            1 => {
                decoder.begin_enum_variant("SetTileBlock")?;
                DownMessage::SetTileBlock(DownMessageSetTileBlock::decode(decoder)?)
            }
            _ => unreachable!()
        })
    }
}

#[derive(Debug)]
pub struct DownMessageLoadChunk {
    pub cc: Vec3<i64>,
    pub ci: usize,
    pub chunk_tile_blocks: ChunkBlocks,
}

impl DownMessageLoadChunk {
    pub fn schema() -> Schema {
        schema!(
            struct {
                (cc: %vec3_schema::<i64>()),
                (ci: %usize::schema(Default::default())),
                (chunk_tile_blocks: seq(NUM_LTIS)(u16)),
            }
        )
    }

    pub fn encode(&self, encoder: &mut Encoder<Vec<u8>>) -> Result<()> {
        encoder.begin_struct()?;
        encoder.begin_struct_field("cc")?;
        vec3_encode::<i64, _>(self.cc, encoder)?;
        encoder.begin_struct_field("ci")?;
        self.ci.serialize(&mut *encoder)?;
        encoder.begin_struct_field("chunk_tile_blocks")?;
        encoder.begin_fixed_len_seq(NUM_LTIS)?;
        for lti in 0..=MAX_LTI {
            encoder.begin_seq_elem()?;
            encoder.encode_u16(self.chunk_tile_blocks.get(lti).0)?;
            self.chunk_tile_blocks.raw_meta::<()>(lti);
        }
        encoder.finish_seq()?;
        encoder.finish_struct()
    }

    pub fn decode(decoder: &mut Decoder<&[u8]>, game: &Arc<GameData>) -> Result<Self> {
        decoder.begin_struct()?;
        let value = DownMessageLoadChunk {
            cc: {
                decoder.begin_struct_field("cc")?;
                vec3_decode(decoder)?
            },
            ci: {
                decoder.begin_struct_field("ci")?;
                usize::deserialize(&mut *decoder)?
            },
            chunk_tile_blocks: {
                decoder.begin_struct_field("chunk_tile_blocks")?;
                decoder.begin_fixed_len_seq(NUM_LTIS)?;
                let mut chunk_tile_blocks = ChunkBlocks::new(&game.blocks);
                for lti in 0..=MAX_LTI {
                    decoder.begin_seq_elem()?;
                    chunk_tile_blocks.raw_set(
                        lti,
                        RawBlockId(decoder.decode_u16()?),
                        (),
                    );
                }
                decoder.finish_seq()?;
                chunk_tile_blocks
            }
        };
        decoder.finish_struct()?;
        Ok(value)
    }
}

#[derive(Debug)]
pub struct DownMessageSetTileBlock {
    pub ci: usize,
    pub lti: u16,
    pub bid: RawBlockId,
}

impl DownMessageSetTileBlock {
    pub fn schema() -> Schema {
        schema!(
            struct {
                (ci: %usize::schema(Default::default())),
                (lti: u16),
                (bid: u16),
            }
        )
    }

    pub fn encode(&self, encoder: &mut Encoder<Vec<u8>>) -> Result<()> {
        encoder.begin_struct()?;
        encoder.begin_struct_field("ci")?;
        self.ci.serialize(&mut *encoder)?;
        encoder.begin_struct_field("lti")?;
        encoder.encode_u16(self.lti)?;
        encoder.begin_struct_field("bid")?;
        encoder.encode_u16(self.bid.0)?;
        encoder.finish_struct()
    }

    pub fn decode(decoder: &mut Decoder<&[u8]>) -> Result<Self> {
        decoder.begin_struct()?;
        let value = DownMessageSetTileBlock {
            ci: {
                decoder.begin_struct_field("ci")?;
                usize::deserialize(&mut *decoder)?
            },
            lti: {
                decoder.begin_struct_field("lti")?;
                decoder.decode_u16()?
            },
            bid: {
                decoder.begin_struct_field("bid")?;
                RawBlockId(decoder.decode_u16()?)
            },
        };
        decoder.finish_struct()?;
        Ok(value)
    }
}
