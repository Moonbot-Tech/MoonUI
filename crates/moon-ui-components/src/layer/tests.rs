//! Band-ordering tests for the deferred-draw layer constants live here and are authored by the
//! breakage prover.

use super::{LAYER_MOON_POPOVER, LAYER_MOON_POPOVER_MENU, LAYER_OVERLAY, LAYER_TOOLTIP};

/// `layer.rs` is the only place these bands are numbered. Collapsing two bands together or
/// renumbering `LAYER_TOOLTIP` under `LAYER_MOON_POPOVER_MENU` lets a Moon popover or its
/// in-popover select menu repaint over a tooltip again, reproducing MoonUI #628.
#[test]
fn tooltip_outranks_every_other_deferred_band() {
    assert!(LAYER_TOOLTIP > LAYER_MOON_POPOVER_MENU);
    assert!(LAYER_MOON_POPOVER_MENU > LAYER_MOON_POPOVER);
    assert!(LAYER_MOON_POPOVER > LAYER_OVERLAY);

    // Independently derived from the frozen literal at
    // crates/moon-ui-components/src/time/date_picker.rs:512 (`.with_priority(2)`), a
    // component-mirror donor file this ordering must never regress below.
    let date_picker_calendar_priority: usize = 2;
    assert!(LAYER_TOOLTIP > date_picker_calendar_priority);
}
