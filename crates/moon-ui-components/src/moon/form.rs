use gpui::*;

use super::{text::MoonText, theme::MoonTheme, tokens::MoonRect};

#[derive(IntoElement)]
pub struct MoonFormRow {
    id: SharedString,
    bounds: Option<MoonRect>,
    label: SharedString,
    label_width: f32,
    height: f32,
    control: Option<AnyElement>,
    disabled: bool,
}

impl MoonFormRow {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            label: label.into(),
            label_width: 150.0,
            height: 30.0,
            control: None,
            disabled: false,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn label_width(mut self, width: f32) -> Self {
        self.label_width = width;
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn control(mut self, control: impl IntoElement) -> Self {
        self.control = Some(control.into_any_element());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl RenderOnce for MoonFormRow {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        let mut row = div()
            .id(ElementId::from(self.id))
            .relative()
            .h(px(tokens.ui(self.height)))
            .w_full()
            .flex()
            .items_center()
            .gap(px(tokens.ui(10.0)))
            .child(
                div().w(px(tokens.ui(self.label_width))).child(
                    MoonText::new(self.label)
                        .color(p.text_soft)
                        .alpha(if self.disabled { 0.45 } else { 1.0 })
                        .font_size(10.5)
                        .line_height(13.0)
                        .weight(500.0)
                        .mono(true)
                        .uppercase(false)
                        .render(),
                ),
            )
            .child(
                div()
                    .relative()
                    .flex_1()
                    .child(self.control.unwrap_or_else(|| div().into_any_element())),
            );

        if let Some(bounds) = self.bounds {
            row = row
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }

        row
    }
}
