//! State maintained by event loop between GUI events.

use crate::{
    asset::Assets,
    sound::SoundPlayer,
    game_data::GameData,
    gui::{
        gui_event_loop::EventLoopEffectQueue,
        state_frame::GuiStateFrame,
        state_frame_obj::GuiStateFrameObj,
    },
};
use graphics::{
    Renderer,
    modifier::Transform2,
};
use std::{
    collections::HashSet,
    cell::RefCell,
    sync::Arc,
};
use vek::*;
use tokio::runtime::Handle;


pub use winit::event::{
    VirtualKeyCode,
    ScanCode,
    MouseButton,
};


/// State maintained by event loop between GUI events that is the same for
/// state frame and all nodes, unaffected by the layout process.
#[derive(Debug, Copy, Clone)]
pub struct GuiGlobalContext<'c> {
    pub event_loop: &'c RefCell<EventLoopEffectQueue>, // TODO: these ref cells are ugly
    pub renderer: &'c RefCell<Renderer>,
    pub tokio: &'c Handle,
    pub sound_player: &'c SoundPlayer,
    pub assets: &'c Assets,
    pub game: &'c Arc<GameData>,
    /// Window focus level.
    pub focus_level: FocusLevel,
    /// Set of pressed virtual key codes, if the window is focused. Empty set
    /// if the window is not focused.
    pub pressed_keys_semantic: &'c HashSet<VirtualKeyCode>,
    /// Set of pressed physical keyboard scan codes, if the window is focused.
    /// Empty set if the window is not focused.
    pub pressed_keys_physical: &'c HashSet<ScanCode>,
    /// Set of pressed mouse buttons, if the window is focused. Empty set if
    /// the window is not focused.
    pub pressed_mouse_buttons: &'c HashSet<MouseButton>,
}

/// State maintained by event loop between GUI events that may be subject to
/// different spatial transformations for different GUI nodes.
#[derive(Debug, Copy, Clone)]
pub struct GuiSpatialContext<'c> {
    /// Space-invariant state.
    pub global: &'c GuiGlobalContext<'c>,
    /// Cursor position in this space, if cursor exists and has a finite
    /// position in this space, even if the window is unfocused or the cursor
    /// is outside of the window or blocked by a different window or clipped.
    ///
    /// Guaranteed to be `None` if `focus_level` == `MouseCaptured`.
    pub cursor_pos: Option<Vec2<f32>>,
}

impl<'c> GuiGlobalContext<'c> {
    pub fn pop_state_frame(&self) {
        self.event_loop.borrow_mut().pop_state_frame();
    }

    pub fn push_state_frame<T>(&self, state_frame: T)
    where
        T: GuiStateFrame + 'static,
    {
        self.event_loop.borrow_mut().push_state_frame(state_frame);
    }

    pub fn push_state_frame_obj(
        &self,
        state_frame: Box<dyn GuiStateFrameObj>,
    ) {
        self.event_loop.borrow_mut().push_state_frame_obj(state_frame);
    }

    pub fn set_scale(&self, scale: f32) {
        self.event_loop.borrow_mut().set_scale(scale);
    }

    pub fn capture_mouse(&self) {
        self.event_loop.borrow_mut().capture_mouse();
    }

    pub fn uncapture_mouse(&self) {
        self.event_loop.borrow_mut().uncapture_mouse();
    }
}

impl<'c> GuiSpatialContext<'c> {
    /// Relativize the spatial contextual state against the given
    /// transformation.
    ///
    /// That effectively means applying the transformation to coordinates
    /// in reverse. Irreversibility-safe.
    pub fn relativize(&mut self, transform: Transform2) {
        if let Some(cursor_pos) = self.cursor_pos {
            self.cursor_pos = transform
                .reverse()
                .map(|reversed| reversed.apply(cursor_pos));
        }
    }

    pub fn sound_player(&self) -> &'c SoundPlayer {
        &self.global.sound_player
    }

    pub fn assets(&self) -> &'c Assets {
        &self.global.assets
    }

    pub fn game(&self) -> &'c Arc<GameData> {
        &self.global.game
    }

    pub fn cursor_in_area<A, B>(&self, min: A, max: B) -> bool
    where
        A: Into<Vec2<f32>>,
        B: Into<Vec2<f32>>,
    {
        if let Some(pos) = self.cursor_pos {
            let min = min.into();
            let max = max.into();
            
            pos.x >= min.x
                && pos.y >= min.y
                && pos.x <= max.x
                && pos.y <= max.y
        } else {
            false
        }
    }
}

/// State maintained by event loop between GUI events that is accessable by
/// state frame.
pub struct GuiWindowContext<'c> {
    /// Spatial state without any spatial transformations.
    pub spatial: GuiSpatialContext<'c>,
    /// Window canvas size.
    pub size: Extent2<u32>,
    /// Window UI scaling factor.
    pub scale: f32,
}

impl<'c> GuiWindowContext<'c> {
    pub fn global(&self) -> &'c GuiGlobalContext<'c> {
        &self.spatial.global
    }

    pub fn sound_player(&self) -> &'c SoundPlayer {
        &self.spatial.global.sound_player
    }

    pub fn assets(&self) -> &'c Assets {
        &self.spatial.global.assets
    }

    pub fn game(&self) -> &'c Arc<GameData> {
        &self.spatial.global.game
    }
}

/// Window focus level. These form a semantically meaningful ordering, in which
/// greater focus levels are "more" focused than their previous levels. Thus it
/// is often appropriate to compare with comparison operators rather than just
/// equality checks.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum FocusLevel {
    /// Window is unfocused.
    Unfocused,
    /// Window is focused by the cursor is not captured.
    Focused,
    /// Window is focused and the cursor is captured.
    MouseCaptured,
}
