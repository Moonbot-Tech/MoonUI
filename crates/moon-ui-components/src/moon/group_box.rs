use gpui::*;

use super::{
    background::MoonBackgroundPolicy,
    text::MoonText,
    theme::MoonTheme,
    tokens::{MoonRect, rgba_from},
};

#[derive(IntoElement)]
pub struct MoonGroupBox {
    id: SharedString,
    bounds: Option<MoonRect>,
    title: Option<SharedString>,
    children: Vec<AnyElement>,
    background_policy: MoonBackgroundPolicy,
    padding: f32,
    gap: f32,
}

impl MoonGroupBox {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            title: None,
            children: Vec::new(),
            background_policy: MoonBackgroundPolicy::Opaque,
            padding: 10.0,
            gap: 8.0,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn background_policy(mut self, policy: MoonBackgroundPolicy) -> Self {
        self.background_policy = policy;
        self
    }

    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }
}

impl ParentElement for MoonGroupBox {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for MoonGroupBox {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        let mut root = div()
            .id(ElementId::from(self.id))
            .relative()
            .rounded(px(tokens.ui(5.0)))
            .border(px(tokens.ui(1.0)))
            .border_color(rgba_from(p.border, 1.0))
            .p(px(tokens.ui(self.padding)))
            .flex()
            .flex_col()
            .gap(px(tokens.ui(self.gap)));
        root = self.background_policy.apply(root, p.shell_high, 0.98);

        if let Some(bounds) = self.bounds {
            root = root
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }

        if let Some(title) = self.title {
            root = root.child(
                MoonText::new(title)
                    .color(p.text_soft)
                    .font_size(10.5)
                    .line_height(13.0)
                    .weight(600.0)
                    .mono(true)
                    .uppercase(false)
                    .render(),
            );
        }

        root.children(self.children)
    }
}
