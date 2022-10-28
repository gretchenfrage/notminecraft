//! Block implementations.


mod simple_gui_block;

mod axis_swap;
mod align;
mod layer;
mod margin;
mod modify;
mod logical_size;
mod stack;
mod tile_image;


//mod mc;
/*mod axis_swap;
mod center;
mod cursor_is_over_tracker;
mod layer;
mod margin;
mod modify;
mod stable_unscaled_size;
mod stack;*/
//mod text;
//mod tile_9;
//mod tile_image;

pub use self::{
    axis_swap::{
        axis_swap,
        axis_swap_seq,
    },
    align::{
        h_align,
        v_align,
    },
    layer::layer,
    margin::{
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
};
