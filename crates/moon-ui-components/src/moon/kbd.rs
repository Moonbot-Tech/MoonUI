//! Monospaced shortcut chips sized to the active density tier.

use gpui::*;

use super::{
    foundation::MoonSize,
    theme::{MoonTheme, MoonThemeTokens},
    tokens::{MoonRect, rgba_from},
};

/// A shared text-height tier or custom legacy chip metrics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MoonKbdSize {
    /// Supports Xs, Sm and Md; larger tiers snap to Md.
    Tier(MoonSize),
    Custom {
        height: f32,
        font_size: f32,
        line_height: f32,
        radius: f32,
        pad_x: f32,
    },
}

impl From<MoonSize> for MoonKbdSize {
    /// Wraps a shared tier; metrics snap unsupported sizes when rendered.
    fn from(size: MoonSize) -> Self {
        Self::Tier(size)
    }
}

/// Final pixel metrics; only Custom text follows legacy font scaling.
#[derive(Clone, Copy, Debug)]
struct KbdMetrics {
    height: f32,
    font_size: f32,
    line_height: f32,
    radius: f32,
    pad_x: f32,
}

/// A shortcut chip whose omitted size follows the active density.
#[derive(IntoElement)]
pub struct MoonKbd {
    bounds: Option<MoonRect>,
    label: SharedString,
    size: Option<MoonKbdSize>,
    outline: bool,
}

impl MoonKbd {
    /// Creates a shortcut chip with a density-selected size.
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            bounds: None,
            label: label.into(),
            size: None,
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
            // A shortcut that IS one modifier reaches here under `Keystroke::parse`'s own name for
            // it ("control", not "ctrl"), which the fallback below would spell out as written.
            #[cfg(target_os = "macos")]
            "control" => keys.push('⌃'),
            #[cfg(not(target_os = "macos"))]
            "control" => keys.push_str("Ctrl"),
            #[cfg(target_os = "macos")]
            "platform" => keys.push('⌘'),
            #[cfg(not(target_os = "macos"))]
            "platform" => keys.push_str("Win"),
            "function" => keys.push_str("Fn"),
            #[cfg(target_os = "macos")]
            "capslock" => keys.push('⇪'),
            #[cfg(not(target_os = "macos"))]
            "capslock" => keys.push_str("Caps Lock"),
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

    /// Overrides density with a tier (via Into) or custom metrics.
    pub fn size(mut self, size: MoonKbdSize) -> Self {
        self.size = Some(size);
        self
    }

    pub fn outline(mut self, outline: bool) -> Self {
        self.outline = outline;
        self
    }

    /// Resolves final pixels, using text-line height instead of pointer-target height.
    fn metrics(&self, tokens: &MoonThemeTokens) -> KbdMetrics {
        match self.size.unwrap_or(MoonKbdSize::Tier(tokens.tier())) {
            MoonKbdSize::Tier(tier) => {
                let tier = tier.nearest(&[MoonSize::Xs, MoonSize::Sm, MoonSize::Md]);
                let control = tier.control_metrics();
                let font_size = match tier {
                    MoonSize::Xs => 11.0,
                    MoonSize::Sm => 12.0,
                    _ => 14.0,
                };
                KbdMetrics {
                    height: tokens.ui(control.line_height),
                    font_size: tokens.ui(font_size),
                    line_height: tokens.ui(control.line_height),
                    radius: tokens.ui(control.radius * (2.0 / 3.0)),
                    pad_x: tokens.ui(control.pad_x * (2.0 / 3.0)),
                }
            }
            MoonKbdSize::Custom {
                height,
                font_size,
                line_height,
                radius,
                pad_x,
            } => KbdMetrics {
                height: tokens.ui(height),
                font_size: tokens.font(font_size),
                line_height: tokens.line_height(line_height),
                radius: tokens.ui(radius),
                pad_x: tokens.ui(pad_x),
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
    /// Renders resolved text and chrome without double scaling.
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        let metrics = self.metrics(&tokens);
        let mut root = div()
            .relative()
            .h(px(metrics.height))
            .px(px(metrics.pad_x))
            .rounded(px(metrics.radius))
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
                div()
                    .font_family(tokens.font_family(true))
                    .text_color(rgba_from(p.text_soft, 1.0))
                    .text_size(px(metrics.font_size))
                    .line_height(px(metrics.line_height))
                    .font_weight(FontWeight::SEMIBOLD)
                    .whitespace_nowrap()
                    .child(self.label),
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
mod tests;
