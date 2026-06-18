use gpui::*;

use super::{
    dropdown::{MoonMenuItem, MoonPopupMenu},
    tokens::{MoonPalette, MoonRect},
};

#[derive(IntoElement)]
pub struct MoonContextMenu {
    id: SharedString,
    bounds: Option<MoonRect>,
    items: Vec<MoonMenuItem>,
    open: bool,
    width: f32,
}

impl MoonContextMenu {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            items: Vec::new(),
            open: false,
            width: 180.0,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
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
}

impl RenderOnce for MoonContextMenu {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = MoonPalette::active(cx);
        let mut root = div().id(ElementId::from(self.id.clone())).absolute();
        if let Some(bounds) = self.bounds {
            root = root.left(px(bounds.x)).top(px(bounds.y));
        }
        if self.open {
            root = root.child(
                MoonPopupMenu::new(format!("{}:popup", self.id))
                    .items(self.items)
                    .width(self.width)
                    .render_with_palette(p),
            );
        }
        root
    }
}
