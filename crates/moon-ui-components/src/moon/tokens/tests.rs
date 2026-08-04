//! Regression coverage for the published terminal palette contracts.

use super::{MoonPalette, MoonTone, contrast_ratio};

/// WCAG floor for normal-size text. Selected rows print small labels, so the strict bar applies.
const SELECTED_TEXT_CONTRAST_FLOOR: f32 = 4.5;

/// Catches choosing `tokens.rs:MoonPalette::selected_fg` by contrast against the **accent**.
///
/// The fill it pairs with is `selected_background`, an 11% accent tint fading to nothing, so this
/// ink lands on the panel underneath. Measuring it against the accent picks a near-black ink for
/// the dark palette, which is black text on a dark panel — every selected list, menu and dropdown
/// row unreadable. Shipped once; this is the guard.
#[test]
fn selected_row_ink_stays_readable_on_the_panel_it_is_tinted_over() {
    for (name, p) in [
        ("dark", MoonPalette::TERMINAL),
        ("light", MoonPalette::LIGHT),
    ] {
        let ratio = contrast_ratio(p.selected_fg(), p.panel);
        assert!(
            ratio >= SELECTED_TEXT_CONTRAST_FLOOR,
            "{name} palette: selected-row ink #{:06X} on panel #{:06X} is {ratio:.2}:1, below {SELECTED_TEXT_CONTRAST_FLOOR}",
            p.selected_fg(),
            p.panel
        );
    }
}

/// Catches `tokens.rs:MoonPalette::ink_on` answering by theme instead of by measurement. A solid
/// accent fill is light in the dark palette and dark in the light one, so the readable ink is the
/// opposite of what the theme suggests.
#[test]
fn ink_on_a_solid_fill_is_readable_in_both_palettes() {
    for (name, p) in [
        ("dark", MoonPalette::TERMINAL),
        ("light", MoonPalette::LIGHT),
    ] {
        for (fill_name, fill) in [("accent", p.accent), ("panel", p.panel)] {
            let ratio = contrast_ratio(p.ink_on(fill), fill);
            assert!(
                ratio >= SELECTED_TEXT_CONTRAST_FLOOR,
                "{name} palette: ink #{:06X} on {fill_name} #{fill:06X} is {ratio:.2}:1, below {SELECTED_TEXT_CONTRAST_FLOOR}",
                p.ink_on(fill),
            );
        }
    }
}

/// Catches changing `tokens.rs:MoonPalette::TERMINAL` away from the released dark-palette
/// specification, which would unexpectedly recolor existing MoonTerminal screens.
#[test]
fn dark_terminal_palette_keeps_legacy_core_values() {
    let p = MoonPalette::TERMINAL;
    assert_eq!(p.shell, 0x131416);
    assert_eq!(p.shell_high, 0x1A1C1F);
    assert_eq!(p.panel, 0x20232A);
    assert_eq!(p.border, 0x2A2D31);
    assert_eq!(p.text, 0xE8E4DC);
    assert_eq!(p.green, 0x1E8C5B);
    assert_eq!(p.red, 0xE5484D);
    assert_eq!(p.orange, 0xFF8E5A);
    assert_eq!(p.blue, 0x7FC9FF);
    assert_eq!(p.accent, 0xFFB347);
}

/// Catches changing `tokens.rs:MoonPalette::LIGHT` away from the neutral-terminal design
/// specification, which would break semantic contrast and status coloring in the light theme.
#[test]
fn light_palette_matches_neutral_terminal_spec() {
    let p = MoonPalette::LIGHT;
    assert_eq!(p.shell, 0xF3F5F7);
    assert_eq!(p.window, 0xF7F8FA);
    assert_eq!(p.chrome, 0xF5F7FA);
    assert_eq!(p.tabbar, 0xF2F5F8);
    assert_eq!(p.surface, 0xFFFFFF);
    assert_eq!(p.card, 0xFFFFFF);
    assert_eq!(p.row_alt, 0xFCFDFE);
    assert_eq!(p.head_row, 0xF3F6F8);
    assert_eq!(p.gutter, 0xEEF2F6);
    assert_eq!(p.border, 0xD5DBE1);
    assert_eq!(p.border_soft, 0xE1E5EA);
    assert_eq!(p.border_card, 0xDCE2E8);
    assert_eq!(p.row_line, 0xECEFF2);
    assert_eq!(p.text, 0x17202A);
    assert_eq!(p.text_soft, 0x4B5865);
    assert_eq!(p.text_dim, 0x2D3945);
    assert_eq!(p.text_muted, 0x768391);
    assert_eq!(p.text_faint, 0x98A3AE);
    assert_eq!(p.accent, 0x009DFF);
    assert_eq!(p.accent_fg, 0x0A3F68);
    assert_ne!(p.accent, p.accent_fg);
    assert_eq!(MoonTone::Accent.color(p), p.accent_fg);
    assert_eq!(MoonTone::Info.color(p), p.blue);
    assert_eq!(p.green_text, 0x0E6E45);
    assert_eq!(p.green_btn, 0x178A57);
    assert_eq!(p.red, 0xD2483F);
    assert_eq!(p.red_text, 0xB7352F);
    assert_eq!(p.red_soft_bd, 0xE1B5B0);
}

/// Catches disconnecting new role fallbacks in `tokens.rs:MoonPalette::with_legacy_defaults` from
/// their established palette roles, which would leave old configurations with invisible colors.
#[test]
fn legacy_palette_defaults_fill_new_roles() {
    let legacy = MoonPalette {
        window: 0,
        tabbar: 0,
        card: 0,
        row_alt: 0,
        head_row: 0,
        border_soft: 0,
        border_card: 0,
        row_line: 0,
        text_dim: 0,
        text_faint: 0,
        green_btn: 0,
        green_text: 0,
        red_text: 0,
        red_soft_bd: 0,
        ..MoonPalette::TERMINAL
    }
    .with_legacy_defaults();

    assert_eq!(legacy.window, MoonPalette::TERMINAL.shell);
    assert_eq!(legacy.tabbar, MoonPalette::TERMINAL.chrome);
    assert_eq!(legacy.card, MoonPalette::TERMINAL.table_body);
    assert_eq!(legacy.green_text, MoonPalette::TERMINAL.green);
    assert_eq!(legacy.red_text, MoonPalette::TERMINAL.red);
}
