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

    pub fn from_keystroke(stroke: Keystroke) -> Self {
        Self::new(Self::format_keystroke(&stroke))
    }

    pub fn format_keystroke(key: &Keystroke) -> String {
        #[cfg(target_os = "macos")]
        const SEPARATOR: &str = "";
        #[cfg(not(target_os = "macos"))]
        const SEPARATOR: &str = "+";

        let mut parts = Vec::new();

        if key.modifiers.control {
            #[cfg(target_os = "macos")]
            parts.push("⌃");
            #[cfg(not(target_os = "macos"))]
            parts.push("Ctrl");
        }

        if key.modifiers.alt {
            #[cfg(target_os = "macos")]
            parts.push("⌥");
            #[cfg(not(target_os = "macos"))]
            parts.push("Alt");
        }

        if key.modifiers.shift {
            #[cfg(target_os = "macos")]
            parts.push("⇧");
            #[cfg(not(target_os = "macos"))]
            parts.push("Shift");
        }

        if key.modifiers.platform {
            #[cfg(target_os = "macos")]
            parts.push("⌘");
            #[cfg(not(target_os = "macos"))]
            parts.push("Win");
        }

        let mut keys = String::new();
        let key_str = key.key.as_str();
        match key_str {
            #[cfg(target_os = "macos")]
            "ctrl" => keys.push('⌃'),
            #[cfg(not(target_os = "macos"))]
            "ctrl" => keys.push_str("Ctrl"),
            #[cfg(target_os = "macos")]
            "alt" => keys.push('⌥'),
            #[cfg(not(target_os = "macos"))]
            "alt" => keys.push_str("Alt"),
            #[cfg(target_os = "macos")]
            "shift" => keys.push('⇧'),
            #[cfg(not(target_os = "macos"))]
            "shift" => keys.push_str("Shift"),
            #[cfg(target_os = "macos")]
            "cmd" => keys.push('⌘'),
            #[cfg(not(target_os = "macos"))]
            "cmd" => keys.push_str("Win"),
            #[cfg(target_os = "macos")]
            "space" => keys.push_str("Space"),
            #[cfg(target_os = "macos")]
            "backspace" => keys.push('⌫'),
            #[cfg(not(target_os = "macos"))]
            "backspace" => keys.push_str("Backspace"),
            #[cfg(target_os = "macos")]
            "delete" => keys.push('⌫'),
            #[cfg(not(target_os = "macos"))]
            "delete" => keys.push_str("Delete"),
            #[cfg(target_os = "macos")]
            "escape" => keys.push('⎋'),
            #[cfg(not(target_os = "macos"))]
            "escape" => keys.push_str("Esc"),
            #[cfg(target_os = "macos")]
            "enter" => keys.push('⏎'),
            #[cfg(not(target_os = "macos"))]
            "enter" => keys.push_str("Enter"),
            "pagedown" => keys.push_str("Page Down"),
            "pageup" => keys.push_str("Page Up"),
            #[cfg(target_os = "macos")]
            "left" => keys.push('←'),
            #[cfg(not(target_os = "macos"))]
            "left" => keys.push_str("Left"),
            #[cfg(target_os = "macos")]
            "right" => keys.push('→'),
            #[cfg(not(target_os = "macos"))]
            "right" => keys.push_str("Right"),
            #[cfg(target_os = "macos")]
            "up" => keys.push('↑'),
            #[cfg(not(target_os = "macos"))]
            "up" => keys.push_str("Up"),
            #[cfg(target_os = "macos")]
            "down" => keys.push('↓'),
            #[cfg(not(target_os = "macos"))]
            "down" => keys.push_str("Down"),
            _ => {
                if key_str.len() == 1 {
                    keys.push_str(&key_str.to_uppercase());
                } else {
                    let mut chars = key_str.chars();
                    if let Some(first_char) = chars.next() {
                        keys.push_str(&format!(
                            "{}{}",
                            first_char.to_uppercase(),
                            chars.collect::<String>()
                        ));
                    } else {
                        keys.push_str(key_str);
                    }
                }
            }
        }

        parts.push(&keys);
        parts.join(SEPARATOR)
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

impl From<Keystroke> for MoonKbd {
    fn from(stroke: Keystroke) -> Self {
        Self::from_keystroke(stroke)
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
    use gpui::Keystroke;

    #[test]
    fn kbd_metrics_match_designer_reference() {
        let compact = MoonKbd::new("Esc").size(MoonKbdSize::Compact);
        assert_eq!(compact.metrics().height, 17.0);
        let normal = MoonKbd::new("Ctrl+K");
        assert_eq!(normal.metrics().height, 20.0);
    }

    #[test]
    fn kbd_formats_keystrokes_like_longbridge() {
        #[cfg(target_os = "macos")]
        {
            assert_eq!(
                MoonKbd::format_keystroke(&Keystroke::parse("cmd-enter").unwrap()),
                "⌘⏎"
            );
            assert_eq!(
                MoonKbd::format_keystroke(&Keystroke::parse("cmd-ctrl-shift-a").unwrap()),
                "⌃⇧⌘A"
            );
        }

        #[cfg(not(target_os = "macos"))]
        {
            assert_eq!(
                MoonKbd::format_keystroke(&Keystroke::parse("ctrl-a").unwrap()),
                "Ctrl+A"
            );
            assert_eq!(
                MoonKbd::format_keystroke(&Keystroke::parse("ctrl-alt-shift-a").unwrap()),
                "Ctrl+Alt+Shift+A"
            );
        }
    }
}
