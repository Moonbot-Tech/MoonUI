use gpui::prelude::FluentBuilder as _;
use gpui::*;
use std::rc::Rc;

use crate::h_flex;

use super::{
    foundation::{MoonClickHandler, box_shadow},
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

/// Opacity of the tone-coloured border of a toned [`MoonTagVariant::Tinted`] chip.
const TINTED_BORDER_ALPHA: f32 = 0.35;

/// Opacity of a whole tag dimmed by [`MoonTag::severity_count`] for a zero count.
///
/// A dimmed tag draws its label in the body text, not its quiet label colour: `text_soft` at this
/// opacity falls to about 3:1 on the panel, while the body text stays above the AA floor.
const DIMMED_OPACITY: f32 = 0.65;

/// Design-reference width of the accent ring around a selected clickable tag.
const SELECTED_RING_WIDTH: f32 = 2.0;

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
    /// A severity chip: a faint tone wash with a tone-tinted border, a quiet label and a count in
    /// the tone's readable colour. The neutral [`MoonTone::Default`] chip keeps the plain panel
    /// border and no wash, so a zero count reads as calm next to the coloured ones.
    Tinted,
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
    count: Option<SharedString>,
    dimmed: bool,
    selected: bool,
    on_click: Option<(ElementId, MoonClickHandler)>,
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
            count: None,
            dimmed: false,
            selected: false,
            on_click: None,
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

    /// Show a count after the tag's content, in a mono semibold face and the tone's readable
    /// colour, as in the health-strip chip `Offline 2`. No count by default.
    ///
    /// Args:
    ///     count: Count text, drawn as given.
    ///
    /// Returns:
    ///     The updated tag.
    pub fn count(mut self, count: impl Into<SharedString>) -> Self {
        self.count = Some(count.into());
        self
    }

    /// Show `count` and let it pick the severity: `tone` while it is non-zero, and a neutral,
    /// dimmed tag when it is zero.
    ///
    /// Pair it with [`MoonTagVariant::Tinted`] for the health-strip chip, whose wash, border and
    /// count colour then follow the severity on their own.
    ///
    /// Args:
    ///     count: The number to show.
    ///     tone: Tone for a non-zero count, such as [`MoonTone::Danger`] or [`MoonTone::Warning`].
    ///
    /// Returns:
    ///     The updated tag.
    pub fn severity_count(mut self, count: usize, tone: MoonTone) -> Self {
        self.count = Some(count.to_string().into());
        self.tone = if count == 0 { MoonTone::Default } else { tone };
        self.dimmed = count == 0;
        self
    }

    /// Dim the whole tag to a quiet, still-readable state. Off by default.
    ///
    /// Args:
    ///     dimmed: Whether the tag is dimmed.
    ///
    /// Returns:
    ///     The updated tag.
    pub fn dimmed(mut self, dimmed: bool) -> Self {
        self.dimmed = dimmed;
        self
    }

    /// Mark a tag as the active filter with an accent ring around it. Off by default.
    ///
    /// Args:
    ///     selected: Whether the tag is selected.
    ///
    /// Returns:
    ///     The updated tag.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Make the tag clickable: a pointer cursor, a hover border, and `handler` on click.
    ///
    /// Args:
    ///     id: Stable element identity the click is tracked under.
    ///     handler: Click callback.
    ///
    /// Returns:
    ///     The updated tag.
    pub fn on_click(
        mut self,
        id: impl Into<ElementId>,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some((id.into(), Rc::new(handler)));
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

/// The opaque tone wash and its readable label, as a soft tag paints them on the panel.
///
/// Shared by every Moon count badge that sits on a soft tone wash, so a segment count pill and a
/// soft tag of the same tone are the same two colours. The fill is returned opaque (already
/// composited over the panel), which lets a caller paint it on any surface — including a solid
/// accent fill — without the label's measured contrast changing.
///
/// Args:
///     tone: Tone of the wash.
///     p: Palette to paint with.
///
/// Returns:
///     `(fill, label)` as opaque `0xRRGGBB` colours.
pub(super) fn tone_wash(tone: MoonTone, p: MoonPalette) -> (u32, u32) {
    let color = tone.color(p);
    let fill = composite(color, p.panel, SOFT_FILL_ALPHA);
    (fill, soft_label(tone, color, fill, p))
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
        MoonTagVariant::Soft => (
            rgba_from(color, 0.0),
            rgba_from(color, SOFT_FILL_ALPHA),
            tone_wash(tone, p).1,
        ),
        MoonTagVariant::Filled => (
            rgba_from(color, 1.0),
            rgba_from(color, 1.0),
            p.ink_on(color),
        ),
        MoonTagVariant::Tinted if tone == MoonTone::Default => (
            rgba_from(p.border, 1.0),
            rgba_from(color, 0.0),
            quiet_label(p.panel, p),
        ),
        MoonTagVariant::Tinted => (
            rgba_from(color, TINTED_BORDER_ALPHA),
            rgba_from(color, SOFT_FILL_ALPHA),
            quiet_label(tone_wash(tone, p).0, p),
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

/// The quiet label of a [`MoonTagVariant::Tinted`] chip: `text_soft`, or the body text where
/// `text_soft` would sit under [`LABEL_CONTRAST_FLOOR`] on `fill`.
///
/// Args:
///     fill: The opaque colour behind the label.
///     p: Palette the tag paints with.
///
/// Returns:
///     The label colour.
fn quiet_label(fill: u32, p: MoonPalette) -> u32 {
    if contrast_ratio(p.text_soft, fill) >= LABEL_CONTRAST_FLOOR {
        p.text_soft
    } else {
        p.text
    }
}

/// The colour of a tag's count, which carries the tone even where the label is quiet.
///
/// Args:
///     tone: The tag's tone.
///     variant: How the tone is painted.
///     p: Palette the tag paints with.
///
/// Returns:
///     Body text on a neutral tinted chip, the soft-wash label on a soft or toned tinted one, the
///     fill's ink on a filled one, and the tone itself on an outline tag.
fn count_color(tone: MoonTone, variant: MoonTagVariant, p: MoonPalette) -> u32 {
    let color = tone.color(p);
    match variant {
        MoonTagVariant::Outline => color,
        MoonTagVariant::Filled => p.ink_on(color),
        MoonTagVariant::Tinted if tone == MoonTone::Default => p.text,
        MoonTagVariant::Soft | MoonTagVariant::Tinted => tone_wash(tone, p).1,
    }
}

impl RenderOnce for MoonTag {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let text = tokens.text(11.0, 14.0);
        let chrome = tag_chrome(&tokens, self.tone, self.variant, self.radius, self.compact);
        let p = tokens.palette;
        let count_color = count_color(self.tone, self.variant, p);

        let tag = h_flex()
            .flex_none()
            .items_center()
            .gap(px(tokens.ui(if self.compact { 5.0 } else { 6.0 })))
            .min_h(px(chrome.min_h))
            .px(px(chrome.pad_x))
            .rounded(px(chrome.radius))
            .border_1()
            .border_color(chrome.border)
            .bg(chrome.background)
            .text_color(rgba_from(
                if self.dimmed { p.text } else { chrome.text },
                1.0,
            ))
            .font_family(tokens.font_family(self.mono))
            .text_size(px(text.font_size))
            .line_height(px(text.line_height))
            .font_weight(FontWeight::MEDIUM)
            .children(self.children)
            .when_some(self.count, |this, count| {
                this.child(
                    div()
                        .font_family(tokens.font_family(true))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgba_from(count_color, 1.0))
                        .child(count),
                )
            })
            .when(self.dimmed, |this| this.opacity(DIMMED_OPACITY))
            .when(self.selected, |this| {
                this.shadow(vec![box_shadow(
                    px(0.0),
                    px(0.0),
                    px(0.0),
                    px(tokens.ui(SELECTED_RING_WIDTH)),
                    rgba_from(p.accent, 1.0),
                )])
            });

        match self.on_click {
            Some((id, handler)) => tag
                .id(id)
                .cursor_pointer()
                .hover(move |this| this.border_color(rgba_from(p.border_hover, 1.0)))
                .on_click(move |event, window, cx| handler(event, window, cx))
                .into_any_element(),
            None => tag.into_any_element(),
        }
    }
}

#[cfg(test)]
mod tests;
