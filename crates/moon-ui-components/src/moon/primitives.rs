//! Primitive colour scales.
//!
//! These are the raw ramps a palette is built from, not colours a component paints with. A widget
//! that reads `MoonColorScale::RED.c500` directly is pinned to one hue in every theme and stops
//! following the active palette, so components keep reading the theme's role colours and palettes
//! map those roles onto these steps.

use gpui::{Hsla, Rgba, rgba};

/// An sRGB colour with straight (not premultiplied) alpha, packed as `0xRRGGBBAA`.
///
/// A distinct type rather than a bare `u32` because this crate already passes colours around as
/// `0xRRGGBB` integers (`MoonPalette`, `rgba_from`), and the two layouts cannot be told apart at a
/// call site: `0xFFFFFF00` is transparent white here but opaque yellow through `gpui::rgb`. The
/// wrapper keeps an alpha-carrying colour from reaching an RGB-only helper unnoticed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MoonColor(u32);

impl MoonColor {
    /// Opaque white.
    pub const WHITE: Self = Self::rgb(0xFFFFFF);

    /// Opaque black.
    pub const BLACK: Self = Self::rgb(0x000000);

    /// White at zero alpha.
    pub const TRANSPARENT: Self = Self::rgba(0xFFFFFF00);

    /// Build an opaque colour from `0xRRGGBB`.
    ///
    /// Args:
    ///     hex: The colour channels as `0xRRGGBB`.
    ///
    /// Returns:
    ///     The colour with full alpha.
    ///
    /// Panics:
    ///     If `hex` has bits above the low 24, which means an `0xRRGGBBAA` value was passed by
    ///     mistake. Inside a `const` item the panic is a compile error.
    pub const fn rgb(hex: u32) -> Self {
        assert!(
            hex <= 0xFF_FFFF,
            "MoonColor::rgb takes 0xRRGGBB; use MoonColor::rgba for a colour with alpha"
        );
        Self((hex << 8) | 0xFF)
    }

    /// Build a colour from `0xRRGGBBAA` with straight alpha.
    ///
    /// Args:
    ///     hex: The colour as `0xRRGGBBAA`; `0x00` alpha is transparent, `0xFF` opaque.
    ///
    /// Returns:
    ///     The colour exactly as packed.
    pub const fn rgba(hex: u32) -> Self {
        Self(hex)
    }

    /// The colour channels as `0xRRGGBB`, with alpha dropped.
    ///
    /// This is the layout `MoonPalette` and the WCAG contrast helpers take. A translucent colour
    /// yields the channels it tints with, not the colour it appears as over a surface.
    ///
    /// Returns:
    ///     The red, green and blue channels as `0xRRGGBB`.
    pub const fn rgb_hex(self) -> u32 {
        self.0 >> 8
    }

    /// The alpha channel.
    ///
    /// Returns:
    ///     Opacity from `0.0` (transparent) to `1.0` (opaque).
    pub fn alpha(self) -> f32 {
        (self.0 & 0xFF) as f32 / 255.0
    }
}

impl From<MoonColor> for Rgba {
    /// Convert through GPUI's own `0xRRGGBBAA` decoder, so both sides agree on byte order.
    fn from(color: MoonColor) -> Self {
        rgba(color.0)
    }
}

impl From<MoonColor> for Hsla {
    /// Convert through [`Rgba`], the path GPUI itself uses for hex colours.
    fn from(color: MoonColor) -> Self {
        Rgba::from(color).into()
    }
}

/// One primitive ramp: eleven steps from `c50`, the lightest, to `c950`, the darkest.
///
/// A Rust field cannot be a bare number, so each step carries a `c` prefix: step 500 of the red
/// scale is `MoonColorScale::RED.c500`.
///
/// [`Self::NEUTRAL_ALPHA`] is the one ramp that runs by opacity instead of lightness: every step
/// is white, fading from nearly opaque at `c50` to fully transparent at `c950`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MoonColorScale {
    pub c50: MoonColor,
    pub c100: MoonColor,
    pub c200: MoonColor,
    pub c300: MoonColor,
    pub c400: MoonColor,
    pub c500: MoonColor,
    pub c600: MoonColor,
    pub c700: MoonColor,
    pub c800: MoonColor,
    pub c900: MoonColor,
    pub c950: MoonColor,
}

impl MoonColorScale {
    /// The neutral scale.
    pub const NEUTRAL: Self = Self {
        c50: MoonColor::rgb(0xFAFAFA),
        c100: MoonColor::rgb(0xF5F5F5),
        c200: MoonColor::rgb(0xE5E5E5),
        c300: MoonColor::rgb(0xD4D4D4),
        c400: MoonColor::rgb(0xA3A3A3),
        c500: MoonColor::rgb(0x737373),
        c600: MoonColor::rgb(0x525252),
        c700: MoonColor::rgb(0x404040),
        c800: MoonColor::rgb(0x262626),
        c900: MoonColor::rgb(0x171717),
        c950: MoonColor::rgb(0x0A0A0A),
    };

    /// The neutral scale as white at falling opacity.
    pub const NEUTRAL_ALPHA: Self = Self {
        c50: MoonColor::rgba(0xFFFFFFF7),
        c100: MoonColor::rgba(0xFFFFFFF5),
        c200: MoonColor::rgba(0xFFFFFFE5),
        c300: MoonColor::rgba(0xFFFFFFD1),
        c400: MoonColor::rgba(0xFFFFFF9E),
        c500: MoonColor::rgba(0xFFFFFF6E),
        c600: MoonColor::rgba(0xFFFFFF4A),
        c700: MoonColor::rgba(0xFFFFFF38),
        c800: MoonColor::rgba(0xFFFFFF1F),
        c900: MoonColor::rgba(0xFFFFFF0D),
        c950: MoonColor::rgba(0xFFFFFF00),
    };

    /// The brand scale.
    pub const BRAND: Self = Self {
        c50: MoonColor::rgb(0xEFF6FF),
        c100: MoonColor::rgb(0xDBEAFE),
        c200: MoonColor::rgb(0xBFDBFE),
        c300: MoonColor::rgb(0x93C5FD),
        c400: MoonColor::rgb(0x60A5FA),
        c500: MoonColor::rgb(0x3B82F6),
        c600: MoonColor::rgb(0x2563EB),
        c700: MoonColor::rgb(0x1D4ED8),
        c800: MoonColor::rgb(0x1E40AF),
        c900: MoonColor::rgb(0x1E3A8A),
        c950: MoonColor::rgb(0x172554),
    };

    /// The red scale.
    pub const RED: Self = Self {
        c50: MoonColor::rgb(0xFEF2F2),
        c100: MoonColor::rgb(0xFEE2E2),
        c200: MoonColor::rgb(0xFECACA),
        c300: MoonColor::rgb(0xFCA5A5),
        c400: MoonColor::rgb(0xF87171),
        c500: MoonColor::rgb(0xEF4444),
        c600: MoonColor::rgb(0xDC2626),
        c700: MoonColor::rgb(0xB91C1C),
        c800: MoonColor::rgb(0x991B1B),
        c900: MoonColor::rgb(0x7F1D1D),
        c950: MoonColor::rgb(0x450A0A),
    };

    /// The orange scale.
    pub const ORANGE: Self = Self {
        c50: MoonColor::rgb(0xFFF7ED),
        c100: MoonColor::rgb(0xFFEDD5),
        c200: MoonColor::rgb(0xFED7AA),
        c300: MoonColor::rgb(0xFDBA74),
        c400: MoonColor::rgb(0xFB923C),
        c500: MoonColor::rgb(0xF97316),
        c600: MoonColor::rgb(0xEA580C),
        c700: MoonColor::rgb(0xC2410C),
        c800: MoonColor::rgb(0x9A3412),
        c900: MoonColor::rgb(0x7C2D12),
        c950: MoonColor::rgb(0x431407),
    };

    /// The amber scale.
    pub const AMBER: Self = Self {
        c50: MoonColor::rgb(0xFFFBEB),
        c100: MoonColor::rgb(0xFEF3C7),
        c200: MoonColor::rgb(0xFDE68A),
        c300: MoonColor::rgb(0xFCD34D),
        c400: MoonColor::rgb(0xFBBF24),
        c500: MoonColor::rgb(0xF59E0B),
        c600: MoonColor::rgb(0xD97706),
        c700: MoonColor::rgb(0xB45309),
        c800: MoonColor::rgb(0x92400E),
        c900: MoonColor::rgb(0x78350F),
        c950: MoonColor::rgb(0x451A03),
    };

    /// The yellow scale.
    pub const YELLOW: Self = Self {
        c50: MoonColor::rgb(0xFEFCE8),
        c100: MoonColor::rgb(0xFEF9C3),
        c200: MoonColor::rgb(0xFEF08A),
        c300: MoonColor::rgb(0xFDE047),
        c400: MoonColor::rgb(0xFACC15),
        c500: MoonColor::rgb(0xEAB308),
        c600: MoonColor::rgb(0xCA8A04),
        c700: MoonColor::rgb(0xA16207),
        c800: MoonColor::rgb(0x854D0E),
        c900: MoonColor::rgb(0x713F12),
        c950: MoonColor::rgb(0x422006),
    };

    /// The lime scale.
    pub const LIME: Self = Self {
        c50: MoonColor::rgb(0xF7FEE7),
        c100: MoonColor::rgb(0xECFCCB),
        c200: MoonColor::rgb(0xD9F99D),
        c300: MoonColor::rgb(0xBEF264),
        c400: MoonColor::rgb(0xA3E635),
        c500: MoonColor::rgb(0x84CC16),
        c600: MoonColor::rgb(0x65A30D),
        c700: MoonColor::rgb(0x4D7C0F),
        c800: MoonColor::rgb(0x3F6212),
        c900: MoonColor::rgb(0x365314),
        c950: MoonColor::rgb(0x1A2E05),
    };

    /// The green scale.
    pub const GREEN: Self = Self {
        c50: MoonColor::rgb(0xF0FDF4),
        c100: MoonColor::rgb(0xDCFCE7),
        c200: MoonColor::rgb(0xBBF7D0),
        c300: MoonColor::rgb(0x86EFAC),
        c400: MoonColor::rgb(0x4ADE80),
        c500: MoonColor::rgb(0x22C55E),
        c600: MoonColor::rgb(0x16A34A),
        c700: MoonColor::rgb(0x15803D),
        c800: MoonColor::rgb(0x166534),
        c900: MoonColor::rgb(0x14532D),
        c950: MoonColor::rgb(0x052E16),
    };

    /// The emerald scale.
    pub const EMERALD: Self = Self {
        c50: MoonColor::rgb(0xECFDF5),
        c100: MoonColor::rgb(0xD1FAE5),
        c200: MoonColor::rgb(0xA7F3D0),
        c300: MoonColor::rgb(0x6EE7B7),
        c400: MoonColor::rgb(0x34D399),
        c500: MoonColor::rgb(0x10B981),
        c600: MoonColor::rgb(0x059669),
        c700: MoonColor::rgb(0x047857),
        c800: MoonColor::rgb(0x065F46),
        c900: MoonColor::rgb(0x064E3B),
        c950: MoonColor::rgb(0x022C22),
    };

    /// The teal scale.
    pub const TEAL: Self = Self {
        c50: MoonColor::rgb(0xF0FDFA),
        c100: MoonColor::rgb(0xCCFBF1),
        c200: MoonColor::rgb(0x99F6E4),
        c300: MoonColor::rgb(0x5EEAD4),
        c400: MoonColor::rgb(0x2DD4BF),
        c500: MoonColor::rgb(0x14B8A6),
        c600: MoonColor::rgb(0x0D9488),
        c700: MoonColor::rgb(0x0F766E),
        c800: MoonColor::rgb(0x115E59),
        c900: MoonColor::rgb(0x134E4A),
        c950: MoonColor::rgb(0x042F2E),
    };

    /// The cyan scale.
    pub const CYAN: Self = Self {
        c50: MoonColor::rgb(0xECFEFF),
        c100: MoonColor::rgb(0xCFFAFE),
        c200: MoonColor::rgb(0xA5F3FC),
        c300: MoonColor::rgb(0x67E8F9),
        c400: MoonColor::rgb(0x22D3EE),
        c500: MoonColor::rgb(0x06B6D4),
        c600: MoonColor::rgb(0x0891B2),
        c700: MoonColor::rgb(0x0E7490),
        c800: MoonColor::rgb(0x155E75),
        c900: MoonColor::rgb(0x164E63),
        c950: MoonColor::rgb(0x083344),
    };

    /// The sky scale.
    pub const SKY: Self = Self {
        c50: MoonColor::rgb(0xF0F9FF),
        c100: MoonColor::rgb(0xE0F2FE),
        c200: MoonColor::rgb(0xBAE6FD),
        c300: MoonColor::rgb(0x7DD3FC),
        c400: MoonColor::rgb(0x38BDF8),
        c500: MoonColor::rgb(0x0EA5E9),
        c600: MoonColor::rgb(0x0284C7),
        c700: MoonColor::rgb(0x0369A1),
        c800: MoonColor::rgb(0x075985),
        c900: MoonColor::rgb(0x0C4A6E),
        c950: MoonColor::rgb(0x082F49),
    };

    /// The blue scale.
    pub const BLUE: Self = Self {
        c50: MoonColor::rgb(0xEFF6FF),
        c100: MoonColor::rgb(0xDBEAFE),
        c200: MoonColor::rgb(0xBFDBFE),
        c300: MoonColor::rgb(0x93C5FD),
        c400: MoonColor::rgb(0x60A5FA),
        c500: MoonColor::rgb(0x3B82F6),
        c600: MoonColor::rgb(0x2563EB),
        c700: MoonColor::rgb(0x1D4ED8),
        c800: MoonColor::rgb(0x1E40AF),
        c900: MoonColor::rgb(0x1E3A8A),
        c950: MoonColor::rgb(0x172554),
    };

    /// The indigo scale.
    pub const INDIGO: Self = Self {
        c50: MoonColor::rgb(0xEEF2FF),
        c100: MoonColor::rgb(0xE0E7FF),
        c200: MoonColor::rgb(0xC7D2FE),
        c300: MoonColor::rgb(0xA5B4FC),
        c400: MoonColor::rgb(0x818CF8),
        c500: MoonColor::rgb(0x6366F1),
        c600: MoonColor::rgb(0x4F46E5),
        c700: MoonColor::rgb(0x4338CA),
        c800: MoonColor::rgb(0x3730A3),
        c900: MoonColor::rgb(0x312E81),
        c950: MoonColor::rgb(0x1E1B4B),
    };

    /// The violet scale.
    pub const VIOLET: Self = Self {
        c50: MoonColor::rgb(0xF5F3FF),
        c100: MoonColor::rgb(0xEDE9FE),
        c200: MoonColor::rgb(0xDDD6FE),
        c300: MoonColor::rgb(0xC4B5FD),
        c400: MoonColor::rgb(0xA78BFA),
        c500: MoonColor::rgb(0x8B5CF6),
        c600: MoonColor::rgb(0x7C3AED),
        c700: MoonColor::rgb(0x6D28D9),
        c800: MoonColor::rgb(0x5B21B6),
        c900: MoonColor::rgb(0x4C1D95),
        c950: MoonColor::rgb(0x2E1065),
    };

    /// The purple scale.
    pub const PURPLE: Self = Self {
        c50: MoonColor::rgb(0xFAF5FF),
        c100: MoonColor::rgb(0xF3E8FF),
        c200: MoonColor::rgb(0xE9D5FF),
        c300: MoonColor::rgb(0xD8B4FE),
        c400: MoonColor::rgb(0xC084FC),
        c500: MoonColor::rgb(0xA855F7),
        c600: MoonColor::rgb(0x9333EA),
        c700: MoonColor::rgb(0x7E22CE),
        c800: MoonColor::rgb(0x6B21A8),
        c900: MoonColor::rgb(0x581C87),
        c950: MoonColor::rgb(0x3B0764),
    };

    /// The fuchsia scale.
    pub const FUCHSIA: Self = Self {
        c50: MoonColor::rgb(0xFDF4FF),
        c100: MoonColor::rgb(0xFAE8FF),
        c200: MoonColor::rgb(0xF5D0FE),
        c300: MoonColor::rgb(0xF0ABFC),
        c400: MoonColor::rgb(0xE879F9),
        c500: MoonColor::rgb(0xD946EF),
        c600: MoonColor::rgb(0xC026D3),
        c700: MoonColor::rgb(0xA21CAF),
        c800: MoonColor::rgb(0x86198F),
        c900: MoonColor::rgb(0x701A75),
        c950: MoonColor::rgb(0x4A044E),
    };

    /// The pink scale.
    pub const PINK: Self = Self {
        c50: MoonColor::rgb(0xFDF2F8),
        c100: MoonColor::rgb(0xFCE7F3),
        c200: MoonColor::rgb(0xFBCFE8),
        c300: MoonColor::rgb(0xF9A8D4),
        c400: MoonColor::rgb(0xF472B6),
        c500: MoonColor::rgb(0xEC4899),
        c600: MoonColor::rgb(0xDB2777),
        c700: MoonColor::rgb(0xBE185D),
        c800: MoonColor::rgb(0x9D174D),
        c900: MoonColor::rgb(0x831843),
        c950: MoonColor::rgb(0x500724),
    };

    /// The rose scale.
    pub const ROSE: Self = Self {
        c50: MoonColor::rgb(0xFFF1F2),
        c100: MoonColor::rgb(0xFFE4E6),
        c200: MoonColor::rgb(0xFECDD3),
        c300: MoonColor::rgb(0xFDA4AF),
        c400: MoonColor::rgb(0xFB7185),
        c500: MoonColor::rgb(0xF43F5E),
        c600: MoonColor::rgb(0xE11D48),
        c700: MoonColor::rgb(0xBE123C),
        c800: MoonColor::rgb(0x9F1239),
        c900: MoonColor::rgb(0x881337),
        c950: MoonColor::rgb(0x4C0519),
    };

    /// The slate scale.
    pub const SLATE: Self = Self {
        c50: MoonColor::rgb(0xF8FAFC),
        c100: MoonColor::rgb(0xF1F5F9),
        c200: MoonColor::rgb(0xE2E8F0),
        c300: MoonColor::rgb(0xCBD5E1),
        c400: MoonColor::rgb(0x94A3B8),
        c500: MoonColor::rgb(0x64748B),
        c600: MoonColor::rgb(0x475569),
        c700: MoonColor::rgb(0x334155),
        c800: MoonColor::rgb(0x1E293B),
        c900: MoonColor::rgb(0x0F172A),
        c950: MoonColor::rgb(0x020617),
    };

    /// The gray scale.
    pub const GRAY: Self = Self {
        c50: MoonColor::rgb(0xF9FAFB),
        c100: MoonColor::rgb(0xF3F4F6),
        c200: MoonColor::rgb(0xE5E7EB),
        c300: MoonColor::rgb(0xD1D5DB),
        c400: MoonColor::rgb(0x9CA3AF),
        c500: MoonColor::rgb(0x6B7280),
        c600: MoonColor::rgb(0x4B5563),
        c700: MoonColor::rgb(0x374151),
        c800: MoonColor::rgb(0x1F2937),
        c900: MoonColor::rgb(0x111827),
        c950: MoonColor::rgb(0x030712),
    };

    /// The zinc scale.
    pub const ZINC: Self = Self {
        c50: MoonColor::rgb(0xFAFAFA),
        c100: MoonColor::rgb(0xF4F4F5),
        c200: MoonColor::rgb(0xE4E4E7),
        c300: MoonColor::rgb(0xD4D4D8),
        c400: MoonColor::rgb(0xA1A1AA),
        c500: MoonColor::rgb(0x71717A),
        c600: MoonColor::rgb(0x52525B),
        c700: MoonColor::rgb(0x3F3F46),
        c800: MoonColor::rgb(0x27272A),
        c900: MoonColor::rgb(0x18181B),
        c950: MoonColor::rgb(0x09090B),
    };

    /// The stone scale.
    pub const STONE: Self = Self {
        c50: MoonColor::rgb(0xFAFAF9),
        c100: MoonColor::rgb(0xF5F5F4),
        c200: MoonColor::rgb(0xE7E5E4),
        c300: MoonColor::rgb(0xD6D3D1),
        c400: MoonColor::rgb(0xA8A29E),
        c500: MoonColor::rgb(0x78716C),
        c600: MoonColor::rgb(0x57534E),
        c700: MoonColor::rgb(0x44403C),
        c800: MoonColor::rgb(0x292524),
        c900: MoonColor::rgb(0x1C1917),
        c950: MoonColor::rgb(0x0C0A09),
    };

    /// The taupe scale.
    pub const TAUPE: Self = Self {
        c50: MoonColor::rgb(0xFBFAF9),
        c100: MoonColor::rgb(0xF3F1F1),
        c200: MoonColor::rgb(0xE8E4E3),
        c300: MoonColor::rgb(0xD8D2D0),
        c400: MoonColor::rgb(0xABA09C),
        c500: MoonColor::rgb(0x7C6D67),
        c600: MoonColor::rgb(0x5B4F4B),
        c700: MoonColor::rgb(0x473C39),
        c800: MoonColor::rgb(0x2B2422),
        c900: MoonColor::rgb(0x1D1816),
        c950: MoonColor::rgb(0x0C0A09),
    };

    /// The mauve scale.
    pub const MAUVE: Self = Self {
        c50: MoonColor::rgb(0xFAFAFA),
        c100: MoonColor::rgb(0xF3F1F3),
        c200: MoonColor::rgb(0xE7E4E7),
        c300: MoonColor::rgb(0xD7D0D7),
        c400: MoonColor::rgb(0xA89EA9),
        c500: MoonColor::rgb(0x79697B),
        c600: MoonColor::rgb(0x594C5B),
        c700: MoonColor::rgb(0x463947),
        c800: MoonColor::rgb(0x2A212C),
        c900: MoonColor::rgb(0x1D161E),
        c950: MoonColor::rgb(0x0C090C),
    };

    /// The mist scale.
    pub const MIST: Self = Self {
        c50: MoonColor::rgb(0xF9FBFB),
        c100: MoonColor::rgb(0xF1F3F3),
        c200: MoonColor::rgb(0xE3E7E8),
        c300: MoonColor::rgb(0xD0D6D8),
        c400: MoonColor::rgb(0x9CA8AB),
        c500: MoonColor::rgb(0x67787C),
        c600: MoonColor::rgb(0x4B585B),
        c700: MoonColor::rgb(0x394447),
        c800: MoonColor::rgb(0x22292B),
        c900: MoonColor::rgb(0x161B1D),
        c950: MoonColor::rgb(0x090B0C),
    };

    /// The olive scale.
    pub const OLIVE: Self = Self {
        c50: MoonColor::rgb(0xFBFBF9),
        c100: MoonColor::rgb(0xF4F4F0),
        c200: MoonColor::rgb(0xE8E8E3),
        c300: MoonColor::rgb(0xD8D8D0),
        c400: MoonColor::rgb(0xABAB9C),
        c500: MoonColor::rgb(0x7C7C67),
        c600: MoonColor::rgb(0x5B5B4B),
        c700: MoonColor::rgb(0x474739),
        c800: MoonColor::rgb(0x2B2B22),
        c900: MoonColor::rgb(0x1D1D16),
        c950: MoonColor::rgb(0x0C0C09),
    };
}

#[cfg(test)]
mod tests;
