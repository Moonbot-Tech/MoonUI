use gpui::prelude::FluentBuilder as _;
use gpui::*;

use crate::progress::Progress;

use super::{
    skin::{MoonFacet, MoonFacetKind, facet, facet_glow},
    theme::{MoonTheme, MoonThemeTokens},
    tokens::{MoonTone, rgba_from},
};

#[derive(IntoElement)]
pub struct MoonProgress {
    id: ElementId,
    value: f32,
    loading: bool,
    tone: MoonTone,
    color: Option<u32>,
    height: f32,
    radius: Option<f32>,
}

impl MoonProgress {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            value: 0.0,
            loading: false,
            tone: MoonTone::Positive,
            color: None,
            height: 4.0,
            radius: None,
        }
    }

    pub fn value(mut self, value: f32) -> Self {
        self.value = value.clamp(0.0, 100.0);
        self
    }

    /// Show the indeterminate sliding bar instead of `value`.
    ///
    /// **Cost:** the indeterminate bar is an endlessly repeating animation. Every animation frame
    /// resolves to `cx.notify(..)`, and GPUI re-renders the window's whole uncached view tree per
    /// drawn frame, so this repaints **the entire hosting window** on every vsync for as long as
    /// it is visible — not just the bar. Measured in the component gallery (debug build): a single
    /// always-on bar cost ~1.2 cores at idle and starved input handling; the same page idled at
    /// ~0% with it off. Use it for a genuinely pending operation and stop it when the operation
    /// ends; never leave one on as permanent page decoration.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    pub fn tone(mut self, tone: MoonTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn color(mut self, color: u32) -> Self {
        self.color = Some(color);
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
        self
    }

    pub fn render(self) -> impl IntoElement {
        self
    }
}

impl RenderOnce for MoonProgress {
    /// Renders the bar: the base progress bar in the tone's colour under the Flat skin, and
    /// [`ProgressFacet`]'s groove and lit fill under the Facet skin, at the same height.
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        let height = tokens.ui(self.height);
        let radius = self
            .radius
            .map(|radius| tokens.ui(radius))
            .unwrap_or(height * 0.5);
        let color = self.color.unwrap_or_else(|| self.tone.color(p));

        Progress::new(self.id)
            .value(self.value)
            .loading(self.loading)
            .color(rgba_from(color, 1.0))
            .h(px(height))
            .rounded(px(radius))
            .when_some(ProgressFacet::resolve(&tokens, color), |bar, facet| {
                facet.apply(bar)
            })
            .into_any_element()
    }
}

/// How far the top edge of a faceted bar's fill is mixed toward white.
const FACET_FILL_LIFT: f32 = 0.2;
/// Opacity of the glow a faceted bar's fill casts in its own colour.
const FACET_FILL_GLOW_ALPHA: f32 = 0.4;
/// Blur of that glow as the design gives it, in design-reference pixels.
const FACET_FILL_GLOW_BLUR: f32 = 8.0;

/// What a progress bar paints under the Facet skin.
///
/// The shared progress engine owns motion and keyed state; the skin supplies only paint.
#[derive(Clone, Debug, PartialEq)]
struct ProgressFacet {
    /// The groove the bar runs in: the skin's well, painted without its edge.
    track: MoonFacet,
    /// The fill: the colour at its top edge, then the one at its bottom edge.
    fill: [Hsla; 2],
    /// The glow the fill casts in its own colour.
    glow: BoxShadow,
}

impl ProgressFacet {
    /// Resolves a progress bar's faceted surfaces.
    ///
    /// Args:
    ///     tokens: The active theme side; supplies the skin, the palette and the UI scale.
    ///     color: The bar's colour as `0xRRGGBB`: its tone, or the colour set on it.
    ///
    /// Returns:
    ///     The surfaces to paint, or `None` under the Flat skin, where the base bar stands.
    fn resolve(tokens: &MoonThemeTokens, color: u32) -> Option<Self> {
        if !tokens.skin.is_facet() {
            return None;
        }
        let tone = rgba_from(color, 1.0);
        Some(Self {
            track: facet(MoonFacetKind::Well, tokens),
            fill: [tone.blend(white().opacity(FACET_FILL_LIFT)), tone],
            glow: facet_glow(color, FACET_FILL_GLOW_ALPHA, FACET_FILL_GLOW_BLUR, tokens),
        })
    }

    /// Apply the groove and lit fill without replacing the donor's motion or geometry.
    fn apply(self, bar: Progress) -> Progress {
        let track = StyleRefinement {
            background: Some(Fill::Color(self.track.fill)),
            box_shadow: Some(self.track.shadows),
            ..Default::default()
        };
        let fill = StyleRefinement {
            background: Some(
                linear_gradient(
                    180.0,
                    linear_color_stop(self.fill[0], 0.0),
                    linear_color_stop(self.fill[1], 1.0),
                )
                .into(),
            ),
            box_shadow: Some(vec![self.glow]),
            ..Default::default()
        };
        bar.surface_styles(track, fill)
    }
}

#[cfg(test)]
mod tests;
