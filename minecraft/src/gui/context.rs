//! State maintained by event loop between GUI events.

use crate::{
    asset::{
        resource_pack::ResourcePack,
        localization::Localization,
    },
    gui::gui_event_loop::EventLoopEffectQueue,
};
use graphics::{
    Renderer,
    modifier::Transform2,
};
use std::{
    collections::HashSet,
    cell::RefCell,
};
use vek::*;


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
    pub resources: &'c ResourcePack,
    pub lang: &'c Localization,
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

    pub fn resources(&self) -> &'c ResourcePack {
        &self.global.resources
    }

    pub fn lang(&self) -> &'c Localization {
        &self.global.lang
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

    pub fn resources(&self) -> &'c ResourcePack {
        &self.spatial.global.resources
    }

    pub fn lang(&self) -> &'c Localization {
        &self.spatial.global.lang
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