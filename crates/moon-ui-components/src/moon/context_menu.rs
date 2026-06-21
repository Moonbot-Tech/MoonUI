use std::rc::Rc;

use crate::WindowExt as _;
use gpui::*;

use super::{
    dropdown::{MoonMenuItem, MoonPopupMenu},
    tokens::{MoonPalette, MoonRect},
};

#[derive(IntoElement)]
pub struct MoonContextMenu {
    id: SharedString,
    bounds: Option<MoonRect>,
    position: Option<Point<Pixels>>,
    items: Vec<MoonMenuItem>,
    open: bool,
    width: f32,
    max_height: Option<f32>,
}

type MoonContextDismissHandler = Rc<dyn Fn(&mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct MoonContextMenuOverlay {
    id: SharedString,
    bounds: Option<MoonRect>,
    position: Option<Point<Pixels>>,
    items: Vec<MoonMenuItem>,
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
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            position: None,
            items: Vec::new(),
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

    pub fn item(mut self, item: MoonMenuItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn items(mut self, items: impl IntoIterator<Item = MoonMenuItem>) -> Self {
        self.items.extend(items);
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
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            position: None,
            items: Vec::new(),
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

    pub fn item(mut self, item: MoonMenuItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn items(mut self, items: impl IntoIterator<Item = MoonMenuItem>) -> Self {
        self.items.extend(items);
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
        let on_dismiss = Rc::new(on_dismiss);
        self.open_context_menu(cx, move |_window, _cx| {
            let on_dismiss = on_dismiss.clone();
            MoonContextMenuOverlay::new(id.clone())
                .position(position)
                .items(items.clone())
                .open(true)
                .width(width)
                .on_dismiss(move |window, cx| on_dismiss(window, cx))
                .into_any_element()
        });
    }
}

impl RenderOnce for MoonContextMenu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = MoonPalette::active(cx);
        let viewport = window.viewport_size();
        let viewport_w = f32::from(viewport.width);
        let viewport_h = f32::from(viewport.height);
        let margin = 6.0;
        let max_height = self
            .max_height
            .unwrap_or((viewport_h - margin * 2.0).max(80.0));
        let estimated_height = context_menu_estimated_height(self.items.len()).min(max_height);
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
            let x = f32::from(position.x)
                .max(margin)
                .min((viewport_w - self.width - margin).max(margin));
            let y = f32::from(position.y)
                .max(margin)
                .min((viewport_h - estimated_height - margin).max(margin));
            root = root.left(px(x)).top(px(y));
        } else if let Some(bounds) = self.bounds {
            let x = bounds
                .x
                .max(margin)
                .min((viewport_w - self.width - margin).max(margin));
            let y = bounds
                .y
                .max(margin)
                .min((viewport_h - estimated_height - margin).max(margin));
            root = root.left(px(x)).top(px(y));
        }
        if self.open {
            root = root.child(
                MoonPopupMenu::new(format!("{}:popup", self.id))
                    .items(self.items)
                    .width(self.width)
                    .max_height(max_height)
                    .render_with_palette(p),
            );
        }
        root
    }
}

fn context_menu_estimated_height(items: usize) -> f32 {
    let rows = items as f32;
    let gaps = items.saturating_sub(1) as f32;
    8.0 + rows * 24.0 + gaps * 2.0
}

impl RenderOnce for MoonContextMenuOverlay {
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
                let menu = menu.items(self.items).open(true).width(self.width);
                if let Some(max_height) = self.max_height {
                    menu.max_height(max_height)
                } else {
                    menu
                }
            });
        root
    }
}
