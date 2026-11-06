//! The theme skin: the manner in which a theme draws depth.
//!
//! A palette says which colours a theme has; the skin says what a surface made of them looks
//! like. [`MoonSkin::Flat`] is every theme that existed before skins did: one solid fill and a
//! hairline. [`MoonSkin::Facet`] lights each surface from above: a two-stop vertical gradient,
//! a one-pixel edge, a highlight along the top and a shadow underneath.
//!
//! Components ask [`MoonSkin::active`] and, only when it answers `Facet`, paint the surface
//! [`facet`] or [`facet_tinted`] resolves for them. Nothing here is consulted under `Flat`, so
//! a theme without a skin renders exactly as it did before this module existed.
//!
//! Every colour is derived from the palette by rule rather than stored, so the same code
//! serves a dark and a light palette: `overlay` is the highlight ink, `shadow` is the shadow
//! ink, and [`MoonPalette::is_light`] decides strengths only. Every length goes through
//! [`MoonThemeTokens::ui`], so a facet grows with the control it is painted on.

use gpui::{
    AnyElement, App, Background, BoxShadow, Div, Hsla, InteractiveElement as _, IntoElement,
    ParentElement as _, Styled, div, linear_color_stop, linear_gradient, px, relative,
};
use serde::{Deserialize, Serialize};

use super::{
    foundation::box_shadow,
    theme::{MoonTheme, MoonThemeTokens},
    tokens::{MoonPalette, rgba_from},
};

/// The white end of [`lift`], and the highlight ink a light palette uses in place of its
/// (black) `overlay`.
const WHITE: u32 = 0xFFFFFF;
/// The black end of [`sink`].
const BLACK: u32 = 0x000000;

/// What a blur of the design becomes as a blur radius of the renderer.
///
/// The skin's blurs are written as the design gives them: CSS `box-shadow` blurs, where the
/// gaussian's standard deviation is half the blur. The renderer takes the standard deviation
/// itself as its blur radius, so a design blur is halved on the way in. Passed through as it is,
/// every shadow and glow would spread twice as wide and arrive at a fraction of its strength.
const DESIGN_BLUR_TO_RENDERER: f32 = 0.5;

/// How far down the window the backdrop's top light reaches, as a fraction of its height.
const BACKDROP_TOP_LIGHT_REACH: f32 = 0.5;
/// Blur of a backdrop ambient light as the design gives it, in design-reference pixels.
const BACKDROP_LIGHT_BLUR: f32 = 260.0;
/// Spread radius of a backdrop ambient light, in design-reference pixels.
const BACKDROP_LIGHT_SPREAD: f32 = 140.0;

/// The manner in which a theme draws depth.
///
/// A theme side carries one in [`MoonThemeTokens::skin`]. A theme file that does not name a
/// skin gets [`Self::Flat`], which is what every theme was before skins existed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoonSkin {
    /// Solid fills and hairlines; no component consults this module.
    #[default]
    Flat,
    /// Surfaces lit from above: gradient fills, one-pixel edges, highlights and shadows.
    Facet,
}

impl MoonSkin {
    /// Return whether this is the [`Self::Facet`] skin.
    pub fn is_facet(self) -> bool {
        matches!(self, Self::Facet)
    }

    /// Return the skin of the active theme side.
    ///
    /// Reads the [`MoonTheme`] global directly, so it is cheap enough to ask on every render.
    ///
    /// Args:
    ///     cx: The application context holding the theme global.
    ///
    /// Returns:
    ///     The active skin, or [`Self::Flat`] when no theme is installed.
    pub fn active(cx: &App) -> Self {
        MoonTheme::global(cx)
            .map(|theme| theme.skin)
            .unwrap_or_default()
    }
}

/// A resolved faceted surface, ready to paint.
///
/// A component applies `fill` as its background, `edge` as the colour of a one-pixel border
/// and `shadows` through `Styled::shadow`.
#[derive(Clone, Debug, PartialEq)]
pub struct MoonFacet {
    /// The surface fill: a two-stop gradient running from the top edge to the bottom edge.
    pub fill: Background,
    /// The colour of the one-pixel edge around the surface.
    pub edge: Hsla,
    /// The shadow stack: the inset shadow first, then the shadows outside the surface.
    pub shadows: Vec<BoxShadow>,
}

impl MoonFacet {
    /// Paint this surface over the style already set on `this`.
    ///
    /// Sets the fill as the background, the edge as the border colour and the shadow stack.
    /// The border width and the corner radius stay with the caller, so the same surface serves
    /// a hairline frame at any scale.
    ///
    /// Args:
    ///     this: The element or style refinement to paint on.
    ///
    /// Returns:
    ///     `this`, painted.
    pub fn paint<S: Styled>(self, this: S) -> S {
        this.bg(self.fill)
            .border_color(self.edge)
            .shadow(self.shadows)
    }
}

/// The neutral surfaces a faceted theme is built from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoonFacetKind {
    /// A dock panel: the large slab the rest of the interface sits on.
    Slab,
    /// A control that stands off its surface, such as a button or a pill.
    Raised,
    /// [`Self::Raised`] under the pointer.
    RaisedHover,
    /// A surface sunk into its parent: an input, a toggle track, a slider groove.
    Well,
    /// A small well that holds a mark: a checkbox box, a radio circle, a toggle track. It is
    /// [`Self::Well`] under an edge strong enough to read at a dozen pixels, where the well's
    /// own edge disappears into the surface around it.
    Socket,
    /// A surface floating above the window: a menu, popover, tooltip or dialog.
    Popup,
    /// A card resting on a panel.
    Card,
}

/// How strongly a tone colours a faceted surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoonFacetTint {
    /// A light wash of the tone over the panel.
    Soft,
    /// A stronger wash with a glow, for the selected one of several.
    Selected,
    /// The tone itself as the fill, for a primary action.
    Solid,
}

/// Resolve a neutral faceted surface from the palette.
///
/// Args:
///     kind: Which surface to resolve.
///     tokens: The theme side to resolve it for; supplies the palette and the UI scale.
///
/// Returns:
///     The fill, edge and shadow stack for that surface.
pub fn facet(kind: MoonFacetKind, tokens: &MoonThemeTokens) -> MoonFacet {
    let p = tokens.palette;
    let top_highlight =
        |dark: f32, light: f32| inset_shadow(1.0, 0.0, highlight(p, dark, light), tokens);
    let cast = |y: f32, blur: f32, dark: f32, light: f32| {
        drop_shadow(y, blur, shade(p, dark, light), tokens)
    };
    let raised_shadows = || vec![top_highlight(0.16, 0.9), cast(1.0, 2.0, 0.55, 0.12)];

    match kind {
        MoonFacetKind::Slab => MoonFacet {
            fill: vertical(lift(p.shell_high, 0.045), p.shell_high),
            edge: edge(p, 0.07, p.border),
            shadows: vec![
                top_highlight(0.12, 0.9),
                cast(18.0, 40.0, 0.65, 0.10),
                cast(2.0, 6.0, 0.5, 0.08),
            ],
        },
        MoonFacetKind::Raised => MoonFacet {
            fill: vertical(p.panel_high, p.panel),
            edge: edge(p, 0.09, p.border),
            shadows: raised_shadows(),
        },
        MoonFacetKind::RaisedHover => MoonFacet {
            fill: vertical(lift(p.panel_high, 0.05), lift(p.panel, 0.04)),
            edge: edge(p, 0.2, p.border_hover),
            shadows: raised_shadows(),
        },
        // A well inverts the stack: the inset shadow is the dark one, pooling under the top
        // edge, and the light sits outside, along the lip below the well.
        MoonFacetKind::Well => MoonFacet {
            fill: vertical(sink(p.gutter, well_sink(p)), p.gutter),
            edge: edge(p, 0.06, p.border),
            shadows: vec![
                inset_shadow(1.0, 3.0, shade(p, 0.7, 0.12), tokens),
                drop_shadow(1.0, 0.0, highlight(p, 0.07, 0.8), tokens),
            ],
        },
        MoonFacetKind::Socket => MoonFacet {
            edge: edge(p, 0.17, p.border_hover),
            ..facet(MoonFacetKind::Well, tokens)
        },
        MoonFacetKind::Popup => MoonFacet {
            fill: vertical(lift(p.panel_high, 0.02), p.card),
            edge: edge(p, 0.13, p.border),
            shadows: vec![
                top_highlight(0.2, 0.9),
                cast(28.0, 60.0, 0.8, 0.16),
                cast(6.0, 16.0, 0.55, 0.10),
            ],
        },
        MoonFacetKind::Card => MoonFacet {
            fill: vertical(lift(p.card, 0.04), p.card),
            edge: edge(p, 0.08, p.border),
            shadows: vec![top_highlight(0.12, 0.9), cast(2.0, 6.0, 0.5, 0.08)],
        },
    }
}

/// Resolve a faceted surface coloured by a tone.
///
/// Args:
///     tone: The colouring tone as `0xRRGGBB`, such as the palette's `blue` or `accent`.
///     tint: How strongly the tone colours the surface.
///     tokens: The theme side to resolve it for; supplies the palette and the UI scale.
///
/// Returns:
///     The fill, edge and shadow stack for that surface.
pub fn facet_tinted(tone: u32, tint: MoonFacetTint, tokens: &MoonThemeTokens) -> MoonFacet {
    let p = tokens.palette;
    let washed = |share: f32| mix(p.panel, tone, share);
    // The `overlay` ink in either palette: a highlight on a dark one, and on a light one, where
    // `overlay` is black, a faint line of shade under the top edge instead.
    let soft_highlight = || inset_shadow(1.0, 0.0, rgba_from(p.overlay, 0.10), tokens);
    let contact = || drop_shadow(1.0, 2.0, shade(p, 0.55, 0.12), tokens);

    match tint {
        MoonFacetTint::Soft => MoonFacet {
            fill: vertical(washed(0.14), washed(0.06)),
            edge: rgba_from(tone, 0.42),
            shadows: vec![soft_highlight(), contact()],
        },
        MoonFacetTint::Selected => MoonFacet {
            fill: vertical(washed(0.20), washed(0.08)),
            edge: rgba_from(tone, 0.7),
            shadows: vec![
                soft_highlight(),
                contact(),
                facet_glow(tone, 0.26, 18.0, tokens),
            ],
        },
        // The highlight on a solid tone is white in either palette: the fill is the tone itself,
        // which the palette's side does not change.
        MoonFacetTint::Solid => MoonFacet {
            fill: vertical(lift(tone, 0.25), sink(tone, 0.06)),
            edge: rgba_from(sink(tone, 0.2), 1.0),
            shadows: vec![
                inset_shadow(1.0, 0.0, rgba_from(WHITE, 0.45), tokens),
                contact(),
                facet_glow(tone, if p.is_light() { 0.2 } else { 0.32 }, 18.0, tokens),
            ],
        },
    }
}

/// Build a glow: a blurred shadow centred on its element, with no offset and no spread.
///
/// Args:
///     color: The glow colour as `0xRRGGBB`.
///     alpha: The glow's opacity at full strength, from 0 to 1.
///     blur: The blur as the design gives it, a CSS `box-shadow` blur in design-reference
///         pixels; scaled with the UI and converted for the renderer.
///     tokens: The theme side, for the UI scale.
///
/// Returns:
///     The shadow, to append to a surface's shadow stack.
pub fn facet_glow(color: u32, alpha: f32, blur: f32, tokens: &MoonThemeTokens) -> BoxShadow {
    box_shadow(
        px(0.0),
        px(0.0),
        renderer_blur(blur, tokens),
        px(0.0),
        rgba_from(color, alpha),
    )
}

/// Convert a blur of the design, in design-reference pixels, to the renderer's blur radius at
/// the UI scale of `tokens`; see [`DESIGN_BLUR_TO_RENDERER`].
fn renderer_blur(design_blur: f32, tokens: &MoonThemeTokens) -> gpui::Pixels {
    px(tokens.ui(design_blur * DESIGN_BLUR_TO_RENDERER))
}

/// Build a horizontal one-pixel rule that fades out toward both ends.
///
/// The rule fills its parent's width. A gradient has exactly two stops, so the rule is two
/// halves: the first fades in from transparent, the second fades back out.
///
/// Args:
///     color: The colour at the middle of the rule; its own alpha is kept there.
///
/// Returns:
///     The rule element.
pub fn fading_rule_h(color: Hsla) -> impl IntoElement {
    let [rising, falling] = fading_halves(color, 90.0);
    div()
        .flex()
        .flex_row()
        .flex_none()
        .w_full()
        .h(px(1.0))
        .child(div().flex_1().h_full().bg(rising))
        .child(div().flex_1().h_full().bg(falling))
}

/// Build a vertical one-pixel rule that fades out toward both ends.
///
/// The rule fills its parent's height; see [`fading_rule_h`] for the construction.
///
/// Args:
///     color: The colour at the middle of the rule; its own alpha is kept there.
///
/// Returns:
///     The rule element.
pub fn fading_rule_v(color: Hsla) -> impl IntoElement {
    let [rising, falling] = fading_halves(color, 180.0);
    div()
        .flex()
        .flex_col()
        .flex_none()
        .h_full()
        .w(px(1.0))
        .child(div().flex_1().w_full().bg(rising))
        .child(div().flex_1().w_full().bg(falling))
}

/// Build the window backdrop of a faceted theme.
///
/// The backdrop is the window fill with a top light — `shell` lifted along the top edge and
/// settling to plain `shell` part of the way down — and two very soft ambient lights: the
/// accent around the upper-left quarter and blue around the lower-right one. Each light is a
/// one-pixel element casting a large blurred shadow, clipped to the backdrop.
///
/// The element is absolutely positioned and fills its nearest positioned ancestor. It is
/// opaque, so it belongs behind everything else in that ancestor; it takes no pointer input.
/// `MoonRoot` paints it on its own when it owns an opaque window fill. A host whose root
/// paints no fill, to show what lies behind the scene, places it where it wants one.
///
/// Args:
///     tokens: The theme side to build it for; supplies the skin, the palette and the UI scale.
///
/// Returns:
///     The backdrop under [`MoonSkin::Facet`], `None` under [`MoonSkin::Flat`].
pub fn moon_backdrop(tokens: &MoonThemeTokens) -> Option<AnyElement> {
    if !tokens.skin.is_facet() {
        return None;
    }
    let p = tokens.palette;
    let light = |color: u32, alpha: f32| moon_ambient_light(color, alpha, tokens);

    Some(
        div()
            .debug_selector(|| "moon-backdrop".into())
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .overflow_hidden()
            .bg(backdrop_fill(p))
            .child(
                light(p.accent, 0.10)
                    .left(relative(0.25))
                    .top(relative(0.25)),
            )
            .child(
                light(p.blue, 0.08)
                    .right(relative(0.25))
                    .bottom(relative(0.25)),
            )
            .into_any_element(),
    )
}

/// Build one ambient light of a faceted theme: a wide, soft pool of `color`.
///
/// The light is a one-pixel element casting a large blurred shadow — the lights of
/// [`moon_backdrop`] are two of these. It is absolutely positioned and the caller places it;
/// the pool is centred on that point. A host whose window paints no backdrop, because something
/// shows through its root, lays one inside an opaque band of its own instead, in a parent that
/// clips it to that band.
///
/// The skin is the caller's question: this builds the light under either one.
///
/// Args:
///     color: The light's colour as `0xRRGGBB`.
///     alpha: The light's opacity at its centre, from 0 to 1.
///     tokens: The theme side, for the UI scale.
///
/// Returns:
///     The light, absolutely positioned and not yet placed.
pub fn moon_ambient_light(color: u32, alpha: f32, tokens: &MoonThemeTokens) -> Div {
    div()
        .absolute()
        .size(px(1.0))
        .shadow(vec![ambient_light(color, alpha, tokens)])
}

/// Build the backdrop fill: `shell` lifted along the top edge, plain `shell` from
/// [`BACKDROP_TOP_LIGHT_REACH`] of the way down.
fn backdrop_fill(p: MoonPalette) -> Background {
    linear_gradient(
        180.0,
        linear_color_stop(rgba_from(lift(p.shell, 0.035), 1.0), 0.0),
        linear_color_stop(rgba_from(p.shell, 1.0), BACKDROP_TOP_LIGHT_REACH),
    )
}

/// Build the shadow that makes one backdrop ambient light of `color` at `alpha`.
fn ambient_light(color: u32, alpha: f32, tokens: &MoonThemeTokens) -> BoxShadow {
    box_shadow(
        px(0.0),
        px(0.0),
        renderer_blur(BACKDROP_LIGHT_BLUR, tokens),
        px(tokens.ui(BACKDROP_LIGHT_SPREAD)),
        rgba_from(color, alpha),
    )
}

/// Build the two halves of a fading rule along the gradient `angle`: transparent rising to
/// `color`, then `color` falling back to transparent.
///
/// The transparent ends keep the hue of `color`, so the fade does not pass through grey.
fn fading_halves(color: Hsla, angle: f32) -> [Background; 2] {
    let clear = color.opacity(0.0);
    [
        linear_gradient(
            angle,
            linear_color_stop(clear, 0.0),
            linear_color_stop(color, 1.0),
        ),
        linear_gradient(
            angle,
            linear_color_stop(color, 0.0),
            linear_color_stop(clear, 1.0),
        ),
    ]
}

/// Build an opaque gradient running from `top` at the top edge to `bottom` at the bottom edge,
/// both `0xRRGGBB`.
fn vertical(top: u32, bottom: u32) -> Background {
    linear_gradient(
        180.0,
        linear_color_stop(rgba_from(top, 1.0), 0.0),
        linear_color_stop(rgba_from(bottom, 1.0), 1.0),
    )
}

/// Mix `over` into `base` per sRGB channel, all `0xRRGGBB`.
///
/// Args:
///     base: The colour at an `amount` of 0.
///     over: The colour at an `amount` of 1.
///     amount: The share of `over` in the result, clamped to 0..=1.
///
/// Returns:
///     The mixed colour, each channel rounded to the nearest level.
fn mix(base: u32, over: u32, amount: f32) -> u32 {
    let amount = amount.clamp(0.0, 1.0);
    let channel = |shift: u32| {
        let from = ((base >> shift) & 0xFF) as f32;
        let to = ((over >> shift) & 0xFF) as f32;
        ((from + (to - from) * amount).round() as u32) << shift
    };
    channel(16) | channel(8) | channel(0)
}

/// Mix `rgb` toward white by `amount`: 0 keeps the colour, 1 is white.
fn lift(rgb: u32, amount: f32) -> u32 {
    mix(rgb, WHITE, amount)
}

/// Mix `rgb` toward black by `amount`: 0 keeps the colour, 1 is black.
fn sink(rgb: u32, amount: f32) -> u32 {
    mix(rgb, BLACK, amount)
}

/// Resolve how far the top of a well is sunk below its `gutter` floor.
///
/// A share of black, so the same share darkens a light floor many times more than a dark one:
/// 15% is barely a step on a near-black gutter and a slab of grey on a near-white one. A light
/// palette therefore sinks its wells by a fraction of what a dark one does.
fn well_sink(p: MoonPalette) -> f32 {
    if p.is_light() { 0.04 } else { 0.15 }
}

/// Resolve a highlight ink: the palette's `overlay` at `dark` on a dark palette, white at
/// `light` on a light one, where `overlay` is black and cannot light anything.
fn highlight(p: MoonPalette, dark: f32, light: f32) -> Hsla {
    if p.is_light() {
        rgba_from(WHITE, light)
    } else {
        rgba_from(p.overlay, dark)
    }
}

/// Resolve a shadow ink: the palette's `shadow` at `dark` on a dark palette and at `light` on
/// a light one, where a shadow needs far less to show.
fn shade(p: MoonPalette, dark: f32, light: f32) -> Hsla {
    rgba_from(p.shadow, if p.is_light() { light } else { dark })
}

/// Resolve an edge colour: the `overlay` ink at `dark` on a dark palette, the opaque
/// `light_role` border entry on a light one.
fn edge(p: MoonPalette, dark: f32, light_role: u32) -> Hsla {
    if p.is_light() {
        rgba_from(light_role, 1.0)
    } else {
        rgba_from(p.overlay, dark)
    }
}

/// Build a shadow outside its element, `y` below it and blurred by the design blur `blur`, both
/// in design-reference pixels.
fn drop_shadow(y: f32, blur: f32, color: Hsla, tokens: &MoonThemeTokens) -> BoxShadow {
    box_shadow(
        px(0.0),
        px(tokens.ui(y)),
        renderer_blur(blur, tokens),
        px(0.0),
        color,
    )
}

/// Build a shadow inside its element, reaching `y` down from the top edge and blurred by the
/// design blur `blur`, both in design-reference pixels.
fn inset_shadow(y: f32, blur: f32, color: Hsla, tokens: &MoonThemeTokens) -> BoxShadow {
    drop_shadow(y, blur, color, tokens).inset()
}

#[cfg(test)]
mod tests;
