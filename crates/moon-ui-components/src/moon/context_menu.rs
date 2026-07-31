use std::rc::Rc;

use crate::WindowExt as _;
use gpui::*;

use super::{
    dropdown::{MoonMenuItem, MoonMenuLevel, MoonPopupMenu},
    tokens::MoonRect,
};

#[derive(IntoElement)]
/// Positioned Moon popup level used inside the root-owned context-menu overlay.
pub struct MoonContextMenu {
    id: SharedString,
    bounds: Option<MoonRect>,
    position: Option<Point<Pixels>>,
    items: MoonMenuLevel,
    open: bool,
    width: f32,
    max_height: Option<f32>,
}

type MoonContextDismissHandler = Rc<dyn Fn(&mut Window, &mut App)>;

#[derive(IntoElement)]
/// Full-window dismiss layer that owns one positioned Moon context menu.
pub struct MoonContextMenuOverlay {
    id: SharedString,
    bounds: Option<MoonRect>,
    position: Option<Point<Pixels>>,
    items: MoonMenuLevel,
    open: bool,
    width: f32,
    max_height: Option<f32>,
    on_dismiss: Option<MoonContextDismissHandler>,
}

pub trait MoonContextMenuWindowExt {
    fn open_moon_context_menu(
        &mut self,
        cx: &mut App,
        id: impl Into<SharedString>,
        position: Point<Pixels>,
        items: Vec<MoonMenuItem>,
        width: f32,
    );

    fn open_moon_context_menu_with_dismiss(
        &mut self,
        cx: &mut App,
        id: impl Into<SharedString>,
        position: Point<Pixels>,
        items: Vec<MoonMenuItem>,
        width: f32,
        on_dismiss: impl Fn(&mut Window, &mut App) + 'static,
    );
}

impl MoonContextMenu {
    /// Create a closed context menu with empty shared row storage.
    ///
    /// Args:
    ///     id: Stable identity used by the menu and retained virtual-list state.
    ///
    /// Returns:
    ///     A default context-menu builder.
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            position: None,
            items: MoonMenuLevel::new([]),
            open: false,
            width: 180.0,
            max_height: None,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self.position = None;
        self
    }

    pub fn position(mut self, position: Point<Pixels>) -> Self {
        self.position = Some(position);
        self.bounds = None;
        self
    }

    /// Append one context-menu row without a later layout scan.
    ///
    /// Args:
    ///     item: Row to append.
    ///
    /// Returns:
    ///     The updated menu.
    pub fn item(mut self, item: MoonMenuItem) -> Self {
        self.items.extend([item]);
        self
    }

    /// Append context-menu rows while accumulating their layout signature.
    ///
    /// Args:
    ///     items: Rows in display order.
    ///
    /// Returns:
    ///     The updated menu.
    pub fn items(mut self, items: impl IntoIterator<Item = MoonMenuItem>) -> Self {
        self.items.extend(items);
        self
    }

    /// Install a retained menu level without cloning its rows.
    ///
    /// Args:
    ///     items: Shared row storage and layout signature.
    ///
    /// Returns:
    ///     The updated menu.
    fn shared_items(mut self, items: MoonMenuLevel) -> Self {
        self.items = items;
        self
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn max_height(mut self, max_height: f32) -> Self {
        self.max_height = Some(max_height);
        self
    }
}

impl MoonContextMenuOverlay {
    /// Create a closed root-owned context-menu overlay.
    ///
    /// Args:
    ///     id: Stable overlay identity.
    ///
    /// Returns:
    ///     A default overlay builder.
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            position: None,
            items: MoonMenuLevel::new([]),
            open: false,
            width: 180.0,
            max_height: None,
            on_dismiss: None,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self.position = None;
        self
    }

    pub fn position(mut self, position: Point<Pixels>) -> Self {
        self.position = Some(position);
        self.bounds = None;
        self
    }

    /// Append one overlay menu row without a later layout scan.
    ///
    /// Args:
    ///     item: Row to append.
    ///
    /// Returns:
    ///     The updated overlay.
    pub fn item(mut self, item: MoonMenuItem) -> Self {
        self.items.extend([item]);
        self
    }

    /// Append overlay menu rows while accumulating their layout signature.
    ///
    /// Args:
    ///     items: Rows in display order.
    ///
    /// Returns:
    ///     The updated overlay.
    pub fn items(mut self, items: impl IntoIterator<Item = MoonMenuItem>) -> Self {
        self.items.extend(items);
        self
    }

    /// Install a retained menu level without cloning its rows.
    ///
    /// Args:
    ///     items: Shared row storage and layout signature.
    ///
    /// Returns:
    ///     The updated overlay.
    fn shared_items(mut self, items: MoonMenuLevel) -> Self {
        self.items = items;
        self
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn max_height(mut self, max_height: f32) -> Self {
        self.max_height = Some(max_height);
        self
    }

    pub fn on_dismiss(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_dismiss = Some(Rc::new(handler));
        self
    }
}

impl MoonContextMenuWindowExt for Window {
    /// Open a root-owned context menu using shared retained row storage.
    ///
    /// Args:
    ///     cx: Application context used to install the overlay.
    ///     id: Stable context-menu identity.
    ///     position: Requested viewport origin.
    ///     items: Menu rows in display order.
    ///     width: Rendered menu width.
    ///
    /// Returns:
    ///     Nothing; the menu is installed in the window overlay.
    fn open_moon_context_menu(
        &mut self,
        cx: &mut App,
        id: impl Into<SharedString>,
        position: Point<Pixels>,
        items: Vec<MoonMenuItem>,
        width: f32,
    ) {
        self.open_moon_context_menu_with_dismiss(cx, id, position, items, width, |window, cx| {
            window.close_context_menu(cx);
        });
    }

    /// Open a root-owned context menu with a custom dismissal callback.
    ///
    /// Args:
    ///     cx: Application context used to install the overlay.
    ///     id: Stable context-menu identity.
    ///     position: Requested viewport origin.
    ///     items: Menu rows captured once into shared retained storage.
    ///     width: Rendered menu width.
    ///     on_dismiss: Callback invoked after escape or an outside click.
    ///
    /// Returns:
    ///     Nothing; the menu is installed in the window overlay.
    fn open_moon_context_menu_with_dismiss(
        &mut self,
        cx: &mut App,
        id: impl Into<SharedString>,
        position: Point<Pixels>,
        items: Vec<MoonMenuItem>,
        width: f32,
        on_dismiss: impl Fn(&mut Window, &mut App) + 'static,
    ) {
        let id = id.into();
        let items = MoonMenuLevel::new(items);
        let on_dismiss = Rc::new(on_dismiss);
        self.open_context_menu(cx, move |_window, _cx| {
            let on_dismiss = on_dismiss.clone();
            MoonContextMenuOverlay::new(id.clone())
                .position(position)
                .shared_items(items.clone())
                .open(true)
                .width(width)
                .on_dismiss(move |window, cx| on_dismiss(window, cx))
                .into_any_element()
        });
    }
}

impl RenderOnce for MoonContextMenu {
    /// Render the clamped menu and delegate large-list handling to [`MoonPopupMenu`].
    ///
    /// Args:
    ///     window: Owning window used for viewport bounds and popup retained state.
    ///     _cx: Application context consumed by nested elements.
    ///
    /// Returns:
    ///     The positioned context-menu root.
    fn render(self, window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let viewport = window.viewport_size();
        let viewport_w = f32::from(viewport.width);
        let viewport_h = f32::from(viewport.height);
        let max_height = context_menu_max_height(viewport_h, self.max_height);
        let mut root = div()
            .id(ElementId::from(self.id.clone()))
            .absolute()
            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                cx.stop_propagation();
            })
            .on_mouse_down(MouseButton::Right, |_, _, cx| {
                cx.stop_propagation();
            });
        if let Some(position) = self.position {
            let (x, y) = context_menu_clamped_origin(
                viewport_w,
                viewport_h,
                f32::from(position.x),
                f32::from(position.y),
                self.width,
                max_height,
                self.items.len(),
            );
            root = root.left(px(x)).top(px(y));
        } else if let Some(bounds) = self.bounds {
            let (x, y) = context_menu_clamped_origin(
                viewport_w,
                viewport_h,
                bounds.x,
                bounds.y,
                self.width,
                max_height,
                self.items.len(),
            );
            root = root.left(px(x)).top(px(y));
        }
        if self.open {
            root = root.child(
                MoonPopupMenu::new(format!("{}:popup", self.id))
                    .shared_level(self.items)
                    .width(self.width)
                    .max_height(max_height)
                    .render(),
            );
        }
        root
    }
}

/// Resolve a context menu's outer height limit.
///
/// Args:
///     viewport_h: Current viewport height.
///     requested_max_height: Optional caller-supplied limit.
///
/// Returns:
///     The requested limit or the viewport-safe default.
fn context_menu_max_height(viewport_h: f32, requested_max_height: Option<f32>) -> f32 {
    let margin = 6.0;
    requested_max_height.unwrap_or((viewport_h - margin * 2.0).max(80.0))
}

/// Clamp a context-menu origin inside the viewport using its estimated bounded height.
///
/// Args:
///     viewport_w: Current viewport width.
///     viewport_h: Current viewport height.
///     requested_x: Requested horizontal origin.
///     requested_y: Requested vertical origin.
///     width: Rendered menu width.
///     max_height: Resolved outer height limit.
///     items: Number of menu rows.
///
/// Returns:
///     Viewport-safe `(x, y)` coordinates.
fn context_menu_clamped_origin(
    viewport_w: f32,
    viewport_h: f32,
    requested_x: f32,
    requested_y: f32,
    width: f32,
    max_height: f32,
    items: usize,
) -> (f32, f32) {
    let margin = 6.0;
    let estimated_height = context_menu_estimated_height(items).min(max_height);
    let x = requested_x
        .max(margin)
        .min((viewport_w - width - margin).max(margin));
    let y = requested_y
        .max(margin)
        .min((viewport_h - estimated_height - margin).max(margin));
    (x, y)
}

/// Estimate eager menu height for viewport clamping.
///
/// Args:
///     items: Number of rows.
///
/// Returns:
///     Estimated rendered height including gaps and menu chrome.
fn context_menu_estimated_height(items: usize) -> f32 {
    let rows = items as f32;
    let gaps = items.saturating_sub(1) as f32;
    8.0 + rows * 24.0 + gaps * 2.0
}

impl RenderOnce for MoonContextMenuOverlay {
    /// Render the root-owned dismiss layer and its shared positioned menu.
    ///
    /// Args:
    ///     window: Window used for retained focus and popup list state.
    ///     cx: Application context used for callbacks and retained state.
    ///
    /// Returns:
    ///     The full-window overlay, or an empty root while closed.
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id.clone();
        let mut root = div().id(ElementId::from(id.clone())).absolute();
        if !self.open {
            return root;
        }

        let focus = window
            .use_keyed_state(
                ElementId::from(SharedString::from(format!("{id}:focus"))),
                cx,
                |_, cx| cx.focus_handle().tab_stop(true),
            )
            .read(cx)
            .clone();
        focus.focus(window, cx);
        let on_dismiss = self.on_dismiss.clone();
        root = root
            .inset_0()
            .track_focus(&focus)
            .on_key_down({
                let on_dismiss = self.on_dismiss.clone();
                move |event: &KeyDownEvent, window, cx| {
                    if event.keystroke.key.as_str() == "escape" {
                        cx.stop_propagation();
                        if let Some(on_dismiss) = &on_dismiss {
                            on_dismiss(window, cx);
                        }
                    }
                }
            })
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                cx.stop_propagation();
                if let Some(on_dismiss) = &on_dismiss {
                    on_dismiss(window, cx);
                }
            })
            .on_mouse_down(MouseButton::Right, {
                let on_dismiss = self.on_dismiss.clone();
                move |_, window, cx| {
                    cx.stop_propagation();
                    if let Some(on_dismiss) = &on_dismiss {
                        on_dismiss(window, cx);
                    }
                }
            })
            .child({
                let mut menu = MoonContextMenu::new(format!("{id}:menu"));
                if let Some(position) = self.position {
                    menu = menu.position(position);
                } else {
                    menu = menu.bounds(self.bounds.unwrap_or(MoonRect::new(0.0, 0.0, 0.0, 0.0)));
                }
                let menu = menu.shared_items(self.items).open(true).width(self.width);
                if let Some(max_height) = self.max_height {
                    menu.max_height(max_height)
                } else {
                    menu
                }
            });
        root
    }
}

#[cfg(test)]
mod tests;
