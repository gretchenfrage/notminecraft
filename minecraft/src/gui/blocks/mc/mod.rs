//! Submodule for highly minecraft-specific gui blocks.

mod splash_text;
mod title;


pub use self::{
    splash_text::GuiSplashText,
    title::{
        GuiTitleBlock,
        GuiTitleNode,
    },
};
