use super::*;
use crate::{
    AppContext as _, Context, InputEvent, MouseMoveEvent, TestAppContext, util::FluentBuilder as _,
};
use std::rc::Weak;

struct TestTooltipView;

impl Render for TestTooltipView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().w(px(20.)).h(px(20.)).child("tooltip")
    }
}

type CapturedActiveTooltip = Rc<RefCell<Option<Weak<RefCell<Option<ActiveTooltip>>>>>>;

struct TooltipCaptureElement {
    child: AnyElement,
    captured_active_tooltip: CapturedActiveTooltip,
}

impl IntoElement for TooltipCaptureElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TooltipCaptureElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        (self.child.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.child.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
        window.with_global_id("target".into(), |global_id, window| {
            window.with_element_state::<InteractiveElementState, _>(global_id, |state, _window| {
                let state = state.unwrap();
                *self.captured_active_tooltip.borrow_mut() =
                    state.active_tooltip.as_ref().map(Rc::downgrade);
                ((), state)
            })
        });
    }
}

struct TooltipOwner {
    captured_active_tooltip: CapturedActiveTooltip,
    show_delay_override: Option<Duration>,
    hoverable: bool,
}

impl Render for TooltipOwner {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let target = div().id("target").w(px(50.)).h(px(50.));
        let target = if self.hoverable {
            target.hoverable_tooltip(|_, cx| cx.new(|_| TestTooltipView).into())
        } else {
            target.tooltip(|_, cx| cx.new(|_| TestTooltipView).into())
        };
        TooltipCaptureElement {
            child: div()
                .size_full()
                .child(target.when_some(self.show_delay_override, |this, delay| {
                    this.tooltip_show_delay(delay)
                }))
                .into_any_element(),
            captured_active_tooltip: self.captured_active_tooltip.clone(),
        }
    }
}

#[test]
fn scroll_handle_aligns_wide_children_to_left_edge() {
    let handle = ScrollHandle::new();
    {
        let mut state = handle.0.borrow_mut();
        state.bounds = Bounds::new(point(px(0.), px(0.)), size(px(80.), px(20.)));
        state.child_bounds = vec![Bounds::new(point(px(25.), px(0.)), size(px(200.), px(20.)))];
        state.overflow.x = Overflow::Scroll;
        state.active_item = Some(ScrollActiveItem {
            index: 0,
            strategy: ScrollStrategy::default(),
        });
    }

    handle.scroll_to_active_item();

    assert_eq!(handle.offset().x, px(-25.));
}

#[test]
fn scroll_handle_aligns_tall_children_to_top_edge() {
    let handle = ScrollHandle::new();
    {
        let mut state = handle.0.borrow_mut();
        state.bounds = Bounds::new(point(px(0.), px(0.)), size(px(20.), px(80.)));
        state.child_bounds = vec![Bounds::new(point(px(0.), px(25.)), size(px(20.), px(200.)))];
        state.overflow.y = Overflow::Scroll;
        state.active_item = Some(ScrollActiveItem {
            index: 0,
            strategy: ScrollStrategy::default(),
        });
    }

    handle.scroll_to_active_item();

    assert_eq!(handle.offset().y, px(-25.));
}

/// Build a rendered tooltip owner and begin one pointer-hover episode.
fn setup_tooltip_owner_test(
    show_delay_override: Option<Duration>,
) -> (
    TestAppContext,
    crate::AnyWindowHandle,
    CapturedActiveTooltip,
) {
    setup_tooltip_owner_test_with_hoverability(show_delay_override, false)
}

/// Build a tooltip owner with explicit hoverability and begin one pointer-hover episode.
fn setup_tooltip_owner_test_with_hoverability(
    show_delay_override: Option<Duration>,
    hoverable: bool,
) -> (
    TestAppContext,
    crate::AnyWindowHandle,
    CapturedActiveTooltip,
) {
    let mut test_app = TestAppContext::single();
    let captured_active_tooltip: CapturedActiveTooltip = Rc::new(RefCell::new(None));
    let window = test_app.add_window({
        let captured_active_tooltip = captured_active_tooltip.clone();
        move |_, _| TooltipOwner {
            captured_active_tooltip,
            show_delay_override,
            hoverable,
        }
    });
    let any_window = window.into();

    test_app
        .update_window(any_window, |_, window, cx| {
            window.draw(cx).clear();
        })
        .unwrap();

    test_app
        .update_window(any_window, |_, window, cx| {
            window.dispatch_event(
                MouseMoveEvent {
                    position: point(px(10.), px(10.)),
                    modifiers: Default::default(),
                    pressed_button: None,
                }
                .to_platform_input(),
                cx,
            );
        })
        .unwrap();

    test_app
        .update_window(any_window, |_, window, cx| {
            window.draw(cx).clear();
        })
        .unwrap();

    (test_app, any_window, captured_active_tooltip)
}

/// Moving the pointer within or outside the test window drives the real tooltip handlers.
fn move_test_pointer(
    test_app: &mut TestAppContext,
    any_window: crate::AnyWindowHandle,
    x: f32,
    y: f32,
) {
    test_app
        .update_window(any_window, |_, window, cx| {
            window.dispatch_event(
                MouseMoveEvent {
                    position: point(px(x), px(y)),
                    modifiers: Default::default(),
                    pressed_button: None,
                }
                .to_platform_input(),
                cx,
            );
        })
        .unwrap();
}

#[test]
fn tooltip_waiting_for_show_is_released_when_its_owner_disappears() {
    let (mut test_app, any_window, captured_active_tooltip) = setup_tooltip_owner_test(None);

    let weak_active_tooltip = captured_active_tooltip.borrow().clone().unwrap();
    let active_tooltip = weak_active_tooltip.upgrade().unwrap();
    assert!(matches!(
        active_tooltip.borrow().as_ref(),
        Some(ActiveTooltip::WaitingForShow { .. })
    ));

    test_app
        .update_window(any_window, |_, window, _| {
            window.remove_window();
        })
        .unwrap();
    test_app.run_until_parked();
    drop(active_tooltip);

    assert!(weak_active_tooltip.upgrade().is_none());
}

#[test]
fn tooltip_respects_custom_show_delay() {
    let extra_delay = Duration::from_secs(1);
    let show_delay_override = DEFAULT_TOOLTIP_SHOW_DELAY + extra_delay;
    let (mut test_app, _any_window, captured_active_tooltip) =
        setup_tooltip_owner_test(Some(show_delay_override));

    let weak_active_tooltip = captured_active_tooltip.borrow().clone().unwrap();
    let active_tooltip = weak_active_tooltip.upgrade().unwrap();

    test_app
        .dispatcher
        .advance_clock(DEFAULT_TOOLTIP_SHOW_DELAY);
    test_app.run_until_parked();

    assert!(matches!(
        active_tooltip.borrow().as_ref(),
        Some(ActiveTooltip::WaitingForShow { .. })
    ));

    test_app.dispatcher.advance_clock(extra_delay);
    test_app.run_until_parked();

    assert!(matches!(
        active_tooltip.borrow().as_ref(),
        Some(ActiveTooltip::Visible { .. })
    ));
}

/// Catches restoring the old 500 ms default, which made a brief pointer pause open a tooltip.
#[test]
fn regular_tooltip_requires_a_deliberate_eight_hundred_millisecond_hover() {
    let (mut test_app, _any_window, captured_active_tooltip) = setup_tooltip_owner_test(None);
    let active_tooltip = captured_active_tooltip
        .borrow()
        .clone()
        .unwrap()
        .upgrade()
        .unwrap();

    test_app
        .dispatcher
        .advance_clock(Duration::from_millis(799));
    test_app.run_until_parked();
    assert!(matches!(
        active_tooltip.borrow().as_ref(),
        Some(ActiveTooltip::WaitingForShow { .. })
    ));

    test_app.dispatcher.advance_clock(Duration::from_millis(1));
    test_app.run_until_parked();
    assert!(matches!(
        active_tooltip.borrow().as_ref(),
        Some(ActiveTooltip::Visible { .. })
    ));
}

/// Catches dropping the five-second lifetime or rearming an expired tooltip under mouse jitter.
#[test]
fn regular_tooltip_expires_once_and_waits_for_pointer_reentry() {
    let (mut test_app, any_window, captured_active_tooltip) = setup_tooltip_owner_test(None);
    let active_tooltip = captured_active_tooltip
        .borrow()
        .clone()
        .unwrap()
        .upgrade()
        .unwrap();

    test_app
        .dispatcher
        .advance_clock(Duration::from_millis(800));
    test_app.run_until_parked();
    test_app
        .dispatcher
        .advance_clock(Duration::from_millis(4_999));
    test_app.run_until_parked();
    assert!(matches!(
        active_tooltip.borrow().as_ref(),
        Some(ActiveTooltip::Visible { .. })
    ));

    test_app.dispatcher.advance_clock(Duration::from_millis(1));
    test_app.run_until_parked();
    assert!(matches!(
        active_tooltip.borrow().as_ref(),
        Some(ActiveTooltip::SuppressedUntilMouseLeaves)
    ));

    move_test_pointer(&mut test_app, any_window, 12., 12.);
    test_app
        .dispatcher
        .advance_clock(Duration::from_millis(800));
    test_app.run_until_parked();
    assert!(matches!(
        active_tooltip.borrow().as_ref(),
        Some(ActiveTooltip::SuppressedUntilMouseLeaves)
    ));

    move_test_pointer(&mut test_app, any_window, 75., 75.);
    assert!(active_tooltip.borrow().is_none());
    move_test_pointer(&mut test_app, any_window, 10., 10.);
    assert!(matches!(
        active_tooltip.borrow().as_ref(),
        Some(ActiveTooltip::WaitingForShow { .. })
    ));
}

/// Catches applying the informational auto-hide deadline to interactive hoverable content.
#[test]
fn hoverable_tooltip_stays_visible_while_its_trigger_remains_hovered() {
    let (mut test_app, _any_window, captured_active_tooltip) =
        setup_tooltip_owner_test_with_hoverability(None, true);
    let active_tooltip = captured_active_tooltip
        .borrow()
        .clone()
        .unwrap()
        .upgrade()
        .unwrap();

    test_app
        .dispatcher
        .advance_clock(Duration::from_millis(800));
    test_app.run_until_parked();
    test_app.dispatcher.advance_clock(Duration::from_secs(6));
    test_app.run_until_parked();

    assert!(matches!(
        active_tooltip.borrow().as_ref(),
        Some(ActiveTooltip::Visible {
            is_hoverable: true,
            ..
        })
    ));
}

#[test]
fn tooltip_is_released_when_its_owner_disappears() {
    let (mut test_app, any_window, captured_active_tooltip) = setup_tooltip_owner_test(None);

    let weak_active_tooltip = captured_active_tooltip.borrow().clone().unwrap();
    let active_tooltip = weak_active_tooltip.upgrade().unwrap();

    test_app
        .dispatcher
        .advance_clock(DEFAULT_TOOLTIP_SHOW_DELAY);
    test_app.run_until_parked();

    assert!(matches!(
        active_tooltip.borrow().as_ref(),
        Some(ActiveTooltip::Visible { .. })
    ));

    test_app
        .update_window(any_window, |_, window, _| {
            window.remove_window();
        })
        .unwrap();
    test_app.run_until_parked();
    drop(active_tooltip);

    assert!(weak_active_tooltip.upgrade().is_none());
}

#[test]
fn tooltip_hides_after_mouse_leaves_origin() {
    let (mut test_app, any_window, captured_active_tooltip) = setup_tooltip_owner_test(None);

    let weak_active_tooltip = captured_active_tooltip.borrow().clone().unwrap();
    let active_tooltip = weak_active_tooltip.upgrade().unwrap();

    test_app
        .dispatcher
        .advance_clock(DEFAULT_TOOLTIP_SHOW_DELAY);
    test_app.run_until_parked();

    assert!(matches!(
        active_tooltip.borrow().as_ref(),
        Some(ActiveTooltip::Visible { .. })
    ));

    test_app
        .update_window(any_window, |_, window, cx| {
            window.dispatch_event(
                MouseMoveEvent {
                    position: point(px(75.), px(75.)),
                    modifiers: Default::default(),
                    pressed_button: None,
                }
                .to_platform_input(),
                cx,
            );
        })
        .unwrap();

    assert!(active_tooltip.borrow().is_none());
}

/// Catches dropping the `inspector_transparent` term from `Interactivity::should_insert_hitbox`,
/// which would put a hitbox back on every decorative overlay while the inspector is picking.
///
/// The consequence is not visible and not local: picking takes the topmost hitbox that carries an
/// inspector id, so a transparent layer stretched over a list answers for every row under it. A
/// terminal that documents its own interface through the inspector could name nothing inside any
/// scrollable surface — six tables and lists all came back as their scrollbar, sized to the whole
/// panel.
#[test]
fn inspector_picking_looks_through_an_element_that_declared_itself_transparent() {
    let mut test_app = TestAppContext::single();
    let window = test_app.add_window(move |_, _| TestTooltipView);
    let any_window = window.into();

    test_app
        .update_window(any_window, |_, window, cx| {
            let style = Style::default();
            let plain = Interactivity::default();
            let transparent = Interactivity {
                inspector_transparent: true,
                ..Default::default()
            };

            // Outside picking neither of them is interactive, so neither takes a hitbox — the
            // half of the behaviour the flag must leave exactly as it was.
            assert!(!plain.should_insert_hitbox(&style, window, cx));
            assert!(!transparent.should_insert_hitbox(&style, window, cx));

            // `Inspector::new` starts in picking mode, which is what gives every element a hitbox
            // so that any of them can be selected.
            window.toggle_inspector(cx);
            assert!(
                window.is_inspector_picking(cx),
                "the inspector opens already picking"
            );
            assert!(
                plain.should_insert_hitbox(&style, window, cx),
                "picking offers every element"
            );
            assert!(
                !transparent.should_insert_hitbox(&style, window, cx),
                "except one that asked to be looked through"
            );
        })
        .unwrap();
}
