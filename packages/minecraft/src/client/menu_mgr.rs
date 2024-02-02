//! Manager for the client having a menu open.

use crate::{
    client::menu_esc::EscMenu,
    gui::prelude::*,
};
use std::cell::Cell;


/// Manager for the client having a menu open.
#[derive(Default)]
pub struct MenuMgr {
    // currently open menu
    menu: Option<Menu>,
    // if Some, menu will be set to this value next tick
    set_to: Cell<Option<Option<Menu>>>,
}

/// Shareable callback for a menu to set the open menu to something else.
#[derive(Copy, Clone)]
pub struct MenuSetter<'a>(&'a Cell<Option<Option<Menu>>>);

/// A menu that the client can have open.
#[derive(Debug)]
pub enum Menu {
    EscMenu(EscMenu),
}

impl MenuMgr {
    /// Construct with no open menu.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the open menu.
    pub fn set_menu<M: Into<Menu>>(&self, menu: M) {
        self.set_menu_opt(Some(menu.into()))
    }

    /// Clear the open menu.
    pub fn clear_menu(&self) {
        self.set_menu_opt(None);
    }

    /// Set or clear the open menu.
    pub fn set_menu_opt(&self, menu: Option<Menu>) {
        self.set_to.set(Some(menu));
    }

    /// Whether a menu is open.
    pub fn is_open_menu(&self) -> bool {
        self.menu.is_some()
    }

    /// Get the gui for any menu that may be open.
    pub fn gui<'a>(
        &'a mut self,
        _ctx: &'a GuiWindowContext,
    ) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
        self.menu.as_mut().map(|menu| {
            // darkened background
            let darkened_bg = match menu {
                &mut Menu::EscMenu(_) => true,
            };
            let darkened_bg = if darkened_bg {
                Some(solid([0.0, 0.0, 0.0, 1.0 - 0x2a as f32 / 0x97 as f32]))
            } else { None };

            // delegate
            let inner = match menu {
                &mut Menu::EscMenu(ref mut inner) => inner.gui(),
            };

            // compose
            layer((
                darkened_bg,
                inner,
            ))
        })
    }

    /// Have the open menu handle this gui state event, if a menu is open.
    pub fn on_key_press(
        &mut self,
        _: &GuiWindowContext,
        key: PhysicalKey,
        _: Option<TypingInput>,
    ) {
        if let Some(menu) = self.menu.as_mut() {
            let menu_setter = MenuSetter(&self.set_to);
            match menu {
                &mut Menu::EscMenu(ref mut inner) => inner.on_key_press(key, menu_setter),
            }
        }
    }

    /// Handle menu gui effects.
    pub fn process_gui_effects(&mut self, ctx: &GuiWindowContext) {
        if let Some(set_to) = self.set_to.take() {
            self.menu = set_to;
        }
    }
}

impl<'a> MenuSetter<'a> {
    /// Set the open menu.
    pub fn set_menu<M: Into<Menu>>(&self, menu: M) {
        self.set_menu_opt(Some(menu.into()))
    }

    /// Clear the open menu.
    pub fn clear_menu(&self) {
        self.set_menu_opt(None);
    }

    /// Set or clear the open menu.
    pub fn set_menu_opt(&self, menu: Option<Menu>) {
        self.0.set(Some(menu));
    }
}

impl From<EscMenu> for Menu {
    fn from(inner: EscMenu) -> Self {
        Menu::EscMenu(inner)
    }
}
