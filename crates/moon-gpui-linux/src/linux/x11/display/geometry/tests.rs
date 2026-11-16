use super::{Rect, containing, logical_rect, monitor_display_id, screen_index_of};

/// Two 1920-wide monitors side by side, as a laptop with an external screen reports them.
fn side_by_side() -> [Rect; 2] {
    [
        Rect {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1200.0,
        },
        Rect {
            x: 1920.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
        },
    ]
}

/// A monitor id must lead back to its X screen. Dropping the tag bit makes the name atom read
/// as the screen index, and the window then asks for an X screen that does not exist.
#[test]
fn monitor_id_leads_back_to_its_screen() {
    assert_eq!(screen_index_of(monitor_display_id(0, 517)), 0);
    assert_eq!(screen_index_of(monitor_display_id(1, 517)), 1);
    assert_eq!(screen_index_of(1), 1);
}

/// Two monitors of one screen must not share an id, and neither may collide with the id of the
/// whole screen, or `find_display` would hand back the wrong rectangle.
#[test]
fn monitor_ids_are_distinct_from_each_other_and_from_the_screen() {
    let first = monitor_display_id(0, 517);
    let second = monitor_display_id(0, 518);
    assert_ne!(first, second);
    assert_ne!(first, 0);
    assert_ne!(monitor_display_id(1, 517), first);
}

/// The origin is scaled together with the size: at 1.5 the second monitor starts at 1280, not
/// at 1920, because that is where a window placed on it has its logical origin.
#[test]
fn logical_rect_scales_origin_and_size() {
    assert_eq!(
        logical_rect(1920, 0, 1920, 1080, 1.5),
        Some(Rect {
            x: 1280.0,
            y: 0.0,
            width: 1280.0,
            height: 720.0,
        })
    );
}

/// A scale that cannot divide is read as 1 rather than producing an infinite monitor.
#[test]
fn logical_rect_survives_an_unusable_scale() {
    let unscaled = Some(Rect {
        x: 10.0,
        y: 20.0,
        width: 800.0,
        height: 600.0,
    });
    for scale in [0.0, -2.0, f32::NAN, f32::INFINITY] {
        assert_eq!(logical_rect(10, 20, 800, 600, scale), unscaled);
    }
}

#[test]
fn logical_rect_rejects_a_monitor_without_area() {
    assert_eq!(logical_rect(0, 0, 0, 1080, 1.0), None);
    assert_eq!(logical_rect(0, 0, 1920, 0, 1.0), None);
}

/// The window from the report: 2880 wide from x=480, so 1440 px on each monitor. The taller
/// first monitor holds no more of a 900-tall window than the second, and the tie goes to the
/// earlier one.
#[test]
fn a_window_split_evenly_belongs_to_the_earlier_monitor() {
    let window = Rect {
        x: 480.0,
        y: 150.0,
        width: 2880.0,
        height: 900.0,
    };
    assert_eq!(containing(window, &side_by_side()), Some(0));
}

#[test]
fn a_window_belongs_to_the_monitor_holding_most_of_it() {
    let window = Rect {
        x: 1800.0,
        y: 100.0,
        width: 800.0,
        height: 600.0,
    };
    assert_eq!(containing(window, &side_by_side()), Some(1));
}

/// A window past every monitor still gets one: the nearest, here the right-hand monitor.
#[test]
fn a_window_off_every_monitor_belongs_to_the_nearest() {
    let window = Rect {
        x: 5000.0,
        y: 100.0,
        width: 800.0,
        height: 600.0,
    };
    assert_eq!(containing(window, &side_by_side()), Some(1));
}

#[test]
fn no_monitors_means_no_display() {
    let window = Rect {
        x: 0.0,
        y: 0.0,
        width: 800.0,
        height: 600.0,
    };
    assert_eq!(containing(window, &[]), None);
}
