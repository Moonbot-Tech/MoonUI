use crate::{ActiveTheme as _, popover::Popover as CorePopover};
use gpui::{prelude::FluentBuilder as _, *};

use super::{
    background::MoonBackgroundPolicy,
    theme::MoonTheme,
    tokens::{MoonPalette, MoonRect, rgba_from},
};

const MOON_POPOVER_PRIORITY: usize = 30_000;
const POPOVER_PADDING: f32 = 6.0;
const POPOVER_BORDER: f32 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq)]
/// Width policy for the popover's outer or content box.
enum MoonPopoverWidth {
    Outer(f32),
    Content(f32),
    UiContent(f32),
    FontContent(f32),
    Intrinsic,
}

impl MoonPopoverWidth {
    /// Resolve an optional outer width for the popup border box.
    ///
    /// `None` leaves the box intrinsic. Content policies add the exact padding and border drawn by
    /// the popup, keeping downstream callers independent of those private metrics.
    ///
    /// Args:
    ///     tokens: Active theme tokens used by scaled content policies.
    ///
    /// Returns:
    ///     Resolved outer border-box width, or `None` for intrinsic layout.
    fn resolve(self, tokens: &super::theme::MoonThemeTokens) -> Option<f32> {
        let chrome = tokens.ui(POPOVER_PADDING) * 2.0 + POPOVER_BORDER * 2.0;
        match self {
            Self::Outer(width) => Some(width),
            Self::Content(width) => Some(width + chrome),
            Self::UiContent(width) => Some(tokens.ui(width) + chrome),
            Self::FontContent(width) => Some(tokens.font_width(width) + chrome),
            Self::Intrinsic => None,
        }
    }
}

/// Which surface the popup box paints.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MoonPopoverChrome {
    /// Moon chrome: compact scaled padding on the shell-high surface. The default.
    #[default]
    Moon,
    /// The chrome the Longbridge pickers draw: the theme `popover` surface, `p_3` padding, the
    /// doubled radius and a soft shadow.
    ///
    /// Use it for a popup that opens next to a Mirror picker's popup — a date picker and a
    /// date+time picker in the same form must not show two different popup backgrounds.
    Picker,
}

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
/// Anchored Moon popover with explicit outer, content, scaled, or intrinsic width ownership.
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
    width: MoonPopoverWidth,
    offset_x: f32,
    offset_y: f32,
    background_policy: MoonBackgroundPolicy,
    chrome: MoonPopoverChrome,
    on_open_change: Option<std::rc::Rc<dyn Fn(bool, &mut Window, &mut App)>>,
}

impl MoonPopover {
    /// Create an anchored popover with the legacy rendered outer width.
    ///
    /// Args:
    ///     id: Stable identity used by the popover state and debug selector.
    ///
    /// Returns:
    ///     A default popover builder.
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
            width: MoonPopoverWidth::Outer(220.0),
            offset_x: 0.0,
            offset_y: 6.0,
            background_policy: MoonBackgroundPolicy::Opaque,
            chrome: MoonPopoverChrome::default(),
            on_open_change: None,
        }
    }

    /// Choose the surface the popup box paints; see [`MoonPopoverChrome`].
    pub fn chrome(mut self, chrome: MoonPopoverChrome) -> Self {
        self.chrome = chrome;
        self
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

    /// Set a legacy rendered outer width for the popup border box.
    ///
    /// Prefer [`Self::content_width`], [`Self::content_width_ui`],
    /// [`Self::content_width_font`], or [`Self::fit_content`] for new code so callers do not
    /// reproduce the popup's padding and border.
    ///
    /// Args:
    ///     width: Outer border-box width in rendered pixels.
    ///
    /// Returns:
    ///     The updated popover.
    pub fn width(mut self, width: f32) -> Self {
        self.width = MoonPopoverWidth::Outer(width);
        self
    }

    /// Set an already-rendered width for the popup's content box.
    ///
    /// MoonPopover adds its own scaled padding and fixed border to produce the outer width.
    ///
    /// Args:
    ///     width: Rendered content-box width.
    ///
    /// Returns:
    ///     The updated popover.
    pub fn content_width(mut self, width: f32) -> Self {
        self.width = MoonPopoverWidth::Content(width);
        self
    }

    /// Set a UI-scaled design-reference width for the popup's content box.
    ///
    /// Args:
    ///     width: Content width at the configured UI reference scale.
    ///
    /// Returns:
    ///     The updated popover.
    pub fn content_width_ui(mut self, width: f32) -> Self {
        self.width = MoonPopoverWidth::UiContent(width);
        self
    }

    /// Set a font-scaled design-reference width for the popup's content box.
    ///
    /// Args:
    ///     width: Content width at the configured font reference size.
    ///
    /// Returns:
    ///     The updated popover.
    pub fn content_width_font(mut self, width: f32) -> Self {
        self.width = MoonPopoverWidth::FontContent(width);
        self
    }

    /// Let the popup shrink-wrap its content without imposing an explicit width.
    ///
    /// Returns:
    ///     The updated popover.
    pub fn fit_content(mut self) -> Self {
        self.width = MoonPopoverWidth::Intrinsic;
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
    /// Render the trigger and its optionally open, width-resolved anchored popup.
    ///
    /// Args:
    ///     window: Window owning the keyed open state and anchored popover.
    ///     cx: Application context used to resolve active theme tokens.
    ///
    /// Returns:
    ///     The rendered trigger and popup host.
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
        let popup_debug_id = format!("{}:popup", self.id);
        let chrome = self.chrome;
        let mut popup = div()
            .debug_selector(move || popup_debug_id)
            .border(px(POPOVER_BORDER))
            .border_color(rgba_from(p.border, 1.0))
            .occlude()
            .mt(px(self.offset_y))
            .ml(px(self.offset_x))
            .map(|this| match chrome {
                MoonPopoverChrome::Moon => this
                    .p(px(tokens.ui(POPOVER_PADDING)))
                    .rounded(px(tokens.ui(5.0)))
                    .shadow(vec![shadow]),
                // The exact box the Longbridge pickers draw, so a Moon popup can sit next to a
                // Mirror picker's popup without a second surface colour showing up in the form.
                MoonPopoverChrome::Picker => this
                    .p_3()
                    .rounded((cx.theme().radius * 2.).min(px(8.)))
                    .shadow_lg()
                    .bg(cx.theme().popover)
                    .text_color(cx.theme().popover_foreground),
            })
            .child(self.content.unwrap_or_else(|| div().into_any_element()));
        if let Some(width) = self.width.resolve(&tokens) {
            popup = popup.w(px(width));
        }

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

        if matches!(chrome, MoonPopoverChrome::Moon) {
            popup = self.background_policy.apply(popup, p.shell_high, 0.98);
        }

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

#[cfg(test)]
mod tests;
