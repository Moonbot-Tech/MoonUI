use gpui::{App, Entity, Global, OwnedMenu};

use crate::text::TextViewState;

pub(crate) fn init(cx: &mut App) {
    cx.set_global(GlobalState::new());
}

impl Global for GlobalState {}

/// Shared application menus and text-selection coordination.
pub struct GlobalState {
    pub(crate) text_view_state_stack: Vec<Entity<TextViewState>>,
    /// Application menus storage
    app_menus: Vec<OwnedMenu>,
    /// When true, the window-level text selection must not start on the
    /// current mouse down. Set by components that own their own mouse-down
    /// interaction (e.g. `Input`, `Button`); reset by the selection
    /// controller in the capture phase of every left mouse down.
    pub(crate) suppress_text_selection: bool,
}

impl GlobalState {
    /// Start with no menus or active text-selection owners.
    pub(crate) fn new() -> Self {
        Self {
            text_view_state_stack: Vec::new(),
            app_menus: Vec::new(),
            suppress_text_selection: false,
        }
    }

    /// Suppress the window-level text selection for the current mouse down.
    ///
    /// Call this from a mouse-down handler (bubble phase) of a component that
    /// owns its own press/drag interaction, so that pressing it does not start
    /// a window text selection. The flag is reset on the next mouse down.
    pub fn suppress_text_selection(cx: &mut App) {
        Self::global_mut(cx).suppress_text_selection = true;
    }

    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }

    pub fn global_mut(cx: &mut App) -> &mut Self {
        cx.global_mut::<Self>()
    }

    pub(crate) fn text_view_state(&self) -> Option<&Entity<TextViewState>> {
        self.text_view_state_stack.last()
    }

    /// Get the application menus
    pub fn app_menus(&self) -> &[OwnedMenu] {
        &self.app_menus
    }

    /// Set the application menus
    pub fn set_app_menus(&mut self, menus: Vec<OwnedMenu>) {
        self.app_menus = menus;
    }
}
