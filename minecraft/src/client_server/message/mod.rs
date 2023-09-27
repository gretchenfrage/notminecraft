//! Messages sent between client and server.

use crate::game_binschema::GameBinschema;
use chunk_data::*;
use vek::*;


/// Makes the enum, and makes the variants structs stored in a submodule.
macro_rules! message_enum {
    ($submodule:ident $enum:ident {$(
        $variant:ident {$(
            $field:ident: $field_type:ty,
        )*}
    )*})=>{
        #[derive(Debug, GameBinschema)]
        pub enum $enum {$(
            $variant($submodule::$variant),
        )*}

        $(
            impl From<$submodule::$variant> for $enum {
                fn from(inner: $submodule::$variant) -> Self {
                    $enum::$variant(inner)
                }
            }
        )*

        pub mod $submodule {
            use super::*;

            $(
                #[derive(Debug, GameBinschema)]
                pub struct $variant {$(
                    pub $field: $field_type,
                )*}
            )*
        }
    };
}

message_enum!(edit Edit {
    SetTileBlock {
        lti: u16,
        bid_meta: ErasedTileBlock,
    }
});

message_enum!(up UpMessage {
    LogIn {
        username: String,
    }
    JoinGame {}
    SetTileBlock {
        gtc: Vec3<i64>,
        bid_meta: ErasedTileBlock,
    }
    Say {
        text: String,
    }
    SetCharState {
        char_state: CharState,
    }
});

message_enum!(down DownMessage {
    Close {}
    AcceptLogin {}
    ShouldJoinGame {
        own_client_key: usize,
    }
    AddChunk {
        cc: Vec3<i64>,
        ci: usize,
        chunk_tile_blocks: ChunkBlocks,
    }
    RemoveChunk {
        cc: Vec3<i64>,
        ci: usize,
    }
    AddClient {
        client_key: usize,
        username: String,
        char_state: CharState,
    }
    RemoveClient {
        client_key: usize,
    }
    ApplyEdit {
        ack: Option<u64>,
        ci: usize,
        edit: Edit,
    }
    Ack {
        last_processed: u64,
    }
    ChatLine {
        line: String,
    }
    SetCharState {
        client_key: usize,
        char_state: CharState,
    }
});

/// State of a client's char that's set by the client and streamed back down
/// to other clients.
#[derive(Debug, GameBinschema, Copy, Clone, PartialEq)]
pub struct CharState {
    pub pos: Vec3<f32>,
    pub pitch: f32,
    pub yaw: f32,
    pub pointing: bool,
    pub load_dist: u8,
}
