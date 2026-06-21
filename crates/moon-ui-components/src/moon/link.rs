use gpui::prelude::FluentBuilder;
use gpui::*;

use super::{
    text::MoonText,
    theme::MoonTheme,
    tokens::{MoonRect, MoonTone, rgba_from},
};

#[derive(IntoElement)]
pub struct MoonLink {
    id: SharedString,
    bounds: Option<MoonRect>,
    label: SharedString,
    tone: MoonTone,
    disabled: bool,
    mono: bool,
    font_size: f32,
    line_height: f32,
    underline: bool,
    on_click: Option<std::rc::Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
}

impl MoonLink {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            label: label.into(),
            tone: MoonTone::Info,
            disabled: false,
            mono: true,
            font_size: 10.5,
            line_height: 13.0,
            underline: true,
            on_click: None,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn tone(mut self, tone: MoonTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn mono(mut self, mono: bool) -> Self {
        self.mono = mono;
        self
    }

    pub fn font_size(mut self, font_size: f32) -> Self {
        self.font_size = font_size;
        self
    }

    pub fn line_height(mut self, line_height: f32) -> Self {
        self.line_height = line_height;
        self
    }

    pub fn underline(mut self, underline: bool) -> Self {
        self.underline = underline;
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(std::rc::Rc::new(handler));
        self
    }
}

impl RenderOnce for MoonLink {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        let color = if self.disabled {
            p.text_muted
        } else {
            self.tone.color(p)
        };
        let alpha = if self.disabled { 0.48 } else { 1.0 };
        let underline = self.underline;
        let disabled = self.disabled;
        let text = tokens.text(self.font_size, self.line_height);
        let mut root = div()
            .id(ElementId::from(self.id))
            .relative()
            .flex()
            .items_center()
            .h(px(text.line_height + tokens.ui(2.0)))
            .cursor_default()
            .when(!disabled, |this| {
                this.cursor_pointer()
                    .hover(move |this| this.bg(rgba_from(color, 0.07)))
                    .active(move |this| this.bg(rgba_from(color, 0.045)))
            })
            .child(
                MoonText::new(self.label)
                    .color(color)
                    .alpha(alpha)
                    .font_size(text.font_size)
                    .line_height(text.line_height)
                    .weight(500.0)
                    .mono(self.mono)
                    .uppercase(false)
                    .render(),
            )
            .when(underline, |this| {
                this.child(
                    div()
                        .absolute()
                        .left(px(0.0))
                        .right(px(0.0))
                        .bottom(px(0.0))
                        .h(px(tokens.ui(1.0)))
                        .bg(rgba_from(color, 0.40 * alpha)),
                )
            });

        if let Some(bounds) = self.bounds {
            root = root
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }

        root = root.on_mouse_down(MouseButton::Left, move |_, window, cx| {
            cx.stop_propagation();
            if disabled {
                return;
            }
            window.prevent_default();
        });

        if let Some(on_click) = self.on_click {
            root = root.on_click(move |event, window, cx| {
                if disabled {
                    cx.stop_propagation();
                    return;
                }
                on_click(event, window, cx);
            });
        }

        root
    }
}
