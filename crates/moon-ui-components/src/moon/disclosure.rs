//! Shared passive and interactive disclosure-caret variants.

use gpui::prelude::FluentBuilder;
use gpui::*;

use super::{
    icons::{MOON_ICON_CARET_DOWN, moon_icon},
    theme::MoonTheme,
};

/// Alpha applied on top of the caller's alpha while the caret is disabled.
const DISABLED_ALPHA: f32 = 0.45;

/// Which pair of poses the caret renders for collapsed and expanded states.
///
/// Named after the PAIR rather than one arrow: a caller that only knew the collapsed pose would
/// have to guess the expanded one, and the two conventions below disagree about it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoonDisclosureDirection {
    /// Collapsed points right, expanded points down — the section and tree disclosure triangle.
    RightDown,
    /// Collapsed points down, expanded points up — the card-fold affordance.
    ///
    /// Not the dropdown-trigger caret: that one is a measured text suffix inside the trigger's
    /// label (see `dropdown.rs`), so it is deliberately out of this component's reach.
    DownUp,
}

/// Turns of rotation applied to the down-pointing caret asset to reach a pose.
///
/// The asset points down when unrotated. Rotation is clockwise, as established by
/// `moon/tooltip.rs`: its right-side placement rotates the down-pointing arrow by `0.25` turns to
/// point left, while its left-side placement uses `0.75` turns to point right. Keep this mapping
/// aligned with that rendering convention.
///
/// Args:
///     direction: The pose pair this caret renders.
///     expanded: Whether the disclosed content is open right now.
///
/// Returns:
///     Turns to pass to `Transformation::rotate(percentage(..))`; `0.0` means draw the asset as is.
pub(crate) fn moon_disclosure_rotation_turns(
    direction: MoonDisclosureDirection,
    expanded: bool,
) -> f32 {
    match (direction, expanded) {
        // Down rotated three quarters clockwise points right.
        (MoonDisclosureDirection::RightDown, false) => 0.75,
        (MoonDisclosureDirection::RightDown, true) => 0.0,
        (MoonDisclosureDirection::DownUp, false) => 0.0,
        (MoonDisclosureDirection::DownUp, true) => 0.5,
    }
}

/// The expanded state a click produces, or `None` when the caret must not react to one.
///
/// A non-interactive caret is drawn inside a row that is itself the click target, so it must let
/// the click through untouched rather than toggle anything of its own.
///
/// Args:
///     expanded: The current state.
///     interactive: Whether this caret was built with interactive identity.
///     disabled: Whether the caret is disabled.
///
/// Returns:
///     `Some(next)` when the click toggles, `None` when it must be ignored.
pub(crate) fn moon_disclosure_click_next(
    expanded: bool,
    interactive: bool,
    disabled: bool,
) -> Option<bool> {
    if disabled || !interactive {
        None
    } else {
        Some(!expanded)
    }
}

/// The shared expand/collapse caret.
///
/// Built through one of two constructors. [`MoonDisclosure::glyph`] is a bare caret drawn beside a
/// label whose owner handles the click; [`MoonDisclosure::button`] is a caret that is itself the
/// control. The split exists because an interactive caret needs an [`ElementId`] and a passive one
/// must not have the cursor, hover style, or listener that would give it a hitbox — in this fork
/// `should_insert_hitbox` inserts one for any of those three, so passivity has to be built rather
/// than inferred from the absence of an id.
#[derive(IntoElement)]
pub struct MoonDisclosure {
    id: Option<ElementId>,
    expanded: bool,
    direction: MoonDisclosureDirection,
    size: f32,
    box_size: Option<f32>,
    color: Option<u32>,
    hover_color: Option<u32>,
    alpha: f32,
    disabled: bool,
    tooltip: Option<SharedString>,
    on_toggle: Option<std::rc::Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
}

impl MoonDisclosure {
    /// Create a disclosure caret with optional interactive identity.
    ///
    /// Args:
    ///     id: Element identity for the interactive arm, or `None` for the passive arm.
    ///     expanded: Whether the disclosed content is currently open.
    ///
    /// Returns:
    ///     A disclosure caret with the default right-to-down pose pair.
    fn new(id: Option<ElementId>, expanded: bool) -> Self {
        Self {
            id,
            expanded,
            direction: MoonDisclosureDirection::RightDown,
            size: 11.0,
            box_size: None,
            color: None,
            hover_color: None,
            alpha: 1.0,
            disabled: false,
            tooltip: None,
            on_toggle: None,
        }
    }

    /// A bare caret whose surrounding row owns the click.
    ///
    /// Installs no cursor, hover style, or listener, so it does not request a hitbox during normal
    /// rendering and cannot swallow the click meant for that row. `tooltip` and `on_toggle` are
    /// inert on this arm.
    ///
    /// Args:
    ///     expanded: Whether the disclosed content is currently open.
    ///
    /// Returns:
    ///     A passive disclosure caret.
    pub fn glyph(expanded: bool) -> Self {
        Self::new(None, expanded)
    }

    /// A caret that acts as the control, with an enabled-state pointer cursor and optional tooltip.
    ///
    /// Args:
    ///     id: Stable identity for the interactive element.
    ///     expanded: Whether the disclosed content is currently open.
    ///
    /// Returns:
    ///     An interactive disclosure caret.
    pub fn button(id: impl Into<ElementId>, expanded: bool) -> Self {
        Self::new(Some(id.into()), expanded)
    }

    /// Select the collapsed and expanded pose pair.
    ///
    /// Args:
    ///     direction: Pose pair to render.
    ///
    /// Returns:
    ///     This caret with the requested pose pair.
    pub fn direction(mut self, direction: MoonDisclosureDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Base (unscaled) edge length of the glyph. Pass a design-reference number — this is scaled
    /// by the active UI token at render, so a pre-scaled value scales twice.
    ///
    /// Args:
    ///     size: Unscaled design-reference glyph edge length.
    ///
    /// Returns:
    ///     This caret with the requested glyph size.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Base (unscaled) square box drawn around the glyph — the hit target on the interactive arm
    /// and the alignment cell on the passive one. Defaults to `size`.
    ///
    /// Args:
    ///     box_size: Unscaled design-reference box edge length.
    ///
    /// Returns:
    ///     This caret with the requested box size.
    pub fn box_size(mut self, box_size: f32) -> Self {
        self.box_size = Some(box_size);
        self
    }

    /// Override the default muted-text glyph colour.
    ///
    /// Args:
    ///     color: Packed Moon palette colour.
    ///
    /// Returns:
    ///     This caret with the requested glyph colour.
    pub fn color(mut self, color: u32) -> Self {
        self.color = Some(color);
        self
    }

    /// Colour taken while the pointer is over the caret's BOX, not merely over the glyph.
    ///
    /// Applied through a group, because an svg resolves its colour from its own style only:
    /// `compute_style_internal` starts from `Style::default()` and refines with the element's own
    /// base style, so a `text_color` set on the parent — hovered or not — never reaches the icon.
    /// Ignored on the passive arm, where a hover style would create the very hitbox it must not have.
    ///
    /// Args:
    ///     hover_color: Packed Moon palette colour used while the interactive box is hovered.
    ///
    /// Returns:
    ///     This caret with the requested hover colour.
    pub fn hover_color(mut self, hover_color: u32) -> Self {
        self.hover_color = Some(hover_color);
        self
    }

    /// Set the glyph alpha before any disabled-state reduction.
    ///
    /// Args:
    ///     alpha: Base glyph opacity.
    ///
    /// Returns:
    ///     This caret with the requested base opacity.
    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha;
        self
    }

    /// Set whether the caret suppresses pointer styling, hover colour, and toggle callbacks.
    ///
    /// Args:
    ///     disabled: Whether the caret is disabled.
    ///
    /// Returns:
    ///     This caret with the requested disabled state.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set tooltip text for the interactive arm.
    ///
    /// The passive arm ignores this value so a tooltip builder cannot create a hitbox.
    ///
    /// Args:
    ///     tooltip: Tooltip text shown over the interactive caret.
    ///
    /// Returns:
    ///     This caret with the requested tooltip text.
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Set a handler that receives the next expanded value on click.
    ///
    /// This handler is active only for [`MoonDisclosure::button`].
    ///
    /// Args:
    ///     handler: Callback receiving the toggled expanded value and UI contexts.
    ///
    /// Returns:
    ///     This caret with the requested toggle callback.
    pub fn on_toggle(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(std::rc::Rc::new(handler));
        self
    }

    /// Report whether this caret was constructed as the interactive arm.
    ///
    /// Returns:
    ///     `true` when the caret was constructed with [`MoonDisclosure::button`].
    pub(crate) fn is_interactive(&self) -> bool {
        self.id.is_some()
    }
}

impl MoonDisclosure {
    /// Build the caret box — the whole of this component's layout.
    ///
    /// Split out of [`RenderOnce::render`] so the geometry and the passive arm's inertness can be
    /// asserted directly: `render`'s `impl IntoElement` return cannot be inspected, and a caret
    /// that quietly grows a cursor is indistinguishable from a correct one until a click is
    /// swallowed in a live window.
    ///
    /// Args:
    ///     cx: Application context used to resolve active theme tokens.
    ///
    /// Returns:
    ///     The complete caret box without interactive identity or listeners.
    pub(crate) fn caret_box(&self, cx: &App) -> Div {
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;

        let interactive = self.is_interactive();
        let alpha = if self.disabled {
            self.alpha * DISABLED_ALPHA
        } else {
            self.alpha
        };
        let color = self.color.unwrap_or(p.text_muted);
        let glyph_px = tokens.ui(self.size);
        let box_px = tokens.ui(self.box_size.unwrap_or(self.size));
        let turns = moon_disclosure_rotation_turns(self.direction, self.expanded);

        // SVG painting builds the transformation matrix around `bounds.center()`, while layout and
        // hitbox calculation use the untransformed bounds. Rotation therefore changes only the
        // painted pose and cannot move the caret box or its neighbouring label.
        let mut icon = moon_icon(MOON_ICON_CARET_DOWN, glyph_px, color, alpha);
        if turns != 0.0 {
            icon = icon.with_transformation(Transformation::rotate(percentage(turns)));
        }

        let hover_group = self
            .hover_color
            .filter(|_| interactive && !self.disabled)
            .and_then(|hover| {
                self.id
                    .as_ref()
                    .map(|id| (SharedString::from(format!("{id}:disclosure")), hover))
            });

        if let Some((group, hover)) = hover_group.as_ref() {
            let hover = *hover;
            icon = icon.group_hover(group.clone(), move |this| {
                this.text_color(super::tokens::rgba_from(hover, alpha))
            });
        }

        div()
            .w(px(box_px))
            .h(px(box_px))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .when(interactive && !self.disabled, |this| this.cursor_pointer())
            .when_some(hover_group, |this, (group, _)| this.group(group))
            .child(icon)
    }
}

impl RenderOnce for MoonDisclosure {
    /// Render the passive caret directly or attach interactive behaviour to its box.
    ///
    /// Args:
    ///     _: Window context unused by this renderer.
    ///     cx: Application context used to resolve theme tokens and build a tooltip view.
    ///
    /// Returns:
    ///     The rendered disclosure caret.
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let boxed = self.caret_box(cx);
        let Some(id) = self.id.clone() else {
            return boxed.into_any_element();
        };

        let expanded = self.expanded;
        let disabled = self.disabled;
        let mut item = boxed.id(id);

        if let Some(tooltip) = self.tooltip.clone() {
            item = item.tooltip(move |_window, cx| {
                cx.new(|_| super::tooltip::MoonTooltipView::new(tooltip.clone()))
                    .into()
            });
        }

        if let Some(on_toggle) = self.on_toggle.clone() {
            item = item.on_click(move |_event, window, cx| {
                if let Some(next) = moon_disclosure_click_next(expanded, true, disabled) {
                    on_toggle(&next, window, cx);
                }
            });
        }

        item.into_any_element()
    }
}

#[cfg(test)]
mod tests;
