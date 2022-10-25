//! State maintained by event loop between GUI events.


use graphics::Renderer;
use std::collections::BTreeSet;
use vek::*;


pub use winit_main::reexports::event::{
    VirtualKeyCode,
    ScanCode,
    MouseButton,
};


/// State maintained by event loop between GUI events that is the same for
/// state frame and all nodes, unaffected by the layout process.
#[derive(Debug, Copy, Clone)]
pub struct GuiGlobalContext<'c> {
    pub renderer: &'c Renderer,
    /// Window focus level.
    pub focus_level: FocusLevel,
    /// Set of pressed virtual key codes, if the window is focused. Empty set
    /// if the window is not focused.
    pub pressed_keys_semantic: &'c BTreeSet<VirtualKeyCode>,
    /// Set of pressed physical keyboard scan codes, if the window is focused.
    /// Empty set if the window is not focused.
    pub pressed_keys_physical: &'c BTreeSet<ScanCode>,
    /// Set of pressed mouse buttons, if the window is focused. Empty set if
    /// the window is not focused.
    pub pressed_mouse_buttons: &'c BTreeSet<MouseButton>,
}

/// State maintained by event loop between GUI events that may be subject to
/// different spatial transformations for different GUI nodes.
#[derive(Debug, Copy, Clone)]
pub struct GuiSpatialContext<'c> {
    /// Space-invariant state.
    pub global: &'c GuiGlobalContext<'c>,
    /// Cursor position in this space, if cursor exists and has a position in
    /// this space, even if the window is unfocused or the cursor is outside of
    /// the window or blocked by a different window.
    ///
    /// Guaranteed to be `None` if `focus_level` == `MouseCaptured`.
    pub cursor_pos: Option<Vec2<f32>>,
}

/// State maintained by event loop between GUI events that is accessable by
/// state frame.
pub struct GuiWindowContext<'c> {
    /// Spatial state without any spatial transformations.
    pub spatial: GuiSpatialContext<'c>,
    /// Window canvas size.
    pub size: Extent2<f32>,
    /// Window UI scaling factor.
    pub scale: f32,
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
