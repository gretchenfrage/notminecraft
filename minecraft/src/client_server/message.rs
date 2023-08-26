//! Messages sent between client and server.

use binschema::{
    error::Result,
    Encoder,
    Decoder,
    Schema,
    schema,
};


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
}

/// Message sent from server to client.
#[derive(Debug)]
pub enum DownMessage {
    /*
    LoadChunk {
        cc: Vec3<i64>,
        ci: usize,
        tile_blocks: ChunkBlocks,
    },
    SetTileBlock {
        ci: usize,
        lti: u16,
        bid: RawBlockId,
    },
    */
}

impl DownMessage {
    pub fn schema() -> Schema {
        schema!(
            enum {
                
            }
        )
    }

    pub fn encode(&self, encoder: &mut Encoder<Vec<u8>>) -> Result<()> {
        match *self {

        }
    }
}
