
use crate::{
    item::ItemStack,
    gui::{
        blocks::*,
        GuiBlock,
        DimChildSets,
    },
};
use std::cell::RefCell;


const DEFAULT_UI_SIZE: f32 = 40.0;

#[derive(Debug)]
pub struct ItemSlot {
    pub content: Option<ItemStack>,
    pub scale: f32,
}

impl Default for ItemSlot {
    pub fn default() -> Self {
        ItemSlot {
            content: None,
            scale: 1.0,
        }
    }
}

impl ItemSlot {
    pub fn gui<'a>(&'a mut self) -> impl GuiBlock<'a, DimChildSets, DimChildSets> {
        logical_size(DEFAULT_UI_SIZE,
            
        )
    }
}

//impl<'a> GuiBlock<'a, DimChildSets, DimChildSets> {
//    
//}

/*
pub struct HeldItem {
    pub content: RefCell<Option<ItemStack>>,
}
*/
