use gpui::prelude::FluentBuilder;
use gpui::*;

use super::{
    foundation::{MoonIndexedClickHandler, accent_underline_colored},
    text::MoonText,
    theme::{MoonTheme, MoonThemeTokens},
    tokens::{MoonPalette, MoonRect, rgba_from},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoonAccent {
    Amber,
    Blue,
    Green,
    Red,
}

/// The WCAG 2.x contrast floor the selected label must clear against the panel.
///
/// AA for normal-size text. The label renders at 11px, below the 18px/14px-bold threshold that
/// would let the 3:1 large-text floor apply.
const LABEL_CONTRAST_FLOOR: f32 = 4.5;

/// The two colours a selected segment draws with.
///
/// They are separate roles, not a redundancy: `text` sits on the panel background and carries the
/// legibility burden, while `underline` is a saturated decoration bar whose contrast against the
/// panel does not matter. Collapsing them into one value costs legibility — see
/// [`MoonAccent::colors`].
#[derive(Clone, Copy)]
struct AccentColors {
    text: u32,
    underline: u32,
}

impl MoonAccent {
    /// Resolve this accent to its selected-segment colours for `p`.
    ///
    /// The underline always takes the accent's own hue, in both themes — that bar is what makes
    /// two differently-accented strips readable as different, and its own contrast does not matter.
    ///
    /// The label cannot follow it unconditionally, so this MEASURES rather than assumes. Whether a
    /// hue is legible depends on the hue and the panel together, and the answer is not the same per
    /// theme: amber reads 8.8:1 on the dark panel but 3.5:1 on the light one, while blue clears the
    /// floor on both; dark green and red labels measure only 3.7:1 and 4.0:1 without a fallback.
    ///
    /// So the label takes the first candidate that clears [`LABEL_CONTRAST_FLOOR`]: the accent's own
    /// hue, then the palette's darkened companion where that hue has one, then `accent_fg`. If none
    /// of them clears it the plain text colour is taken unconditionally — a palette that leaves
    /// `accent_fg` illegible on its own panel has nothing better left to offer. A custom palette
    /// therefore degrades to something readable instead of inheriting a policy tuned for the two
    /// built-in ones.
    fn colors(self, p: MoonPalette) -> AccentColors {
        let underline = match self {
            Self::Amber => p.amber,
            Self::Blue => p.blue,
            Self::Green => p.green,
            Self::Red => p.red,
        };
        // The palette's purpose-built readable variant of this hue — `None` where it has none, so
        // the cascade falls straight through to `accent_fg` rather than weighing it twice.
        let companion = match self {
            Self::Green => Some(p.green_text),
            Self::Red => Some(p.red_text),
            Self::Amber | Self::Blue => None,
        };
        let text = [Some(underline), companion, Some(p.accent_fg)]
            .into_iter()
            .flatten()
            .find(|candidate| contrast_ratio(*candidate, p.panel) >= LABEL_CONTRAST_FLOOR)
            .unwrap_or(p.text);
        AccentColors { text, underline }
    }
}

/// WCAG 2.x relative luminance of an `0xRRGGBB` colour.
///
/// Deliberately not routed through this crate's other sRGB decode (`theme::color`'s oklab path):
/// that one uses the sRGB spec's 0.04045 knee, while WCAG 2.x specifies 0.03928. The two agree to
/// well within a rounding step for every colour in the palettes, but this function exists to answer
/// a WCAG question, so it follows the WCAG definition rather than borrowing a near-neighbour.
fn relative_luminance(color: u32) -> f32 {
    let channel = |shift: u32| {
        let c = ((color >> shift) & 0xFF) as f32 / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(16) + 0.7152 * channel(8) + 0.0722 * channel(0)
}

/// WCAG 2.x contrast ratio between two opaque `0xRRGGBB` colours, from 1.0 to 21.0.
///
/// Symmetric in its arguments — the brighter colour is found, not assumed to be either one.
fn contrast_ratio(a: u32, b: u32) -> f32 {
    let (la, lb) = (relative_luminance(a), relative_luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

#[derive(Clone, Debug)]
pub struct MoonSegmentItem {
    hotkey: SharedString,
    label: SharedString,
    width: f32,
    selected: bool,
    disabled: bool,
}

impl MoonSegmentItem {
    pub fn new(hotkey: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            hotkey: hotkey.into(),
            label: label.into(),
            width: 64.0,
            selected: false,
            disabled: false,
        }
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

#[derive(IntoElement)]
pub struct MoonSegmentedControl {
    id: ElementId,
    bounds: Option<MoonRect>,
    items: Vec<MoonSegmentItem>,
    accent: MoonAccent,
    item_gap: f32,
    on_click: Option<MoonIndexedClickHandler>,
}

impl MoonSegmentedControl {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            items: Vec::new(),
            accent: MoonAccent::Amber,
            item_gap: 0.0,
            on_click: None,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn accent(mut self, accent: MoonAccent) -> Self {
        self.accent = accent;
        self
    }

    pub fn item(mut self, item: MoonSegmentItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn items(mut self, items: impl IntoIterator<Item = MoonSegmentItem>) -> Self {
        self.items.extend(items);
        self
    }

    pub fn item_gap(mut self, item_gap: f32) -> Self {
        self.item_gap = item_gap;
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(usize, &ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(std::rc::Rc::new(handler));
        self
    }

    pub fn render(self) -> impl IntoElement {
        self
    }

    pub fn render_with_palette(self, p: MoonPalette) -> impl IntoElement {
        self.render_with_theme(p, MoonThemeTokens::default())
    }

    pub fn render_with_theme(self, p: MoonPalette, tokens: MoonThemeTokens) -> impl IntoElement {
        let accent = self.accent.colors(p);
        let on_click = self.on_click.clone();

        let mut root = div()
            .id(self.id)
            .relative()
            .flex()
            .items_center()
            .h(px(tokens.fit_height(26.0, 14.0, 6.0)))
            .gap(px(tokens.ui(self.item_gap)))
            .whitespace_nowrap();

        if let Some(bounds) = self.bounds {
            root = root
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }

        for (ix, item) in self.items.into_iter().enumerate() {
            let selected = item.selected;
            let disabled = item.disabled;
            let key_color = if selected { accent.text } else { p.text_muted };
            let key_alpha = if selected { 0.60 } else { 0.667 };
            let label_color = if selected { accent.text } else { p.text_muted };
            let item_click = on_click.clone();

            let mut cell = div()
                .id(("segment-item", ix))
                .relative()
                .flex()
                .items_center()
                .justify_center()
                .gap(px(tokens.ui(5.0)))
                .w(px(item.width))
                .h_full()
                .px(px(tokens.ui(11.0)))
                .cursor_default()
                .when(!selected && !disabled, |this| {
                    this.hover(move |this| this.bg(rgba_from(p.overlay, 0.025)))
                        .active(move |this| this.bg(rgba_from(p.overlay, 0.016)))
                })
                .child(
                    MoonText::new(item.hotkey)
                        .color(key_color)
                        .alpha(if disabled { 0.40 } else { key_alpha })
                        .font_size(8.5)
                        .line_height(12.0)
                        .weight(400.0)
                        .mono(true)
                        .uppercase(false)
                        .render(),
                )
                .child(
                    MoonText::new(item.label)
                        .color(label_color)
                        .alpha(if disabled { 0.40 } else { 1.0 })
                        .font_size(11.0)
                        .line_height(14.0)
                        .weight(if selected { 500.0 } else { 400.0 })
                        .mono(true)
                        .uppercase(false)
                        .render(),
                );

            if selected {
                cell = cell.child(accent_underline_colored(
                    accent.underline,
                    &tokens,
                    8.0,
                    8.0,
                    0.0,
                ));
            }

            if let Some(on_click) = item_click {
                cell = cell.on_click(move |event, window, cx| {
                    if disabled {
                        cx.stop_propagation();
                        return;
                    }
                    on_click(ix, event, window, cx);
                });
            }

            root = root.child(cell);
        }

        root
    }
}

impl RenderOnce for MoonSegmentedControl {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        self.render_with_theme(MoonPalette::active(cx), tokens)
    }
}

#[cfg(test)]
mod tests {
    // NOT `use super::*`: the glob would pull in the `gpui::test` macro, and `#[test]` would
    // expand into itself (recursion limit).
    use super::{LABEL_CONTRAST_FLOOR, MoonAccent, MoonPalette, contrast_ratio};

    const ACCENTS: [MoonAccent; 4] = [
        MoonAccent::Amber,
        MoonAccent::Blue,
        MoonAccent::Green,
        MoonAccent::Red,
    ];

    #[test]
    fn contrast_ratio_matches_the_published_anchors() {
        // Anchors the arithmetic every other assertion here leans on, against values published
        // with the WCAG 2.x definition rather than against this file's own output: the extremes
        // are exactly 21:1 and 1:1, and #767676 / #595959 on white are the canonical greys that
        // sit right on the AA and AAA thresholds.
        assert!((contrast_ratio(0x000000, 0xFFFFFF) - 21.0).abs() < 0.01);
        assert!((contrast_ratio(0x808080, 0x808080) - 1.0).abs() < 0.01);
        assert!((contrast_ratio(0x767676, 0xFFFFFF) - 4.54).abs() < 0.01);
        assert!((contrast_ratio(0x595959, 0xFFFFFF) - 7.00).abs() < 0.01);
        // Symmetric: which colour is brighter is found, not assumed.
        assert_eq!(
            contrast_ratio(0x000000, 0xFFFFFF),
            contrast_ratio(0xFFFFFF, 0x000000)
        );
    }

    #[test]
    fn every_accent_label_clears_the_contrast_floor_in_both_themes() {
        // The product decision this pins: an accent may tint the label only while the label stays
        // readable. The plausible edit that breaks it is letting the label follow the underline
        // hue unconditionally on the theory that a whole theme is
        // "safe". It is not: on the dark panel green and red measure 3.71:1 and 4.02:1, and on the
        // light one amber measures 3.48:1, so that edit reddens here on three separate accents.
        for p in [MoonPalette::TERMINAL, MoonPalette::LIGHT] {
            for accent in ACCENTS {
                let ratio = contrast_ratio(accent.colors(p).text, p.panel);
                assert!(
                    ratio >= LABEL_CONTRAST_FLOOR,
                    "{accent:?} label is {ratio:.2}:1 on the panel (is_light={}), under the \
                     {LABEL_CONTRAST_FLOOR}:1 floor",
                    p.is_light()
                );
            }
        }
    }

    #[test]
    fn each_accent_keeps_its_own_underline() {
        // The underline is what survives the legibility rule above — it always carries the hue, so
        // two differently-accented strips stay tellable apart even where their labels collapse onto
        // the same readable colour. Checks every pair, not one: a resolver that special-cased a
        // single accent would slip past an amber-vs-blue spot check.
        for p in [MoonPalette::TERMINAL, MoonPalette::LIGHT] {
            for (i, a) in ACCENTS.iter().enumerate() {
                for b in &ACCENTS[i + 1..] {
                    assert_ne!(
                        a.colors(p).underline,
                        b.colors(p).underline,
                        "{a:?} and {b:?} share an underline (is_light={})",
                        p.is_light()
                    );
                }
            }
        }
    }

    #[test]
    fn render_resolves_colours_from_the_configured_accent() {
        // Guards the CALL SITE, which no resolver test can reach: this assertion reddens when
        // render stops reading `self.accent` even if the resolver itself remains correct.
        let source = include_str!("segment.rs");
        let implementation = source.split("#[cfg(test)]").next().unwrap_or(source);

        assert!(
            implementation.contains("self.accent.colors(")
                && implementation.contains("accent_underline_colored("),
            "selected-segment colours must be resolved from self.accent, not from a palette-wide role"
        );
    }
}
