//! Block implementations.


pub mod simple_gui_block;
pub mod identity_maperator;
pub mod mc;

mod gap;
mod axis_swap;
mod align;
mod layer;
mod margin;
mod pad;
mod modify;
mod logical_size;
mod stack;
mod image;
mod tile_image;
mod tile_9;
mod either;
mod relative;
mod logical_translate;
mod gui_block_seq_chain;
mod mouse_capturer;
mod solid;
mod before_after;
mod min_size;


pub use self::{
    gap::gap,
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
    pad::{
        pad,
        h_pad,
        v_pad,
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
        v_stack_auto,
        h_stack_auto,
    },
    tile_image::tile_image,
    tile_9::{
        Tile9CropConfig,
        tile_9_crop,
        Tile9Parts,
        tile_9,
    },
    either::GuiEither,
    relative::relative,
    logical_translate::logical_translate,
    gui_block_seq_chain::{
        gui_chain,
        gui_seq_flatten,
    },
    mouse_capturer::mouse_capturer,
    solid::solid,
    before_after::before_after,
    min_size::{
        min_width,
        min_height,
        min_size,
    },
};
