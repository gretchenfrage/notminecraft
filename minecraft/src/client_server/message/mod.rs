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
    /*
    SetTileBlock {
        gtc: Vec3<i64>,
        bid: RawBlockId,
    },
    */
}

impl UpMessage {
    pub fn schema() -> Schema {
        schema!(
            enum {
            }
        )
    }

    pub fn decode(decoder: &mut Decoder<&[u8]>) -> Result<Self> {
        match decoder.begin_enum()? {
            _ => unreachable!()
        }
    }

    pub fn encode(&self, encoder: &mut Encoder<Vec<u8>>) -> Result<()> {
        match *self {
            
        }
    }
}

/// Message sent from server to client.
#[derive(Debug)]
pub enum DownMessage {
    LoadChunk(DownMessageLoadChunk),
    /*
    SetTileBlock {
        ci: usize,
        lti: u16,
        bid: RawBlockId,
    },
    */
}

#[derive(Debug)]
pub struct DownMessageLoadChunk {
    pub cc: Vec3<i64>,
    pub ci: usize,
    pub chunk_tile_blocks: ChunkBlocks,
}

impl DownMessage {
    pub fn schema() -> Schema {
        schema!(
            enum {
                LoadChunk(%DownMessageLoadChunk::schema()),
            }
        )
    }

    pub fn encode(&self, encoder: &mut Encoder<Vec<u8>>) -> Result<()> {
        match self {
            &DownMessage::LoadChunk(ref inner) => {
                encoder.begin_enum(0, "LoadChunk")?;
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
            _ => unreachable!()
        })
    }
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
