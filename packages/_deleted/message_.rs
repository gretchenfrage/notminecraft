//! Messages sent between client and server.

use crate::{
    game_binschema::GameBinschema,
    item::*,
    util::less_than::UsizeLessThan,
};
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

message_enum!(up UpMessage {
    LogIn {
        username: String,
    }
    JoinGame {}
    AcceptMoreChunks {
        number: u32,
    }
    SetTileBlock {
        gtc: Vec3<i64>,
        bid_meta: ErasedBidMeta,
    }
    Say {
        text: String,
    }
    SetCharState {
        char_state: CharState,
    }
    OpenGameMenu {
        menu: GameMenu,
    }
    CloseGameMenu {}
    GameMenuAction {
        action: super::GameMenuAction,
    }
});

message_enum!(edit Edit {
    Tile {
        ci: usize,
        lti: u16,
        edit: TileEdit,
    }
    InventorySlot {
        slot_idx: usize,
        edit: InventorySlotEdit,
    }
});

message_enum!(tile_edit TileEdit {
    SetTileBlock {
        bid_meta: ErasedBidMeta,
    }
});

message_enum!(inventory_slot_edit InventorySlotEdit {
    SetInventorySlot {
        slot_val: ItemSlot,
    }
});

message_enum!(down DownMessage {
    Close {
        message: String,
    }
    AcceptLogin {
        inventory_slots: [ItemSlot; 36],
    }
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
    CloseGameMenu {
        open_menu_msg_idx: u64,
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

/// An in-game menu a client can have open. 
#[derive(Debug, GameBinschema, Copy, Clone, PartialEq)]
pub enum GameMenu {
    Inventory,
    Chest {
        gtc: Vec3<i64>,
    },
}

/// Reference to an item slot relative to the game menu a client has open.
#[derive(Debug, GameBinschema, Copy, Clone, PartialEq)]
pub enum ItemSlotReference {
    /// The item currently picked up with the cursor in a game menu.
    Held,
    /// First 9 are hotbar.
    Inventory(UsizeLessThan<36>),
    Armor(UsizeLessThan<4>),
    /// The small 2x2 crafting square.
    InventoryCrafting(UsizeLessThan<4>),
    InventoryCraftingOutput,
    Chest(UsizeLessThan<27>),
}

/// Action a player can try to do relative to their currently open game gui.
#[derive(Debug, GameBinschema, Copy, Clone, PartialEq)]
pub enum GameMenuAction {
    /// Attempt to move the given number of items from one slot to another.
    TransferItems {
        from: ItemSlotReference,
        to: ItemSlotReference,
        amount: u8,
    },
    /// Attempt to swap the contents of two item slots.
    SwapItemSlots([ItemSlotReference; 2]),
}
