use crate::popover::Popover as CorePopover;
use gpui::*;

use super::{
    background::MoonBackgroundPolicy,
    theme::MoonTheme,
    tokens::{MoonPalette, MoonRect, rgba_from},
};

const MOON_POPOVER_PRIORITY: usize = 30_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoonPopoverPlacement {
    BottomStart,
    BottomEnd,
    TopStart,
    TopEnd,
    RightStart,
    LeftStart,
}

#[derive(Default)]
struct MoonPopoverState {
    open: bool,
}

#[derive(IntoElement)]
pub struct MoonPopover {
    id: SharedString,
    bounds: Option<MoonRect>,
    trigger: Option<AnyElement>,
    content: Option<AnyElement>,
    placement: MoonPopoverPlacement,
    default_open: bool,
    controlled_open: Option<bool>,
    disabled: bool,
    close_on_content_click: bool,
    overlay_closable: bool,
    width: f32,
    offset_x: f32,
    offset_y: f32,
    background_policy: MoonBackgroundPolicy,
    on_open_change: Option<std::rc::Rc<dyn Fn(bool, &mut Window, &mut App)>>,
}

impl MoonPopover {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            trigger: None,
            content: None,
            placement: MoonPopoverPlacement::BottomStart,
            default_open: false,
            controlled_open: None,
            disabled: false,
            close_on_content_click: false,
            overlay_closable: true,
            width: 220.0,
            offset_x: 0.0,
            offset_y: 6.0,
            background_policy: MoonBackgroundPolicy::Opaque,
            on_open_change: None,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn trigger(mut self, trigger: impl IntoElement) -> Self {
        self.trigger = Some(trigger.into_any_element());
        self
    }

    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content = Some(content.into_any_element());
        self
    }

    pub fn placement(mut self, placement: MoonPopoverPlacement) -> Self {
        self.placement = placement;
        self
    }

    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    pub fn open(mut self, open: bool) -> Self {
        self.controlled_open = Some(open);
        self
    }

    pub fn on_open_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open_change = Some(std::rc::Rc::new(handler));
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn close_on_content_click(mut self, close: bool) -> Self {
        self.close_on_content_click = close;
        self
    }

    /// Whether a mouse-down outside the popover dismisses it (default `true`).
    /// Disable for popovers hosting nested overlay layers (dropdown menus, nested
    /// popovers): those are drawn in separate deferred layers, so clicks on their
    /// parts that extend beyond the popover bounds register as "outside" and would
    /// close the popover mid-interaction. Pair with an explicit close control.
    pub fn overlay_closable(mut self, closable: bool) -> Self {
        self.overlay_closable = closable;
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn offset(mut self, x: f32, y: f32) -> Self {
        self.offset_x = x;
        self.offset_y = y;
        self
    }

    pub fn background_policy(mut self, policy: MoonBackgroundPolicy) -> Self {
        self.background_policy = policy;
        self
    }
}

impl RenderOnce for MoonPopover {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = MoonPalette::active(cx);
        let trigger = self.trigger.unwrap_or_else(|| div().into_any_element());
        let parent_view = window.current_view();
        let state = window.use_keyed_state(
            ElementId::from(SharedString::from(format!("{}:moon-state", self.id))),
            cx,
            |_, _| MoonPopoverState {
                open: self.default_open,
            },
        );
        let open = self.controlled_open.unwrap_or_else(|| state.read(cx).open);
        let on_open_change = self.on_open_change.clone();

        let mut root = div().id(ElementId::from(self.id.clone())).relative();
        if let Some(bounds) = self.bounds {
            root = root
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }

        if self.disabled {
            return root.child(trigger).into_any_element();
        }
        let tokens = MoonTheme::active_tokens(cx);

        let shadow = super::foundation::box_shadow(
            px(0.0),
            px(10.0),
            px(22.0),
            px(0.0),
            rgba_from(p.shadow, 0.48),
        );
        let mut popup = div()
            .w(px(self.width))
            .p(px(tokens.ui(6.0)))
            .rounded(px(tokens.ui(5.0)))
            .border(px(1.0))
            .border_color(rgba_from(p.border, 1.0))
            .shadow(vec![shadow])
            .occlude()
            .mt(px(self.offset_y))
            .ml(px(self.offset_x))
            .child(self.content.unwrap_or_else(|| div().into_any_element()));

        if self.close_on_content_click {
            popup = popup.capture_any_mouse_down({
                let state = state.clone();
                let on_open_change = on_open_change.clone();
                let controlled_open = self.controlled_open;
                move |_, window, cx| {
                    window.defer(cx, {
                        let state = state.clone();
                        let on_open_change = on_open_change.clone();
                        move |window, cx| {
                            if let Some(on_open_change) = &on_open_change {
                                on_open_change(false, window, cx);
                            }
                            if controlled_open.is_none() {
                                state.update(cx, |state, _| {
                                    state.open = false;
                                });
                                cx.notify(parent_view);
                            }
                        }
                    });
                }
            });
        }

        popup = self.background_policy.apply(popup, p.shell_high, 0.98);

        let mut popover = CorePopover::new(ElementId::from(self.id.clone()))
            .anchor(anchor_for(self.placement))
            .appearance(false)
            .deferred_priority(MOON_POPOVER_PRIORITY)
            .overlay_closable(self.overlay_closable)
            .open(open)
            .trigger_any(trigger)
            .child(popup);

        {
            let state = state.clone();
            let on_open_change = on_open_change.clone();
            let controlled_open = self.controlled_open;
            popover = popover.on_open_change(move |open, window, cx| {
                if let Some(on_open_change) = &on_open_change {
                    on_open_change(*open, window, cx);
                }
                if controlled_open.is_none() {
                    state.update(cx, |state, _| {
                        state.open = *open;
                    });
                    cx.notify(parent_view);
                }
            });
        }

        root.child(popover).into_any_element()
    }
}

fn anchor_for(placement: MoonPopoverPlacement) -> Anchor {
    match placement {
        MoonPopoverPlacement::BottomStart => Anchor::TopLeft,
        MoonPopoverPlacement::BottomEnd => Anchor::TopRight,
        MoonPopoverPlacement::TopStart => Anchor::BottomLeft,
        MoonPopoverPlacement::TopEnd => Anchor::BottomRight,
        MoonPopoverPlacement::RightStart => Anchor::TopRight,
        MoonPopoverPlacement::LeftStart => Anchor::TopLeft,
    }
}
