//! Block implementations.


pub mod simple_gui_block;

pub mod mc;

mod axis_swap;
mod align;
mod layer;
mod margin;
mod modify;
mod logical_size;
mod stack;
mod image;
mod tile_image;
mod tile_9;
mod text;


pub use self::{
    axis_swap::{
        axis_swap,
        axis_swap_seq,
    },
    align::{
        align,
        h_align,
        v_align,
    },
    layer::layer,
    margin::{
        margin,
        h_margin,
        v_margin,
    },
    modify::modify,
    logical_size::{
    	logical_width,
    	logical_height,
    	logical_size,
    },
    stack::{
        v_stack,
        h_stack,
    },
    tile_image::tile_image,
    tile_9::{
        Tile9CropConfig,
        tile_9_crop,
        Tile9Parts,
        tile_9,
    },
    text::{
        GuiTextBlockConfig,
        GuiTextBlock, 
    }
};
