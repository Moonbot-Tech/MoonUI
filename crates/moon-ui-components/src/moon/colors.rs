//! Colour roles: what a colour is for, resolved for a theme.
//!
//! Components paint with roles such as `text_primary`, `border_secondary` or `bg_brand_solid`
//! rather than with a palette field or a scale step, so a theme can change what a role looks like
//! without touching the component. [`MoonColors::DARK`] and [`MoonColors::LIGHT`] are the colour
//! modes built on the primitive scales. [`MoonColors::from_palette`] resolves the same roles from a
//! legacy [`MoonPalette`], so components can move to roles one call site at a time while the
//! existing themes keep rendering as they do now.

use gpui::App;

use super::{primitives::MoonColor, tokens::MoonPalette};

mod modes;

/// Every colour role, resolved to a concrete colour.
///
/// Names say what the colour is for:
///
/// - `text_*` is ink for words and `fg_*` is ink for icons and other marks. The two often match
///   but are allowed to diverge, so pick by what is drawn.
/// - `border_*` and `bg_*` are strokes and fills. A `_hover` role is the hovered state of the role
///   it extends; an `_on_brand` role is ink for a brand-coloured fill.
/// - `focus_ring*` and `shadow_*` are effect colours.
/// - `alpha_white_*` is the page colour and `alpha_black_*` the ink colour at rising opacity, for
///   overlays that lighten or darken whatever is underneath. The names are fixed across modes: in
///   a dark mode the "white" ramp is near-black.
/// - `utility_<hue>_<step>` are hue ramps for badges, tags and charts. Low steps are fills, high
///   steps are text.
/// - The remaining roles belong to one component and are named after it.
///
/// A role may carry alpha — the shadow and alpha roles always do, and legacy palettes build status
/// fills as tints — so paint it as given rather than assuming it is opaque.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MoonColors {
    // Text: ink for words.
    pub text_primary: MoonColor,
    pub text_primary_on_brand: MoonColor,
    pub text_secondary: MoonColor,
    pub text_secondary_hover: MoonColor,
    pub text_secondary_on_brand: MoonColor,
    pub text_tertiary: MoonColor,
    pub text_tertiary_hover: MoonColor,
    pub text_tertiary_on_brand: MoonColor,
    pub text_quaternary: MoonColor,
    pub text_quaternary_on_brand: MoonColor,
    pub text_white: MoonColor,
    pub text_placeholder: MoonColor,
    pub text_brand_primary: MoonColor,
    pub text_brand_secondary: MoonColor,
    pub text_brand_secondary_hover: MoonColor,
    pub text_brand_tertiary: MoonColor,
    pub text_brand_tertiary_alt: MoonColor,
    pub text_error_primary: MoonColor,
    pub text_error_primary_hover: MoonColor,
    pub text_warning_primary: MoonColor,
    pub text_success_primary: MoonColor,

    // Borders and dividers.
    pub border_primary: MoonColor,
    pub border_secondary: MoonColor,
    pub border_secondary_alt: MoonColor,
    pub border_tertiary: MoonColor,
    pub border_brand: MoonColor,
    pub border_brand_alt: MoonColor,
    pub border_error: MoonColor,
    pub border_error_subtle: MoonColor,

    // Foreground: icons and other non-text marks.
    pub fg_primary: MoonColor,
    pub fg_secondary: MoonColor,
    pub fg_secondary_hover: MoonColor,
    pub fg_tertiary: MoonColor,
    pub fg_tertiary_hover: MoonColor,
    pub fg_quaternary: MoonColor,
    pub fg_quaternary_hover: MoonColor,
    pub fg_white: MoonColor,
    pub fg_brand_primary: MoonColor,
    pub fg_brand_primary_alt: MoonColor,
    pub fg_brand_secondary: MoonColor,
    pub fg_brand_secondary_alt: MoonColor,
    pub fg_brand_secondary_hover: MoonColor,
    pub fg_error_primary: MoonColor,
    pub fg_error_secondary: MoonColor,
    pub fg_warning_primary: MoonColor,
    pub fg_warning_secondary: MoonColor,
    pub fg_success_primary: MoonColor,
    pub fg_success_secondary: MoonColor,

    // Backgrounds and fills.
    pub bg_primary: MoonColor,
    pub bg_primary_alt: MoonColor,
    pub bg_primary_hover: MoonColor,
    pub bg_primary_solid: MoonColor,
    pub bg_secondary: MoonColor,
    pub bg_secondary_alt: MoonColor,
    pub bg_secondary_hover: MoonColor,
    pub bg_secondary_solid: MoonColor,
    pub bg_tertiary: MoonColor,
    pub bg_quaternary: MoonColor,
    pub bg_overlay: MoonColor,
    pub bg_brand_primary: MoonColor,
    pub bg_brand_primary_alt: MoonColor,
    pub bg_brand_secondary: MoonColor,
    pub bg_brand_solid: MoonColor,
    pub bg_brand_solid_hover: MoonColor,
    pub bg_brand_section: MoonColor,
    pub bg_brand_section_subtle: MoonColor,
    pub bg_error_primary: MoonColor,
    pub bg_error_secondary: MoonColor,
    pub bg_error_solid: MoonColor,
    pub bg_error_solid_hover: MoonColor,
    pub bg_warning_primary: MoonColor,
    pub bg_warning_secondary: MoonColor,
    pub bg_warning_solid: MoonColor,
    pub bg_success_primary: MoonColor,
    pub bg_success_secondary: MoonColor,
    pub bg_success_solid: MoonColor,

    // Effects: focus rings and shadow colours.
    pub focus_ring: MoonColor,
    pub focus_ring_error: MoonColor,
    pub shadow_xs: MoonColor,
    pub shadow_sm_01: MoonColor,
    pub shadow_sm_02: MoonColor,
    pub shadow_md_01: MoonColor,
    pub shadow_md_02: MoonColor,
    pub shadow_lg_01: MoonColor,
    pub shadow_lg_02: MoonColor,
    pub shadow_lg_03: MoonColor,
    pub shadow_xl_01: MoonColor,
    pub shadow_xl_02: MoonColor,
    pub shadow_xl_03: MoonColor,
    pub shadow_2xl_01: MoonColor,
    pub shadow_2xl_02: MoonColor,
    pub shadow_3xl_01: MoonColor,
    pub shadow_3xl_02: MoonColor,
    pub shadow_skeumorphic_inner: MoonColor,
    pub shadow_skeumorphic_inner_border: MoonColor,
    pub shadow_main_centre_md: MoonColor,
    pub shadow_main_centre_lg: MoonColor,
    pub shadow_overlay_lg: MoonColor,
    pub shadow_grid_md: MoonColor,

    // Alpha overlays: `alpha_white_*` is the page colour, `alpha_black_*` the ink colour, at rising opacity.
    pub alpha_white_10: MoonColor,
    pub alpha_white_20: MoonColor,
    pub alpha_white_30: MoonColor,
    pub alpha_white_40: MoonColor,
    pub alpha_white_50: MoonColor,
    pub alpha_white_60: MoonColor,
    pub alpha_white_70: MoonColor,
    pub alpha_white_80: MoonColor,
    pub alpha_white_90: MoonColor,
    pub alpha_white_100: MoonColor,
    pub alpha_black_10: MoonColor,
    pub alpha_black_20: MoonColor,
    pub alpha_black_30: MoonColor,
    pub alpha_black_40: MoonColor,
    pub alpha_black_50: MoonColor,
    pub alpha_black_60: MoonColor,
    pub alpha_black_70: MoonColor,
    pub alpha_black_80: MoonColor,
    pub alpha_black_90: MoonColor,
    pub alpha_black_100: MoonColor,

    // Utility hue ramps for badges, tags and charts.
    pub utility_neutral_50: MoonColor,
    pub utility_neutral_100: MoonColor,
    pub utility_neutral_200: MoonColor,
    pub utility_neutral_300: MoonColor,
    pub utility_neutral_400: MoonColor,
    pub utility_neutral_500: MoonColor,
    pub utility_neutral_600: MoonColor,
    pub utility_neutral_700: MoonColor,
    pub utility_neutral_800: MoonColor,
    pub utility_neutral_900: MoonColor,
    pub utility_brand_50: MoonColor,
    pub utility_brand_50_alt: MoonColor,
    pub utility_brand_100: MoonColor,
    pub utility_brand_100_alt: MoonColor,
    pub utility_brand_200: MoonColor,
    pub utility_brand_200_alt: MoonColor,
    pub utility_brand_300: MoonColor,
    pub utility_brand_300_alt: MoonColor,
    pub utility_brand_400: MoonColor,
    pub utility_brand_400_alt: MoonColor,
    pub utility_brand_500: MoonColor,
    pub utility_brand_500_alt: MoonColor,
    pub utility_brand_600: MoonColor,
    pub utility_brand_600_alt: MoonColor,
    pub utility_brand_700: MoonColor,
    pub utility_brand_700_alt: MoonColor,
    pub utility_brand_800: MoonColor,
    pub utility_brand_800_alt: MoonColor,
    pub utility_brand_900: MoonColor,
    pub utility_brand_900_alt: MoonColor,
    pub utility_red_50: MoonColor,
    pub utility_red_100: MoonColor,
    pub utility_red_200: MoonColor,
    pub utility_red_300: MoonColor,
    pub utility_red_400: MoonColor,
    pub utility_red_500: MoonColor,
    pub utility_red_600: MoonColor,
    pub utility_red_700: MoonColor,
    pub utility_orange_50: MoonColor,
    pub utility_orange_100: MoonColor,
    pub utility_orange_200: MoonColor,
    pub utility_orange_300: MoonColor,
    pub utility_orange_400: MoonColor,
    pub utility_orange_500: MoonColor,
    pub utility_orange_600: MoonColor,
    pub utility_orange_700: MoonColor,
    pub utility_amber_50: MoonColor,
    pub utility_amber_100: MoonColor,
    pub utility_amber_200: MoonColor,
    pub utility_amber_300: MoonColor,
    pub utility_amber_400: MoonColor,
    pub utility_amber_500: MoonColor,
    pub utility_amber_600: MoonColor,
    pub utility_amber_700: MoonColor,
    pub utility_yellow_50: MoonColor,
    pub utility_yellow_100: MoonColor,
    pub utility_yellow_200: MoonColor,
    pub utility_yellow_300: MoonColor,
    pub utility_yellow_400: MoonColor,
    pub utility_yellow_500: MoonColor,
    pub utility_yellow_600: MoonColor,
    pub utility_yellow_700: MoonColor,
    pub utility_green_50: MoonColor,
    pub utility_green_100: MoonColor,
    pub utility_green_200: MoonColor,
    pub utility_green_300: MoonColor,
    pub utility_green_400: MoonColor,
    pub utility_green_500: MoonColor,
    pub utility_green_600: MoonColor,
    pub utility_green_700: MoonColor,
    pub utility_emerald_50: MoonColor,
    pub utility_emerald_100: MoonColor,
    pub utility_emerald_200: MoonColor,
    pub utility_emerald_300: MoonColor,
    pub utility_emerald_400: MoonColor,
    pub utility_emerald_500: MoonColor,
    pub utility_emerald_600: MoonColor,
    pub utility_emerald_700: MoonColor,
    pub utility_sky_50: MoonColor,
    pub utility_sky_100: MoonColor,
    pub utility_sky_200: MoonColor,
    pub utility_sky_300: MoonColor,
    pub utility_sky_400: MoonColor,
    pub utility_sky_500: MoonColor,
    pub utility_sky_600: MoonColor,
    pub utility_sky_700: MoonColor,
    pub utility_blue_50: MoonColor,
    pub utility_blue_100: MoonColor,
    pub utility_blue_200: MoonColor,
    pub utility_blue_300: MoonColor,
    pub utility_blue_400: MoonColor,
    pub utility_blue_500: MoonColor,
    pub utility_blue_600: MoonColor,
    pub utility_blue_700: MoonColor,
    pub utility_indigo_50: MoonColor,
    pub utility_indigo_100: MoonColor,
    pub utility_indigo_200: MoonColor,
    pub utility_indigo_300: MoonColor,
    pub utility_indigo_400: MoonColor,
    pub utility_indigo_500: MoonColor,
    pub utility_indigo_600: MoonColor,
    pub utility_indigo_700: MoonColor,
    pub utility_purple_50: MoonColor,
    pub utility_purple_100: MoonColor,
    pub utility_purple_200: MoonColor,
    pub utility_purple_300: MoonColor,
    pub utility_purple_400: MoonColor,
    pub utility_purple_500: MoonColor,
    pub utility_purple_600: MoonColor,
    pub utility_purple_700: MoonColor,
    pub utility_fuchsia_50: MoonColor,
    pub utility_fuchsia_100: MoonColor,
    pub utility_fuchsia_200: MoonColor,
    pub utility_fuchsia_300: MoonColor,
    pub utility_fuchsia_400: MoonColor,
    pub utility_fuchsia_500: MoonColor,
    pub utility_fuchsia_600: MoonColor,
    pub utility_fuchsia_700: MoonColor,
    pub utility_pink_50: MoonColor,
    pub utility_pink_100: MoonColor,
    pub utility_pink_200: MoonColor,
    pub utility_pink_300: MoonColor,
    pub utility_pink_400: MoonColor,
    pub utility_pink_500: MoonColor,
    pub utility_pink_600: MoonColor,
    pub utility_pink_700: MoonColor,
    pub utility_slate_50: MoonColor,
    pub utility_slate_100: MoonColor,
    pub utility_slate_200: MoonColor,
    pub utility_slate_300: MoonColor,
    pub utility_slate_400: MoonColor,
    pub utility_slate_500: MoonColor,
    pub utility_slate_600: MoonColor,
    pub utility_slate_700: MoonColor,

    // Component-specific roles.
    pub app_store_badge_border: MoonColor,
    pub avatar_styles_bg_neutral: MoonColor,
    pub footer_button_fg: MoonColor,
    pub footer_button_fg_hover: MoonColor,
    pub icon_fg_brand: MoonColor,
    pub icon_fg_brand_on_brand: MoonColor,
    pub featured_icon_light_fg_brand: MoonColor,
    pub featured_icon_light_fg_gray: MoonColor,
    pub featured_icon_light_fg_error: MoonColor,
    pub featured_icon_light_fg_warning: MoonColor,
    pub featured_icon_light_fg_success: MoonColor,
    pub screen_mockup_border: MoonColor,
    pub slider_handle_bg: MoonColor,
    pub slider_handle_border: MoonColor,
    pub toggle_border: MoonColor,
    pub toggle_slim_border_pressed: MoonColor,
    pub toggle_slim_border_pressed_hover: MoonColor,
    pub tooltip_supporting_text: MoonColor,
    pub text_editor_icon_fg: MoonColor,
    pub text_editor_icon_fg_active: MoonColor,
}

/// Opacity of each utility ramp step, 50 through 900, when a legacy palette paints one of its own
/// hues over the surface: faint washes at the fill steps, the full hue from step 700 up, where the
/// ramp is used for text.
const LEGACY_UTILITY_ALPHAS: [f32; 10] = [0.10, 0.16, 0.24, 0.36, 0.52, 0.70, 0.86, 1.0, 1.0, 1.0];

/// Tint strength of the hover overlay legacy components lay over a row or button. Components vary
/// between 0.018 and 0.055; this is the middle of that range.
const LEGACY_HOVER_ALPHA: f32 = 0.04;

impl MoonColors {
    /// Resolve every role from a legacy palette.
    ///
    /// Where a role has a counterpart in today's rendering, it takes the colour components paint
    /// for that counterpart now: the same palette field, the same tint alpha and the same
    /// light/dark branch as the base-theme bridge and [`MoonTone`](super::MoonTone). Moving such a
    /// call site from a palette field to the role changes nothing on screen.
    ///
    /// Roles the palettes never had a colour for are filled in three ways:
    ///
    /// - Hover, alternate and on-brand variants reuse the nearest base role, because a palette has
    ///   one ink per purpose and no separate hover shade.
    /// - Status fills, overlays and the utility ramps of hues the palette has are that hue at a
    ///   rising alpha, so they follow the palette's own accent and status colours over any surface.
    /// - Hues no palette defines (emerald, sky, indigo, purple, fuchsia, pink, slate) and the
    ///   app-store badge border take the colour mode of the same side: [`Self::LIGHT`] for a light
    ///   palette, [`Self::DARK`] otherwise.
    ///
    /// Shadows keep today's single-layer look: the first layer of each size carries the strength
    /// overlays use now (0.30 for avatars, 0.46 for menus and tooltips, 0.48 for popovers, sizes in
    /// between interpolated) and the extra layers stay transparent.
    ///
    /// Args:
    ///     palette: The legacy palette to resolve against.
    ///
    /// Returns:
    ///     Every role as that palette paints it.
    pub fn from_palette(palette: MoonPalette) -> Self {
        let p = palette;
        let light = p.is_light();
        let side = if light { Self::LIGHT } else { Self::DARK };

        // The branches the base-theme bridge and `MoonTone` already take.
        let on_brand = solid(p.ink_on(p.accent));
        let brand_text = solid(if light { p.accent_fg } else { p.accent });
        let danger_text = solid(if light { p.red_text } else { p.red });
        let success_text = solid(if light { p.green_text } else { p.green });
        let success_fill = solid(if light { p.green_btn } else { p.green });
        let ring = solid(if light { p.accent } else { p.blue });
        let brand_tint = tint(p.accent, p.accent_tint_a);

        let neutral = legacy_utility_ramp(p.text);
        let brand = legacy_utility_ramp(p.accent);
        // The modes alias the dark side's alternate brand ramp to the neutral one.
        let brand_alt = if light { brand } else { neutral };
        let red = legacy_utility_ramp(p.red);
        let orange = legacy_utility_ramp(p.orange);
        let amber = legacy_utility_ramp(p.amber);
        let yellow = legacy_utility_ramp(p.yellow);
        let green = legacy_utility_ramp(p.green);
        let blue = legacy_utility_ramp(p.blue);

        Self {
            // Text
            text_primary: solid(p.text),
            text_primary_on_brand: on_brand,
            text_secondary: solid(p.text_soft),
            text_secondary_hover: solid(p.text),
            text_secondary_on_brand: on_brand,
            text_tertiary: solid(p.text_muted),
            text_tertiary_hover: solid(p.text_soft),
            text_tertiary_on_brand: on_brand,
            text_quaternary: solid(p.text_faint),
            text_quaternary_on_brand: on_brand,
            text_white: solid(p.on_accent),
            text_placeholder: solid(p.text_muted),
            text_brand_primary: brand_text,
            text_brand_secondary: brand_text,
            text_brand_secondary_hover: brand_text,
            text_brand_tertiary: brand_text,
            text_brand_tertiary_alt: brand_text,
            text_error_primary: danger_text,
            text_error_primary_hover: danger_text,
            text_warning_primary: solid(p.amber),
            text_success_primary: success_text,

            // Borders
            border_primary: solid(p.border),
            border_secondary: solid(p.border_soft),
            border_secondary_alt: solid(p.border_card),
            border_tertiary: solid(p.row_line),
            border_brand: solid(p.accent),
            border_brand_alt: solid(p.accent),
            border_error: solid(p.red),
            border_error_subtle: solid(p.red_soft_bd),

            // Foreground
            fg_primary: solid(p.text),
            fg_secondary: solid(p.text_soft),
            fg_secondary_hover: solid(p.text),
            fg_tertiary: solid(p.text_muted),
            fg_tertiary_hover: solid(p.text_soft),
            fg_quaternary: solid(p.text_faint),
            fg_quaternary_hover: solid(p.text_muted),
            fg_white: solid(p.on_accent),
            fg_brand_primary: solid(p.accent),
            fg_brand_primary_alt: solid(p.accent),
            fg_brand_secondary: solid(p.accent),
            fg_brand_secondary_alt: solid(p.accent),
            fg_brand_secondary_hover: tint(p.accent, 0.82),
            fg_error_primary: danger_text,
            fg_error_secondary: solid(p.red),
            fg_warning_primary: solid(p.amber),
            fg_warning_secondary: solid(p.amber),
            fg_success_primary: success_fill,
            fg_success_secondary: solid(p.green),

            // Backgrounds
            bg_primary: solid(p.shell),
            bg_primary_alt: solid(p.shell_high),
            bg_primary_hover: tint(p.overlay, LEGACY_HOVER_ALPHA),
            // Tooltips and popovers are raised panels in the legacy themes, not an inverted fill.
            bg_primary_solid: solid(p.shell_high),
            bg_secondary: solid(p.panel),
            bg_secondary_alt: solid(p.panel_high),
            bg_secondary_hover: solid(p.panel_head),
            bg_secondary_solid: solid(p.text_muted),
            bg_tertiary: solid(p.panel_head),
            bg_quaternary: solid(p.border),
            bg_overlay: solid(p.shell),
            bg_brand_primary: brand_tint,
            bg_brand_primary_alt: brand_tint,
            bg_brand_secondary: tint(p.accent, 0.18),
            bg_brand_solid: solid(p.accent),
            bg_brand_solid_hover: tint(p.accent, 0.82),
            bg_brand_section: solid(p.accent),
            bg_brand_section_subtle: brand_tint,
            bg_error_primary: tint(p.red, 0.12),
            bg_error_secondary: tint(p.red, 0.18),
            bg_error_solid: solid(p.red),
            bg_error_solid_hover: tint(p.red, 0.72),
            bg_warning_primary: tint(p.amber, 0.12),
            bg_warning_secondary: tint(p.amber, 0.18),
            bg_warning_solid: solid(p.amber),
            bg_success_primary: tint(p.green, 0.12),
            bg_success_secondary: tint(p.green, 0.18),
            bg_success_solid: success_fill,

            // Effects
            focus_ring: ring,
            focus_ring_error: solid(p.red),
            shadow_xs: tint(p.shadow, 0.30),
            // The `sm` pair is the one small controls carry, and a toggle's thumb is the first to ask
            // for it. It takes the colour modes' own strength rather than the overlay strengths the
            // larger sizes inherit, which under a 16px thumb read as a smear across its track.
            shadow_sm_01: tint(p.shadow, 0.10),
            shadow_sm_02: tint(p.shadow, 0.10),
            shadow_md_01: tint(p.shadow, 0.42),
            shadow_md_02: MoonColor::TRANSPARENT,
            shadow_lg_01: tint(p.shadow, 0.46),
            shadow_lg_02: MoonColor::TRANSPARENT,
            shadow_lg_03: MoonColor::TRANSPARENT,
            shadow_xl_01: tint(p.shadow, 0.46),
            shadow_xl_02: MoonColor::TRANSPARENT,
            shadow_xl_03: MoonColor::TRANSPARENT,
            shadow_2xl_01: tint(p.shadow, 0.48),
            shadow_2xl_02: MoonColor::TRANSPARENT,
            shadow_3xl_01: tint(p.shadow, 0.48),
            shadow_3xl_02: MoonColor::TRANSPARENT,
            shadow_skeumorphic_inner: tint(p.shadow, 0.05),
            shadow_skeumorphic_inner_border: tint(p.shadow, 0.18),
            shadow_main_centre_md: tint(p.shadow, 0.42),
            shadow_main_centre_lg: tint(p.shadow, 0.46),
            shadow_overlay_lg: tint(p.shadow, 0.46),
            shadow_grid_md: tint(p.shadow, 0.42),

            // Alpha overlays
            alpha_white_10: tint(p.shell, 0.1),
            alpha_white_20: tint(p.shell, 0.2),
            alpha_white_30: tint(p.shell, 0.3),
            alpha_white_40: tint(p.shell, 0.4),
            alpha_white_50: tint(p.shell, 0.5),
            alpha_white_60: tint(p.shell, 0.6),
            alpha_white_70: tint(p.shell, 0.7),
            alpha_white_80: tint(p.shell, 0.8),
            alpha_white_90: tint(p.shell, 0.9),
            alpha_white_100: solid(p.shell),
            alpha_black_10: tint(p.overlay, 0.1),
            alpha_black_20: tint(p.overlay, 0.2),
            alpha_black_30: tint(p.overlay, 0.3),
            alpha_black_40: tint(p.overlay, 0.4),
            alpha_black_50: tint(p.overlay, 0.5),
            alpha_black_60: tint(p.overlay, 0.6),
            alpha_black_70: tint(p.overlay, 0.7),
            alpha_black_80: tint(p.overlay, 0.8),
            alpha_black_90: tint(p.overlay, 0.9),
            alpha_black_100: solid(p.overlay),

            // Utility hue ramps
            utility_neutral_50: neutral[0],
            utility_neutral_100: neutral[1],
            utility_neutral_200: neutral[2],
            utility_neutral_300: neutral[3],
            utility_neutral_400: neutral[4],
            utility_neutral_500: neutral[5],
            utility_neutral_600: neutral[6],
            utility_neutral_700: neutral[7],
            utility_neutral_800: neutral[8],
            utility_neutral_900: neutral[9],
            utility_brand_50: brand[0],
            utility_brand_50_alt: brand_alt[0],
            utility_brand_100: brand[1],
            utility_brand_100_alt: brand_alt[1],
            utility_brand_200: brand[2],
            utility_brand_200_alt: brand_alt[2],
            utility_brand_300: brand[3],
            utility_brand_300_alt: brand_alt[3],
            utility_brand_400: brand[4],
            utility_brand_400_alt: brand_alt[4],
            utility_brand_500: brand[5],
            utility_brand_500_alt: brand_alt[5],
            utility_brand_600: brand[6],
            utility_brand_600_alt: brand_alt[6],
            utility_brand_700: brand[7],
            utility_brand_700_alt: brand_alt[7],
            utility_brand_800: brand[8],
            utility_brand_800_alt: brand_alt[8],
            utility_brand_900: brand[9],
            utility_brand_900_alt: brand_alt[9],
            utility_red_50: red[0],
            utility_red_100: red[1],
            utility_red_200: red[2],
            utility_red_300: red[3],
            utility_red_400: red[4],
            utility_red_500: red[5],
            utility_red_600: red[6],
            utility_red_700: red[7],
            utility_orange_50: orange[0],
            utility_orange_100: orange[1],
            utility_orange_200: orange[2],
            utility_orange_300: orange[3],
            utility_orange_400: orange[4],
            utility_orange_500: orange[5],
            utility_orange_600: orange[6],
            utility_orange_700: orange[7],
            utility_amber_50: amber[0],
            utility_amber_100: amber[1],
            utility_amber_200: amber[2],
            utility_amber_300: amber[3],
            utility_amber_400: amber[4],
            utility_amber_500: amber[5],
            utility_amber_600: amber[6],
            utility_amber_700: amber[7],
            utility_yellow_50: yellow[0],
            utility_yellow_100: yellow[1],
            utility_yellow_200: yellow[2],
            utility_yellow_300: yellow[3],
            utility_yellow_400: yellow[4],
            utility_yellow_500: yellow[5],
            utility_yellow_600: yellow[6],
            utility_yellow_700: yellow[7],
            utility_green_50: green[0],
            utility_green_100: green[1],
            utility_green_200: green[2],
            utility_green_300: green[3],
            utility_green_400: green[4],
            utility_green_500: green[5],
            utility_green_600: green[6],
            utility_green_700: green[7],
            utility_emerald_50: side.utility_emerald_50,
            utility_emerald_100: side.utility_emerald_100,
            utility_emerald_200: side.utility_emerald_200,
            utility_emerald_300: side.utility_emerald_300,
            utility_emerald_400: side.utility_emerald_400,
            utility_emerald_500: side.utility_emerald_500,
            utility_emerald_600: side.utility_emerald_600,
            utility_emerald_700: side.utility_emerald_700,
            utility_sky_50: side.utility_sky_50,
            utility_sky_100: side.utility_sky_100,
            utility_sky_200: side.utility_sky_200,
            utility_sky_300: side.utility_sky_300,
            utility_sky_400: side.utility_sky_400,
            utility_sky_500: side.utility_sky_500,
            utility_sky_600: side.utility_sky_600,
            utility_sky_700: side.utility_sky_700,
            utility_blue_50: blue[0],
            utility_blue_100: blue[1],
            utility_blue_200: blue[2],
            utility_blue_300: blue[3],
            utility_blue_400: blue[4],
            utility_blue_500: blue[5],
            utility_blue_600: blue[6],
            utility_blue_700: blue[7],
            utility_indigo_50: side.utility_indigo_50,
            utility_indigo_100: side.utility_indigo_100,
            utility_indigo_200: side.utility_indigo_200,
            utility_indigo_300: side.utility_indigo_300,
            utility_indigo_400: side.utility_indigo_400,
            utility_indigo_500: side.utility_indigo_500,
            utility_indigo_600: side.utility_indigo_600,
            utility_indigo_700: side.utility_indigo_700,
            utility_purple_50: side.utility_purple_50,
            utility_purple_100: side.utility_purple_100,
            utility_purple_200: side.utility_purple_200,
            utility_purple_300: side.utility_purple_300,
            utility_purple_400: side.utility_purple_400,
            utility_purple_500: side.utility_purple_500,
            utility_purple_600: side.utility_purple_600,
            utility_purple_700: side.utility_purple_700,
            utility_fuchsia_50: side.utility_fuchsia_50,
            utility_fuchsia_100: side.utility_fuchsia_100,
            utility_fuchsia_200: side.utility_fuchsia_200,
            utility_fuchsia_300: side.utility_fuchsia_300,
            utility_fuchsia_400: side.utility_fuchsia_400,
            utility_fuchsia_500: side.utility_fuchsia_500,
            utility_fuchsia_600: side.utility_fuchsia_600,
            utility_fuchsia_700: side.utility_fuchsia_700,
            utility_pink_50: side.utility_pink_50,
            utility_pink_100: side.utility_pink_100,
            utility_pink_200: side.utility_pink_200,
            utility_pink_300: side.utility_pink_300,
            utility_pink_400: side.utility_pink_400,
            utility_pink_500: side.utility_pink_500,
            utility_pink_600: side.utility_pink_600,
            utility_pink_700: side.utility_pink_700,
            utility_slate_50: side.utility_slate_50,
            utility_slate_100: side.utility_slate_100,
            utility_slate_200: side.utility_slate_200,
            utility_slate_300: side.utility_slate_300,
            utility_slate_400: side.utility_slate_400,
            utility_slate_500: side.utility_slate_500,
            utility_slate_600: side.utility_slate_600,
            utility_slate_700: side.utility_slate_700,

            // Component-specific roles
            app_store_badge_border: side.app_store_badge_border,
            avatar_styles_bg_neutral: solid(p.panel_head),
            footer_button_fg: solid(p.text_soft),
            footer_button_fg_hover: solid(p.text),
            icon_fg_brand: solid(p.accent),
            icon_fg_brand_on_brand: on_brand,
            featured_icon_light_fg_brand: solid(p.accent),
            featured_icon_light_fg_gray: solid(p.text_muted),
            featured_icon_light_fg_error: danger_text,
            featured_icon_light_fg_warning: solid(p.amber),
            featured_icon_light_fg_success: success_text,
            screen_mockup_border: solid(p.border),
            slider_handle_bg: solid(p.accent),
            slider_handle_border: solid(p.shell),
            toggle_border: solid(p.border),
            toggle_slim_border_pressed: solid(p.accent),
            toggle_slim_border_pressed_hover: tint(p.accent, 0.82),
            tooltip_supporting_text: solid(p.text_soft),
            text_editor_icon_fg: solid(p.text_soft),
            text_editor_icon_fg_active: solid(p.text),
        }
    }

    /// Derive a legacy palette that paints like these roles.
    ///
    /// This is what lets a colour mode drive a whole interface while most components still read
    /// palette fields: each field takes the role that plays its part, so surfaces, borders, inks
    /// and status colours match the roles that migrated components paint with. Only opaque roles
    /// are used, since a palette field has no alpha. `accent_tint_a`, which is not a colour, keeps
    /// the strength the bundled palette of the same side uses.
    ///
    /// Returns:
    ///     The palette to install alongside these roles.
    pub fn to_palette(self) -> MoonPalette {
        let hex = |color: MoonColor| color.rgb_hex();
        let mut palette = MoonPalette {
            shell: hex(self.bg_primary),
            shell_high: hex(self.bg_secondary),
            window: hex(self.bg_primary),
            surface: hex(self.bg_primary_alt),
            panel: hex(self.bg_secondary),
            panel_high: hex(self.bg_primary_alt),
            chrome: hex(self.bg_secondary),
            tabbar: hex(self.bg_secondary),
            panel_head: hex(self.bg_secondary_hover),
            gutter: hex(self.bg_secondary_alt),
            chart_bg: hex(self.bg_primary),
            card: hex(self.bg_primary_alt),
            row_alt: hex(self.bg_secondary_alt),
            head_row: hex(self.bg_tertiary),
            border: hex(self.border_primary),
            border_soft: hex(self.border_secondary),
            border_card: hex(self.border_secondary),
            border_hover: hex(self.fg_quaternary),
            row_line: hex(self.border_tertiary),
            shadow: hex(MoonColor::BLACK),
            // The ink overlays lighten a dark theme and darken a light one, as the opaque end of
            // the ink ramp does.
            overlay: hex(self.alpha_black_100),
            on_accent: hex(self.text_white),
            text: hex(self.text_primary),
            text_soft: hex(self.text_secondary),
            text_dim: hex(self.text_secondary_hover),
            text_muted: hex(self.text_tertiary),
            text_faint: hex(self.text_quaternary),
            table_head: hex(self.bg_tertiary),
            table_body: hex(self.bg_primary_alt),
            table_selected: hex(self.bg_brand_solid),
            // One step off the row fill on either side, so a hovered row stays visible.
            table_hover: hex(self.bg_secondary_hover),
            green: hex(self.fg_success_primary),
            green_btn: hex(self.bg_success_solid),
            green_text: hex(self.text_success_primary),
            red: hex(self.fg_error_primary),
            red_text: hex(self.text_error_primary),
            red_soft_bd: hex(self.border_error_subtle),
            orange: hex(self.utility_orange_500),
            amber: hex(self.fg_warning_primary),
            blue: hex(self.utility_blue_600),
            accent: hex(self.bg_brand_solid),
            accent_fg: hex(self.text_brand_secondary),
            accent_tint_a: 0.0,
            yellow: hex(self.fg_warning_secondary),
        };
        palette.accent_tint_a = if palette.is_light() {
            MoonPalette::LIGHT.accent_tint_a
        } else {
            MoonPalette::TERMINAL.accent_tint_a
        };
        palette
    }

    /// The roles for the active theme.
    ///
    /// A theme installed with roles, such as [`crate::moon::MoonThemeConfig::moon_color_modes`],
    /// answers with those roles as given. Otherwise the roles are resolved from the active palette
    /// on every call rather than stored: code and tests replace `MoonTheme::palette` in place, and
    /// a stored copy would keep painting the palette it replaced.
    ///
    /// Args:
    ///     cx: The app holding the installed theme.
    ///
    /// Returns:
    ///     The installed roles, or [`Self::from_palette`] of the active palette, or of the default
    ///     palette when no theme is installed.
    pub fn active(cx: &App) -> Self {
        super::theme::MoonTheme::global(cx)
            .and_then(|theme| theme.colors)
            .unwrap_or_else(|| Self::from_palette(MoonPalette::active(cx)))
    }
}

/// An opaque colour from a legacy `0xRRGGBB` palette value.
///
/// Args:
///     hex: The palette value.
///
/// Returns:
///     The colour at full alpha, packed as [`tint`] packs it.
fn solid(hex: u32) -> MoonColor {
    tint(hex, 1.0)
}

/// A legacy `0xRRGGBB` palette value at `alpha`.
///
/// Packed exactly like `rgba_from`, including shifting out any byte above the channels instead of
/// rejecting it, so a palette loaded from a theme file paints the same through a role as through
/// the palette field.
///
/// Args:
///     hex: The palette value.
///     alpha: Opacity from `0.0` to `1.0`.
///
/// Returns:
///     The colour with that alpha.
fn tint(hex: u32, alpha: f32) -> MoonColor {
    MoonColor::rgba((hex << 8) | (alpha * 255.0).round() as u32)
}

/// A utility ramp built from one legacy palette hue.
///
/// Args:
///     hex: The palette hue.
///
/// Returns:
///     Steps 50 through 900 as that hue at [`LEGACY_UTILITY_ALPHAS`].
fn legacy_utility_ramp(hex: u32) -> [MoonColor; 10] {
    LEGACY_UTILITY_ALPHAS.map(|alpha| tint(hex, alpha))
}

#[cfg(test)]
mod tests;
