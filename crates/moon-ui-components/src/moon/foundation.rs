use std::rc::Rc;

use gpui::{
    App, Background, BoxShadow, ClickEvent, Corners, DefiniteLength, Div, Edges, FontWeight, Hsla,
    ParentElement, Pixels, Refineable, SharedString, StyleRefinement, Styled, Window, div,
    linear_color_stop, linear_gradient, point, px,
};
use serde::{Deserialize, Serialize};

use super::{
    theme::MoonThemeTokens,
    tokens::{MoonPalette, rgba_from},
};

#[inline(always)]
pub fn h_flex() -> Div {
    div().flex().flex_row().items_center()
}

#[inline(always)]
pub fn v_flex() -> Div {
    div().flex().flex_col()
}

#[inline(always)]
pub fn box_shadow(
    x: impl Into<Pixels>,
    y: impl Into<Pixels>,
    blur: impl Into<Pixels>,
    spread: impl Into<Pixels>,
    color: Hsla,
) -> BoxShadow {
    BoxShadow {
        offset: point(x.into(), y.into()),
        blur_radius: blur.into(),
        spread_radius: spread.into(),
        inset: false,
        color,
    }
}

pub fn selected_background(p: MoonPalette) -> Background {
    linear_gradient(
        90.0,
        linear_color_stop(rgba_from(p.accent, p.accent_tint_a), 0.0),
        linear_color_stop(rgba_from(p.accent, 0.0), 0.72),
    )
}

pub fn selected_flat(p: MoonPalette) -> Hsla {
    rgba_from(p.accent, p.accent_tint_a)
}

/// Draw the palette accent as an inset underline with fading ends and a glow.
pub fn accent_underline(
    p: MoonPalette,
    tokens: &MoonThemeTokens,
    left: f32,
    right: f32,
    bottom: f32,
) -> Div {
    accent_underline_colored(p.accent, tokens, left, right, bottom)
}

/// Same treatment as [`accent_underline`], driven by an explicit colour.
///
/// Lets a per-item accent pick the bar's hue instead of the single palette-wide `accent` role.
/// Crate-visible on purpose: `foundation::*` is publicly re-exported, so a `pub` here would
/// enlarge the tracked API surface for a helper only the segmented control needs.
pub(crate) fn accent_underline_colored(
    color: u32,
    tokens: &MoonThemeTokens,
    left: f32,
    right: f32,
    bottom: f32,
) -> Div {
    let underline_left = linear_gradient(
        90.0,
        linear_color_stop(rgba_from(color, 0.0), 0.0),
        linear_color_stop(rgba_from(color, 1.0), 1.0),
    );
    let underline_right = linear_gradient(
        90.0,
        linear_color_stop(rgba_from(color, 1.0), 0.0),
        linear_color_stop(rgba_from(color, 0.0), 1.0),
    );
    let shadow = box_shadow(
        px(0.0),
        px(0.0),
        px(tokens.ui(8.0)),
        px(0.0),
        rgba_from(color, 0.62),
    );

    div()
        .absolute()
        .left(px(tokens.ui(left)))
        .right(px(tokens.ui(right)))
        .bottom(px(tokens.ui(bottom)))
        .h(px(tokens.ui(2.0)))
        .flex()
        .child(
            div()
                .w(px(tokens.ui(25.0)))
                .h_full()
                .bg(underline_left)
                .shadow(vec![shadow.clone()]),
        )
        .child(
            div()
                .flex_1()
                .h_full()
                .bg(rgba_from(color, 1.0))
                .shadow(vec![shadow.clone()]),
        )
        .child(
            div()
                .w(px(tokens.ui(25.0)))
                .h_full()
                .bg(underline_right)
                .shadow(vec![shadow]),
        )
}

pub trait StyledExt: Styled + Sized {
    fn refine_style(mut self, style: &StyleRefinement) -> Self {
        self.style().refine(style);
        self
    }

    #[inline(always)]
    fn h_flex(self) -> Self {
        self.flex().flex_row().items_center()
    }

    #[inline(always)]
    fn v_flex(self) -> Self {
        self.flex().flex_col()
    }

    fn paddings<L>(self, paddings: impl Into<Edges<L>>) -> Self
    where
        L: Into<DefiniteLength> + Clone + Default + std::fmt::Debug + PartialEq,
    {
        let paddings = paddings.into();
        self.pt(paddings.top.into())
            .pb(paddings.bottom.into())
            .pl(paddings.left.into())
            .pr(paddings.right.into())
    }

    fn margins<L>(self, margins: impl Into<Edges<L>>) -> Self
    where
        L: Into<DefiniteLength> + Clone + Default + std::fmt::Debug + PartialEq,
    {
        let margins = margins.into();
        self.mt(margins.top.into())
            .mb(margins.bottom.into())
            .ml(margins.left.into())
            .mr(margins.right.into())
    }

    fn corner_radii(self, radius: Corners<Pixels>) -> Self {
        self.rounded_tl(radius.top_left)
            .rounded_tr(radius.top_right)
            .rounded_bl(radius.bottom_left)
            .rounded_br(radius.bottom_right)
    }

    fn font_normal(self) -> Self {
        self.font_weight(FontWeight::NORMAL)
    }

    fn font_medium(self) -> Self {
        self.font_weight(FontWeight::MEDIUM)
    }

    fn font_semibold(self) -> Self {
        self.font_weight(FontWeight::SEMIBOLD)
    }

    fn font_bold(self) -> Self {
        self.font_weight(FontWeight::BOLD)
    }
}

impl<E: Styled> StyledExt for E {}

/// The shared size scale for Moon components, from smallest to largest.
///
/// A component renders only the tiers it supports (a checkbox has `Xs`, `Sm` and `Md`); any
/// other tier resolves to the nearest supported one instead of failing.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum MoonSize {
    #[serde(alias = "XSmall")]
    Xs,
    #[serde(alias = "Small")]
    Sm,
    #[default]
    #[serde(alias = "Medium")]
    Md,
    #[serde(alias = "Large")]
    Lg,
    Xl,
    Xxl,
}

/// Shared control metrics in unscaled design-reference pixels.
///
/// | Tier | Height | Text / line | Radius | Horizontal padding | Icon / label gap |
/// | --- | --- | --- | --- | --- | --- |
/// | Xs | 20 | 12 / 16 | 4 | 6 | 4 |
/// | Sm | 24 | 14 / 20 | 4 | 8 | 8 |
/// | Md | 32 | 16 / 24 | 6 | 12 | 12 |
/// | Lg | 40 | 18 / 28 | 8 | 16 | 12 |
/// | Xl | 48 | 20 / 28 | 8 | 20 | 16 |
/// | Xxl | 56 | 24 / 32 | 10 | 24 | 16 |
///
/// [WCAG 2.2 SC 2.5.8 (Target Size, Minimum)](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html)
/// sets a 24 by 24 CSS pixel pointer-target minimum, subject to exceptions. `Sm` is the
/// smallest tier meeting the height floor and the intended default for terminal controls;
/// consumers must also provide adequate target width. This does not change [`MoonSize`]'s
/// existing `Md` default. `Xs` is the sole tier below the floor, reserved for fixed-height
/// dense strips (header ticker, status bar) where the spacing exception is satisfied:
/// 24px diameter circles centred on undersized targets must not intersect another target
/// or another undersized target's circle. `Xs` sits below the SC 2.5.8 floor, plainly — it does
/// not meet the 24px minimum on its own. The library exposes `Xs` for two uses: a fixed-height
/// dense strip under the spacing exception above (unchanged), and a product that offers a
/// user-selectable Compact density and has accepted that trade-off for the users who choose it.
/// What must actually hold, because it is checkable: a Compact host row is at least the tier's
/// `control_metrics().height` tall with the tier's `gap` between neighbours, so undersized
/// targets do not overlap. This consumer's default density is Standard; Compact is opt-in.
///
/// Tier metrics follow UI zoom only: apply `tokens.ui(value)` to every field, text included,
/// never `tokens.font()` or `font_delta`. A component's `Custom { .. }` size retains its
/// existing text scaling. These are reference values, not already scaled pixels.
///
/// `Sm` / `Md` text and radius match the reviewed checkbox values so controls align on a row;
/// control height is not checkbox box size. Larger tiers extend the stepped scale for future
/// consumers; adding this table does not migrate any component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MoonControlMetrics {
    /// Overall control height.
    pub height: f32,
    /// Text font size.
    pub font_size: f32,
    /// Text line height.
    pub line_height: f32,
    /// Corner radius.
    pub radius: f32,
    /// Padding on each horizontal side.
    pub pad_x: f32,
    /// Spacing between icon and text, or box and label.
    pub gap: f32,
}

impl MoonSize {
    /// Returns this tier's shared control metrics in unscaled design-reference pixels.
    pub const fn control_metrics(self) -> MoonControlMetrics {
        let (height, font_size, line_height, radius, pad_x, gap) = match self {
            Self::Xs => (20., 12., 16., 4., 6., 4.),
            Self::Sm => (24., 14., 20., 4., 8., 8.),
            Self::Md => (32., 16., 24., 6., 12., 12.),
            Self::Lg => (40., 18., 28., 8., 16., 12.),
            Self::Xl => (48., 20., 28., 8., 20., 16.),
            Self::Xxl => (56., 24., 32., 10., 24., 16.),
        };
        MoonControlMetrics {
            height,
            font_size,
            line_height,
            radius,
            pad_x,
            gap,
        }
    }

    /// Returns the supported tier nearest to `self` by number of steps in the size scale.
    ///
    /// Equal distances resolve to the smaller tier. `supported` may be unordered or contain
    /// duplicates; neither changes the result. Distance is ordinal, not measured in pixels.
    ///
    /// # Panics
    ///
    /// Panics if `supported` is empty.
    pub fn nearest(self, supported: &[MoonSize]) -> MoonSize {
        supported
            .iter()
            .copied()
            .min_by_key(|tier| ((self as u8).abs_diff(*tier as u8), *tier))
            .expect("MoonSize::nearest requires at least one supported tier")
    }
}

pub type Size = MoonSize;

pub trait Sizable<S = MoonSize>: Sized {
    fn size(self, size: S) -> Self;
}

pub trait Disableable: Sized {
    fn disabled(self, disabled: bool) -> Self;
}

pub type MoonClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
pub type MoonBoolChangeHandler = Rc<dyn Fn(&bool, &mut Window, &mut App)>;
pub type MoonF32ChangeHandler = Rc<dyn Fn(f32, &mut Window, &mut App)>;
pub type MoonIndexedClickHandler = Rc<dyn Fn(usize, &ClickEvent, &mut Window, &mut App)>;
pub type MoonSelectHandler = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

pub fn init(cx: &mut App) {
    crate::init(cx);
    super::theme::MoonTheme::install(cx);
    super::input::bind_moon_input_keys(cx);
    super::select::bind_moon_select_keys(cx);
    super::text_area::bind_moon_text_area_keys(cx);
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeMode {
    Light,
    #[default]
    Dark,
    System,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Theme {
    pub mode: ThemeMode,
}

#[cfg(test)]
mod tests;
