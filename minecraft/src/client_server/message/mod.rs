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
        bid: RawBlockId,
    }
});

message_enum!(up UpMessage {
    SetTileBlock {
        gtc: Vec3<i64>,
        bid: RawBlockId,
    }
});

message_enum!(down DownMessage {
    LoadChunk {
        cc: Vec3<i64>,
        ci: usize,
        chunk_tile_blocks: ChunkBlocks,
    }
    ApplyEdit {
        ci: usize,
        edit: Edit,
    }
    Ack {
        processed_before: u64,
    }
});
