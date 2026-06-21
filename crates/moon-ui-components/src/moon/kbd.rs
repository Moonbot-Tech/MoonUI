use gpui::*;

use super::{
    text::MoonText,
    theme::MoonTheme,
    tokens::{MoonRect, rgba_from},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MoonKbdSize {
    Compact,
    Normal,
    Custom {
        height: f32,
        font_size: f32,
        line_height: f32,
        radius: f32,
        pad_x: f32,
    },
}

#[derive(Clone, Copy, Debug)]
struct KbdMetrics {
    height: f32,
    font_size: f32,
    line_height: f32,
    radius: f32,
    pad_x: f32,
}

#[derive(IntoElement)]
pub struct MoonKbd {
    bounds: Option<MoonRect>,
    label: SharedString,
    size: MoonKbdSize,
    outline: bool,
}

impl MoonKbd {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            bounds: None,
            label: label.into(),
            size: MoonKbdSize::Normal,
            outline: false,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn size(mut self, size: MoonKbdSize) -> Self {
        self.size = size;
        self
    }

    pub fn outline(mut self, outline: bool) -> Self {
        self.outline = outline;
        self
    }

    fn metrics(&self) -> KbdMetrics {
        match self.size {
            MoonKbdSize::Compact => KbdMetrics {
                height: 17.0,
                font_size: 8.5,
                line_height: 11.0,
                radius: 3.0,
                pad_x: 5.0,
            },
            MoonKbdSize::Normal => KbdMetrics {
                height: 20.0,
                font_size: 9.5,
                line_height: 12.0,
                radius: 4.0,
                pad_x: 6.0,
            },
            MoonKbdSize::Custom {
                height,
                font_size,
                line_height,
                radius,
                pad_x,
            } => KbdMetrics {
                height,
                font_size,
                line_height,
                radius,
                pad_x,
            },
        }
    }
}

impl RenderOnce for MoonKbd {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        let metrics = self.metrics();
        let text = tokens.text(metrics.font_size, metrics.line_height);
        let mut root = div()
            .relative()
            .h(px(tokens.ui(metrics.height)))
            .px(px(tokens.ui(metrics.pad_x)))
            .rounded(px(tokens.ui(metrics.radius)))
            .border(px(tokens.ui(1.0)))
            .border_color(rgba_from(p.border, if self.outline { 1.0 } else { 0.72 }))
            .bg(rgba_from(
                if self.outline { p.shell } else { p.panel },
                if self.outline { 0.0 } else { 0.92 },
            ))
            .flex()
            .items_center()
            .justify_center()
            .child(
                MoonText::new(self.label)
                    .color(p.text_soft)
                    .font_size(text.font_size)
                    .line_height(text.line_height)
                    .weight(600.0)
                    .mono(true)
                    .uppercase(false)
                    .render(),
            );

        if let Some(bounds) = self.bounds {
            root = root
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }

        root
    }
}

#[cfg(test)]
mod tests {
    use super::{MoonKbd, MoonKbdSize};

    #[test]
    fn kbd_metrics_match_designer_reference() {
        let compact = MoonKbd::new("Esc").size(MoonKbdSize::Compact);
        assert_eq!(compact.metrics().height, 17.0);
        let normal = MoonKbd::new("Ctrl+K");
        assert_eq!(normal.metrics().height, 20.0);
    }
}
