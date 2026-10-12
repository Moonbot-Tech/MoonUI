use gpui::*;

use crate::h_flex;

use super::{
    theme::{MoonTheme, MoonThemeTokens},
    tokens::{MoonPalette, MoonTone, contrast_ratio, rgba_from},
};

/// The WCAG 2.x contrast floor a [`MoonTagVariant::Soft`] or [`MoonTagVariant::Filled`] label
/// must clear against the colour it is painted on.
///
/// AA for normal-size text: the label renders at 11px, below the large-text threshold.
const LABEL_CONTRAST_FLOOR: f32 = 4.5;

/// Opacity of the tone wash behind a [`MoonTagVariant::Soft`] label.
const SOFT_FILL_ALPHA: f32 = 0.14;

/// How a [`MoonTag`] paints its tone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MoonTagVariant {
    /// The original look: a tone-coloured border and label on a transparent fill.
    #[default]
    Outline,
    /// A faint tone wash behind a tone-coloured label, with no visible border — a status chip.
    Soft,
    /// A solid tone fill with the palette ink that reads best on it.
    ///
    /// Positive, Warning, Info and Danger clear the label contrast floor on the light palette;
    /// on the dark terminal palette only Warning and Info do, so prefer `Soft` there for the
    /// green and red tones.
    Filled,
}

/// Corner shape of a [`MoonTag`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MoonTagRadius {
    /// The original pill shape.
    #[default]
    Full,
    /// A square-ish 3px corner, matching inline status chips.
    Small,
}

#[derive(IntoElement)]
pub struct MoonTag {
    tone: MoonTone,
    variant: MoonTagVariant,
    radius: MoonTagRadius,
    compact: bool,
    mono: bool,
    children: Vec<AnyElement>,
}

impl MoonTag {
    pub fn new() -> Self {
        Self {
            tone: MoonTone::Default,
            variant: MoonTagVariant::Outline,
            radius: MoonTagRadius::Full,
            compact: false,
            mono: true,
            children: Vec::new(),
        }
    }

    pub fn positive() -> Self {
        Self::new().tone(MoonTone::Positive)
    }

    pub fn warning() -> Self {
        Self::new().tone(MoonTone::Warning)
    }

    pub fn danger() -> Self {
        Self::new().tone(MoonTone::Danger)
    }

    pub fn info() -> Self {
        Self::new().tone(MoonTone::Info)
    }

    pub fn tone(mut self, tone: MoonTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn outline(mut self) -> Self {
        self.variant = MoonTagVariant::Outline;
        self
    }

    pub fn rounded_full(mut self) -> Self {
        self.radius = MoonTagRadius::Full;
        self
    }

    /// Choose how the tone is painted: outline (default), soft wash, or solid fill.
    pub fn variant(mut self, variant: MoonTagVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Choose the corner shape: pill (default) or a small square-ish radius.
    pub fn radius(mut self, radius: MoonTagRadius) -> Self {
        self.radius = radius;
        self
    }

    /// Tighten the padding and height to an inline status-chip size. Off by default.
    pub fn compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }

    pub fn mono(mut self, mono: bool) -> Self {
        self.mono = mono;
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        let label: SharedString = label.into();
        self.children
            .push(label.to_string().to_uppercase().into_any_element());
        self
    }

    pub fn render(self) -> impl IntoElement {
        self
    }
}

impl ParentElement for MoonTag {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

/// Resolved paint and geometry of one tag, computed apart from rendering so it can be tested.
#[derive(Clone, Copy, Debug, PartialEq)]
struct TagChrome {
    border: Hsla,
    background: Hsla,
    /// Opaque `0xRRGGBB` label colour.
    text: u32,
    radius: f32,
    pad_x: f32,
    min_h: f32,
}

/// Blend `fg` over `bg` at `alpha` into the opaque `0xRRGGBB` colour the eye sees.
///
/// Args:
///     fg: Foreground colour.
///     bg: Opaque background colour.
///     alpha: Foreground opacity from 0.0 to 1.0.
///
/// Returns:
///     The composited colour, rounded per channel.
fn composite(fg: u32, bg: u32, alpha: f32) -> u32 {
    let channel = |shift: u32| {
        let f = ((fg >> shift) & 0xFF) as f32;
        let b = ((bg >> shift) & 0xFF) as f32;
        ((f * alpha + b * (1.0 - alpha)).round() as u32) << shift
    };
    channel(16) | channel(8) | channel(0)
}

/// The label colour for a soft tag: the first of the tone colour, the palette's readable
/// companion of that hue, and the body text that clears [`LABEL_CONTRAST_FLOOR`] on `fill`.
///
/// Args:
///     tone: The tag's tone.
///     color: The tone's resolved colour.
///     fill: The opaque composited wash behind the label.
///     p: Palette the tag paints with.
///
/// Returns:
///     The label colour; body text when nothing earlier clears the floor.
fn soft_label(tone: MoonTone, color: u32, fill: u32, p: MoonPalette) -> u32 {
    let companion = match tone {
        MoonTone::Positive => Some(p.green_text),
        MoonTone::Negative | MoonTone::Danger => Some(p.red_text),
        _ => None,
    };
    [Some(color), companion]
        .into_iter()
        .flatten()
        .find(|candidate| contrast_ratio(*candidate, fill) >= LABEL_CONTRAST_FLOOR)
        .unwrap_or(p.text)
}

/// Resolve a tag's paint and geometry.
///
/// The outline variant reproduces the original tag exactly; the label contrast of the soft and
/// filled variants is measured against the panel the tag sits on with its fill composited over it.
///
/// Args:
///     tokens: Active theme tokens (scale and palette).
///     tone: The tag's tone.
///     variant: How the tone is painted.
///     radius: Corner shape.
///     compact: Whether the inline status-chip geometry applies.
///
/// Returns:
///     The resolved [`TagChrome`].
fn tag_chrome(
    tokens: &MoonThemeTokens,
    tone: MoonTone,
    variant: MoonTagVariant,
    radius: MoonTagRadius,
    compact: bool,
) -> TagChrome {
    let p = tokens.palette;
    let color = tone.color(p);
    let (border, background, text) = match variant {
        MoonTagVariant::Outline => (rgba_from(color, 0.48), rgba_from(color, 0.0), color),
        MoonTagVariant::Soft => {
            let fill = composite(color, p.panel, SOFT_FILL_ALPHA);
            (
                rgba_from(color, 0.0),
                rgba_from(color, SOFT_FILL_ALPHA),
                soft_label(tone, color, fill, p),
            )
        }
        MoonTagVariant::Filled => (
            rgba_from(color, 1.0),
            rgba_from(color, 1.0),
            p.ink_on(color),
        ),
    };
    let radius = match radius {
        MoonTagRadius::Full => 999.0,
        MoonTagRadius::Small => 3.0,
    };
    let (pad_x, min_h) = if compact {
        (tokens.ui(7.0), tokens.fit_height(18.0, 14.0, 2.0))
    } else {
        (tokens.ui(10.0), tokens.fit_height(22.0, 14.0, 4.0))
    };
    TagChrome {
        border,
        background,
        text,
        radius: tokens.ui(radius),
        pad_x,
        min_h,
    }
}

impl RenderOnce for MoonTag {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let text = tokens.text(11.0, 14.0);
        let chrome = tag_chrome(&tokens, self.tone, self.variant, self.radius, self.compact);

        h_flex()
            .flex_none()
            .items_center()
            .gap(px(tokens.ui(if self.compact { 5.0 } else { 6.0 })))
            .min_h(px(chrome.min_h))
            .px(px(chrome.pad_x))
            .rounded(px(chrome.radius))
            .border_1()
            .border_color(chrome.border)
            .bg(chrome.background)
            .text_color(rgba_from(chrome.text, 1.0))
            .font_family(tokens.font_family(self.mono))
            .text_size(px(text.font_size))
            .line_height(px(text.line_height))
            .font_weight(FontWeight::MEDIUM)
            .children(self.children)
    }
}

#[cfg(test)]
mod tests;
