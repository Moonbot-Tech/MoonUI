use super::{
    macos_control_click_as_secondary, macos_control_click_rewrite,
    set_macos_control_click_as_secondary,
};
use crate::{Modifiers, MouseButton};

/// Catches restoring Zed's unconditional rewrite, or shipping the opt-in switched on.
///
/// Either edit makes a Control+left press arrive on macOS as a plain right click with the Control
/// flag erased, so an application gesture bound to Control+left never fires and the right-button
/// action runs in its place — the exact defect this switch exists for.
///
/// One test rather than several: the switch is process-global, so separate tests would race each
/// other inside the one test binary.
#[test]
fn control_click_is_left_until_an_application_opts_in() {
    let ctrl = Modifiers::control();

    assert!(
        !macos_control_click_as_secondary(),
        "the default must deliver Control+left as Left carrying Control"
    );
    assert_eq!(macos_control_click_rewrite(MouseButton::Left, ctrl), None);

    set_macos_control_click_as_secondary(true);
    assert_eq!(
        macos_control_click_rewrite(MouseButton::Left, ctrl),
        Some((MouseButton::Right, Modifiers::none())),
        "the editor convention reports a right click and hides Control from the application"
    );
    // Presses that are not Control+left stay untouched even while the convention is on, or a
    // physical right click would become indistinguishable from a Control-click.
    assert_eq!(macos_control_click_rewrite(MouseButton::Right, ctrl), None);
    assert_eq!(
        macos_control_click_rewrite(MouseButton::Left, Modifiers::none()),
        None
    );

    set_macos_control_click_as_secondary(false);
    assert_eq!(macos_control_click_rewrite(MouseButton::Left, ctrl), None);
}
