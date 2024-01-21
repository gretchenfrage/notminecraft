//! State maintained by event loop between GUI events.

use crate::{
    asset::Assets,
    sound::SoundPlayer,
    thread_pool::ThreadPool,
    game_data::GameData,
    settings::{Settings, SETTINGS_FILE_NAME},
    gui::{
        gui_event_loop::EventLoopEffectQueue,
        state_frame::GuiStateFrame,
        state_frame_obj::GuiStateFrameObj,
        clipboard::Clipboard,
    },
};
use get_assets::DataDir;
use graphics::{
    Renderer,
    modifier::Transform2,
};
use std::{
    collections::HashSet,
    cell::{RefCell, Ref},
    sync::Arc,
    time::{Duration, Instant},
};
use vek::*;
use tokio::runtime::Handle;


pub use winit::{
    event::MouseButton,
    keyboard::{
        PhysicalKey,
        KeyCode,
    },
};


/// State maintained by event loop between GUI events that is the same for
/// state frame and all nodes, unaffected by the layout process.
#[derive(Debug, Copy, Clone)]
pub struct GuiGlobalContext<'c> {
    /// Queue of store side effects to be executed by gui event loop once
    /// gui state frame method returns.
    pub event_loop: &'c RefCell<EventLoopEffectQueue>, // TODO: these ref cells are ugly
    /// Time since the unix epoch. Monotonically increasing--calibrated once at
    /// start up. Updated upon call to gui state frame.
    pub time_since_epoch: Duration,
    /// Gui event loop's goal for time between frames occuring. Inverse of target FPS.
    pub frame_duration_target: Duration,
    /// Gui event loop's goal for when it wants the next frame to occur.
    pub next_frame_target: Instant,
    /// Renderer. Can be used to load or manipulate GPU resources.
    pub renderer: &'c RefCell<Renderer>,
    /// Handle to the tokio runtime. Can be used to spawn futures.
    pub tokio: &'c Handle,
    /// Handle to our own threadpool for semi-heavy CPU/disk tasks. See module docs.
    pub thread_pool: &'c ThreadPool,
    /// Used to access the system clipboard for copy/paste.
    pub clipboard: &'c Clipboard,
    /// Used to play sounds physically on the user's speakers or whatnot.
    pub sound_player: &'c SoundPlayer,
    /// Game-specific assets loaded in on startup.
    pub assets: &'c Assets,
    /// Game installation directory to use for storing persistent files.
    pub data_dir: &'c DataDir,
    /// Current installation-level game settings.
    pub settings: &'c RefCell<Settings>,
    /// Game content and logic within systems.
    pub game: &'c Arc<GameData>,
    /// Window focus level.
    pub focus_level: FocusLevel,
    /// Set of pressed virtual key codes, if the window is focused. Empty set
    /// if the window is not focused.
    pub pressed_keys: &'c HashSet<PhysicalKey>,
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

    /// Convenience method to check whether any system-appropriate "command"
    /// key is in `self.pressed_keys_semantic`. (The ctrl or command key).
    pub fn is_command_key_pressed(&self) -> bool {
        if cfg!(target_os = "macos") {
            self.pressed_keys.contains(&KeyCode::SuperLeft.into())
            || self.pressed_keys.contains(&KeyCode::SuperRight.into())
        } else {
            self.pressed_keys.contains(&KeyCode::ControlLeft.into())
            || self.pressed_keys.contains(&KeyCode::ControlRight.into())
        }
    }

    pub fn settings(&self) -> Ref<Settings> {
        self.settings.borrow()
    }

    pub fn save_settings(&self) {
        if let Err(e) = self.settings.borrow().write(self.data_dir.subdir(SETTINGS_FILE_NAME)) {
            error!(%e, "error saving settings");
        }
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

    pub fn settings(&self) -> Ref<Settings> {
        self.global.settings()
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

    pub fn settings(&self) -> Ref<Settings> {
        self.spatial.global.settings()
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
