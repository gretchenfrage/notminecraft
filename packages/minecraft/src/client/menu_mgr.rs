//! Manager for the client having a menu open.

use crate::gui::prelude::*;
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

impl MenuMgr {
    /// Construct with no open menu.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the open menu. Won't take effect until next tick.
    pub fn set_menu(&self, menu: Option<Menu>) {
        self.set_to.set(Some(menu));
    }

    /// Call upon client tick.
    pub fn update(&mut self) {
        if let Some(set_to) = self.set_to.get_mut().take() {
            self.menu = set_to;
        }
    }

    /// Get the gui for any menu that may be open.
    pub fn gui<'a>(
        &'a mut self,
        _ctx: &'a GuiWindowContext,
    ) -> impl GuiBlock<'a, DimParentSets, DimParentSets> {
        self.menu.as_mut().map(|menu| match menu {
            Menu::Foo => solid([1.0, 0.0, 0.0, 0.2]),
        })
    }    
}

impl<'a> MenuSetter<'a> {
    /// Set the open menu. Won't take effect until next tick.
    pub fn set_menu(&self, menu: Option<Menu>) {
        self.0.set(Some(menu));
    }
}

/// A menu that the client can have open.
#[derive(Debug)]
pub enum Menu {
    Foo,
}
