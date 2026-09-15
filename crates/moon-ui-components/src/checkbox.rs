use std::{rc::Rc, time::Duration};

use crate::{
    Disableable, Selectable, Sizable, Size, StyledExt as _,
    moon::MoonTone,
    moon::{MoonPalette, MoonTheme, MoonThemeTokens, rgba_from, svg::moon_svg},
    text::Text,
    tooltip::ComponentTooltip,
    v_flex,
};
use gpui::{
    Animation, AnimationExt, AnyElement, App, Div, ElementId, Empty, FontWeight, Hsla,
    InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, Window, div, prelude::FluentBuilder as _,
    px,
};

/// A Checkbox element.
#[derive(IntoElement)]
pub struct Checkbox {
    id: ElementId,
    base: Div,
    style: StyleRefinement,
    label: Option<Text>,
    description: Option<SharedString>,
    children: Vec<AnyElement>,
    checked: bool,
    indeterminate: bool,
    disabled: bool,
    size: Size,
    tone: Option<MoonTone>,
    mono: bool,
    tab_stop: bool,
    tab_index: isize,
    on_click: Option<Rc<dyn Fn(&bool, &mut Window, &mut App) + 'static>>,
    tooltip: ComponentTooltip,
}

impl Checkbox {
    /// Create a new Checkbox with the given id.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            base: div(),
            style: StyleRefinement::default(),
            label: None,
            description: None,
            children: Vec::new(),
            checked: false,
            indeterminate: false,
            disabled: false,
            size: Size::default(),
            tone: None,
            mono: false,
            on_click: None,
            tab_stop: true,
            tab_index: 0,
            tooltip: ComponentTooltip::default(),
        }
    }

    /// Set tooltip text for the checkbox.
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip.text = Some((tooltip.into(), None));
        self
    }

    /// Set the label for the checkbox.
    ///
    /// The box stays on the label's first line when the label wraps. An empty label renders no
    /// text, so the checkbox stays exactly its box with no gap.
    pub fn label(mut self, label: impl Into<Text>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set supporting text shown under the label in the muted text colour.
    ///
    /// The box stays on the label's first line rather than centring on label and description
    /// together. An empty description renders nothing.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the checked state for the checkbox.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Show the indeterminate state: a filled box with a minus mark. It renders as checked, so a
    /// click reports `false`.
    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        self.indeterminate = indeterminate;
        self
    }

    /// Set the Moon tone used for the checked mark and box.
    pub fn tone(mut self, tone: MoonTone) -> Self {
        self.tone = Some(tone);
        self
    }

    /// Render checkbox text with the Moon mono font.
    pub fn mono(mut self, mono: bool) -> Self {
        self.mono = mono;
        self
    }

    /// Set the click handler for the checkbox.
    ///
    /// The `&bool` parameter indicates the new checked state after the click.
    pub fn on_click(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Set the tab stop for the checkbox, default is true.
    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        self
    }

    /// Set the tab index for the checkbox, default is 0.
    pub fn tab_index(mut self, tab_index: isize) -> Self {
        self.tab_index = tab_index;
        self
    }

    fn handle_click(
        on_click: &Option<Rc<dyn Fn(&bool, &mut Window, &mut App) + 'static>>,
        checked: bool,
        window: &mut Window,
        cx: &mut App,
    ) {
        let new_checked = !checked;
        if let Some(f) = on_click {
            (f)(&new_checked, window, cx);
        }
    }
}

impl InteractiveElement for Checkbox {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.base.interactivity()
    }
}
impl StatefulInteractiveElement for Checkbox {}

impl Styled for Checkbox {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        &mut self.style
    }
}

impl Disableable for Checkbox {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Selectable for Checkbox {
    fn selected(self, selected: bool) -> Self {
        self.checked(selected)
    }

    fn is_selected(&self) -> bool {
        self.checked
    }
}

impl ParentElement for Checkbox {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Sizable for Checkbox {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

// Mark icons for the checked and indeterminate states. Their shapes come from the files; the
// stroke width comes from the size's metrics and the colour is applied as a tint.
const CHECK_ICON: &str = "icons/moon-checkbox-check.svg";
const MINUS_ICON: &str = "icons/moon-checkbox-minus.svg";

/// Whether a box shows its mark, and whether the mark arrived with a fade.
#[derive(Clone, Copy, Debug, PartialEq)]
enum MarkState {
    Hidden,
    /// Checked since the box first rendered, so the mark shows without a fade.
    Shown,
    /// Checked after the box first rendered. The mark keeps its fade wrapper while it shows, so
    /// the one-shot fade runs to the end instead of being cut off by the next render.
    FadingIn,
}

impl MarkState {
    /// Returns the state of a box first rendered `checked`: an already checked box (such as a
    /// checked row scrolling into view) shows its mark at once.
    fn initial(checked: bool) -> Self {
        if checked { Self::Shown } else { Self::Hidden }
    }

    /// Returns the state after a render with `checked`: only a box that becomes checked fades in.
    fn next(self, checked: bool) -> Self {
        match (checked, self) {
            (false, _) => Self::Hidden,
            (true, Self::Hidden) => Self::FadingIn,
            (true, shown) => shown,
        }
    }
}

/// The geometry of a checkbox size. Radios share it, so a radio lines up with the checkbox of the
/// same size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MoonCheckboxMetrics {
    pub(crate) box_size: gpui::Pixels,
    /// Font size and line height shared by the label and the description.
    pub(crate) font_size: gpui::Pixels,
    pub(crate) line_height: gpui::Pixels,
    pub(crate) label_weight: FontWeight,
    pub(crate) description_weight: FontWeight,
    pub(crate) gap: gpui::Pixels,
    /// Space between the label and the description under it.
    pub(crate) description_gap: gpui::Pixels,
    pub(crate) radius: gpui::Pixels,
    /// Gap from the box edge to the focus ring's outer edge; the ring's stroke lies inside it.
    pub(crate) focus_ring_distance: gpui::Pixels,
    pub(crate) focus_ring_width: gpui::Pixels,
    pub(crate) mark_size: gpui::Pixels,
    /// Rendered stroke width of the mark; `None` keeps the stroke authored in the icon file.
    pub(crate) mark_stroke: Option<gpui::Pixels>,
}

impl MoonCheckboxMetrics {
    fn for_size(size: Size, cx: &App) -> Self {
        Self::resolve(size, &MoonTheme::active_tokens(cx))
    }

    /// Resolves `size` against the theme's scale.
    ///
    /// The `Sm` and `Md` tiers are reviewed designs with fixed geometry, so they follow only the UI
    /// zoom (`scale.ui`): the theme's text scaling (`scale.font`, `scale.font_delta`) does not
    /// grow their box or their text. A `Size::Size` box keeps following text scaling.
    pub(crate) fn resolve(size: Size, tokens: &MoonThemeTokens) -> Self {
        let base = Self::base_for_size(size);
        match size {
            Size::Size(_) => base.scaled(tokens),
            _ => base.zoomed(tokens),
        }
    }

    fn base_for_size(size: Size) -> Self {
        match size {
            Size::XSmall | Size::Small => Self {
                box_size: px(16.),
                font_size: px(14.),
                line_height: px(20.),
                label_weight: FontWeight::MEDIUM,
                description_weight: FontWeight::NORMAL,
                gap: px(8.),
                description_gap: px(0.),
                radius: px(4.),
                focus_ring_distance: px(4.),
                focus_ring_width: px(2.),
                mark_size: px(12.),
                mark_stroke: Some(px(1.67)),
            },
            Size::Size(box_size) => Self {
                box_size,
                font_size: box_size * 0.75,
                line_height: box_size,
                label_weight: FontWeight::NORMAL,
                description_weight: FontWeight::NORMAL,
                gap: px(6.),
                description_gap: px(0.),
                radius: box_size * 0.25,
                focus_ring_distance: px(4.),
                focus_ring_width: px(2.),
                mark_size: (box_size - px(2.)).max(px(9.)),
                mark_stroke: None,
            },
            Size::Medium | Size::Large => Self {
                box_size: px(20.),
                font_size: px(16.),
                line_height: px(24.),
                label_weight: FontWeight::MEDIUM,
                description_weight: FontWeight::NORMAL,
                gap: px(12.),
                description_gap: px(2.),
                radius: px(6.),
                focus_ring_distance: px(4.),
                focus_ring_width: px(2.),
                mark_size: px(14.),
                mark_stroke: Some(px(2.)),
            },
        }
    }

    /// Scales every metric by the UI zoom alone, text included.
    fn zoomed(self, tokens: &MoonThemeTokens) -> Self {
        let ui = |value: gpui::Pixels| px(tokens.ui(value.as_f32()));
        Self {
            box_size: ui(self.box_size),
            font_size: ui(self.font_size),
            line_height: ui(self.line_height),
            label_weight: self.label_weight,
            description_weight: self.description_weight,
            gap: ui(self.gap),
            description_gap: ui(self.description_gap),
            radius: ui(self.radius),
            focus_ring_distance: ui(self.focus_ring_distance),
            focus_ring_width: ui(self.focus_ring_width),
            mark_size: ui(self.mark_size),
            mark_stroke: self.mark_stroke.map(ui),
        }
    }

    /// Scales geometry by the UI zoom and text by the theme's text scaling, growing the box with
    /// the text so a larger label never outgrows it.
    fn scaled(self, tokens: &MoonThemeTokens) -> Self {
        let line_height = tokens.line_height(self.line_height.as_f32());
        let box_size = tokens.ui(self.box_size.as_f32()).max(
            line_height.min(tokens.ui(self.box_size.as_f32()) + tokens.scale.font_delta.max(0.0)),
        );
        Self {
            box_size: px(box_size),
            font_size: px(tokens.font(self.font_size.as_f32())),
            line_height: px(line_height),
            label_weight: self.label_weight,
            description_weight: self.description_weight,
            gap: px(tokens.ui(self.gap.as_f32())),
            description_gap: px(tokens.ui(self.description_gap.as_f32())),
            radius: px(tokens.ui(self.radius.as_f32())),
            focus_ring_distance: px(tokens.ui(self.focus_ring_distance.as_f32())),
            focus_ring_width: px(tokens.ui(self.focus_ring_width.as_f32())),
            mark_size: px(tokens.ui(self.mark_size.as_f32())),
            mark_stroke: self
                .mark_stroke
                .map(|stroke| px(tokens.ui(stroke.as_f32()))),
        }
    }

    /// Returns the top margin that centres the box on the first line of a row with text, which is
    /// top-aligned so the box stays on that line however far the text wraps. Zero when the box is
    /// at least a line tall.
    pub(crate) fn box_offset(&self) -> gpui::Pixels {
        ((self.line_height - self.box_size) * 0.5).max(px(0.))
    }

    /// Returns the top margin of the text column when the box is taller than a line of text.
    fn text_offset(&self) -> gpui::Pixels {
        ((self.box_size - self.line_height) * 0.5).max(px(0.))
    }
}

/// Returns the text column beside a checkbox or radio box: the label, its description under it in
/// `description_color`, then `children`, on the size's line height. Pair it with a box pushed down
/// by [`MoonCheckboxMetrics::box_offset`] so the first line and the box stay centred on each other.
/// `id` prefixes the parts' debug selectors.
pub(crate) fn choice_text_column(
    id: &dyn std::fmt::Display,
    metrics: MoonCheckboxMetrics,
    label: Option<impl IntoElement>,
    description: Option<SharedString>,
    description_color: Hsla,
    children: Vec<AnyElement>,
) -> Div {
    v_flex()
        .flex_1()
        .overflow_hidden()
        .gap(metrics.description_gap)
        .line_height(metrics.line_height)
        .mt(metrics.text_offset())
        .when_some(label, |this, label| {
            this.child(
                div()
                    .debug_selector(|| format!("{id}:label"))
                    .font_weight(metrics.label_weight)
                    .child(label),
            )
        })
        .when_some(description, |this, description| {
            this.child(
                div()
                    .debug_selector(|| format!("{id}:description"))
                    .text_color(description_color)
                    .font_weight(metrics.description_weight)
                    .child(description),
            )
        })
        .children(children)
}

/// The colours of a checkbox or radio.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ChoiceColors {
    pub(crate) border: Hsla,
    pub(crate) fill: Hsla,
    /// The checkbox's check or minus, or the radio's dot.
    pub(crate) mark: Hsla,
    pub(crate) label: Hsla,
    pub(crate) description: Hsla,
    pub(crate) focus_ring: Hsla,
}

impl ChoiceColors {
    /// Resolves the colours of a box in `tone`. A checked box is one solid tone, border and fill
    /// alike, so its mark takes the palette ink that reads best on that tone rather than the tone
    /// itself. A disabled box draws everything at 45% opacity.
    pub(crate) fn resolve(p: MoonPalette, tone: MoonTone, checked: bool, disabled: bool) -> Self {
        let alpha = if disabled { 0.45 } else { 1.0 };
        let tone = tone.color(p);
        let (border, fill) = if checked {
            (rgba_from(tone, alpha), rgba_from(tone, alpha))
        } else {
            (
                rgba_from(p.border, alpha),
                rgba_from(p.shell_high, 0.95 * alpha),
            )
        };
        Self {
            border,
            fill,
            mark: rgba_from(p.ink_on(tone), alpha),
            label: rgba_from(if disabled { p.text_muted } else { p.text_soft }, alpha),
            description: rgba_from(p.text_muted, alpha),
            focus_ring: rgba_from(tone, 1.0),
        }
    }
}

/// Returns the focus ring for a checkbox or radio box whose corners have `box_radius`, to add as
/// the box's child. The ring is an absolute overlay, so it never changes the control's size; its
/// outer edge sits the size's focus ring distance outside the box, with corners that follow the
/// box's.
pub(crate) fn choice_focus_ring(
    id: &dyn std::fmt::Display,
    metrics: MoonCheckboxMetrics,
    box_radius: gpui::Pixels,
    color: Hsla,
) -> Div {
    // Insets start inside the box's 1px border, hence the extra pixel.
    let inset = -(metrics.focus_ring_distance + px(1.));
    div()
        .debug_selector(|| format!("{id}:focus-ring"))
        .absolute()
        .top(inset)
        .left(inset)
        .right(inset)
        .bottom(inset)
        .border(metrics.focus_ring_width)
        .border_color(color)
        .rounded(box_radius + metrics.focus_ring_distance)
}

/// Returns the mark built by `mark` while a box is `checked`, fading it in when the box becomes
/// checked after its first render; `None` for an unchecked box, which draws no mark. `id` keys the
/// box's mark state.
pub(crate) fn fading_mark<E: Styled + IntoElement + 'static>(
    id: ElementId,
    checked: bool,
    window: &mut Window,
    cx: &mut App,
    mark: impl FnOnce() -> E,
) -> Option<AnyElement> {
    let state = window.use_keyed_state(id, cx, |_, _| MarkState::initial(checked));
    let previous = *state.read(cx);
    let current = previous.next(checked);
    if current != previous {
        state.update(cx, |state, _| *state = current);
    }
    match current {
        MarkState::Hidden => None,
        MarkState::Shown => Some(mark().into_any_element()),
        MarkState::FadingIn => Some(
            mark()
                .with_animation(
                    "fade-in",
                    Animation::new(Duration::from_millis(250)),
                    |mark, delta| mark.opacity(delta),
                )
                .into_any_element(),
        ),
    }
}

/// Renders the check mark centred in a checkbox-sized box, for the base radio.
pub(crate) fn checkbox_check_icon(
    id: ElementId,
    size: Size,
    checked: bool,
    disabled: bool,
    checked_color: u32,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    let color = rgba_from(checked_color, if disabled { 0.45 } else { 1.0 });
    checkbox_mark(
        id,
        MoonCheckboxMetrics::for_size(size, cx),
        checked,
        color,
        CHECK_ICON,
        window,
        cx,
    )
    .unwrap_or_else(|| Empty.into_any_element())
}

/// Returns the mark `icon` tinted `color`, drawn at the size's stroke and centred in a box of
/// `metrics.box_size`, fading in when the box becomes checked; `None` for an unchecked box, which
/// draws no mark.
fn checkbox_mark(
    id: ElementId,
    metrics: MoonCheckboxMetrics,
    checked: bool,
    color: Hsla,
    icon: &'static str,
    window: &mut Window,
    cx: &mut App,
) -> Option<AnyElement> {
    fading_mark(id.clone(), checked, window, cx, || {
        // Absolute insets start inside the box's 1px border (`border_1` on both the checkbox and
        // radio boxes), so centring within the outer box takes that border back off.
        let offset = (metrics.box_size - metrics.mark_size) * 0.5 - px(1.);
        moon_svg(icon)
            .debug_selector(|| format!("{id}:mark"))
            .absolute()
            .top(offset)
            .left(offset)
            .size(metrics.mark_size)
            .when_some(metrics.mark_stroke, |mark, stroke| {
                mark.stroke_width(stroke)
            })
            .text_color(color)
    })
}

impl RenderOnce for Checkbox {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let checked = self.checked || self.indeterminate;
        let tokens = MoonTheme::active_tokens(cx);
        let metrics = MoonCheckboxMetrics::resolve(self.size, &tokens);
        let colors = ChoiceColors::resolve(
            tokens.palette,
            self.tone.unwrap_or(MoonTone::Info),
            checked,
            self.disabled,
        );

        let focus_handle = window
            .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        let is_focused = focus_handle.is_focused(window);

        // Empty text counts as no text, so a bare checkbox never gains the text column and, with
        // it, the box-to-text gap and a text line's height.
        let label = self
            .label
            .filter(|label| !matches!(label, Text::String(text) if text.is_empty()));
        let description = self
            .description
            .filter(|description| !description.is_empty());
        let has_text = label.is_some() || description.is_some() || !self.children.is_empty();
        let mark = checkbox_mark(
            self.id.clone(),
            metrics,
            checked,
            colors.mark,
            if self.indeterminate {
                MINUS_ICON
            } else {
                CHECK_ICON
            },
            window,
            cx,
        );

        // The plain wrapper keeps the row at its content height inside a flex parent that
        // stretches its items, so the hit area and the vertical alignment stay on the control.
        div().child(
            self.base
                .id(self.id.clone())
                .when(!self.disabled, |this| {
                    this.track_focus(
                        &focus_handle
                            .tab_stop(self.tab_stop)
                            .tab_index(self.tab_index),
                    )
                })
                .h_flex()
                .when(has_text, |this| this.items_start())
                .gap(metrics.gap)
                .text_size(metrics.font_size)
                .text_color(colors.label)
                .when(self.mono, |this| this.font_family(tokens.font_family(true)))
                .when(!self.disabled, |this| this.cursor_pointer())
                .refine_style(&self.style)
                .child(
                    div()
                        .debug_selector(|| format!("{}:box", self.id))
                        .relative()
                        .when(has_text, |this| this.mt(metrics.box_offset()))
                        .size(metrics.box_size)
                        .flex_shrink_0()
                        .border_1()
                        .border_color(colors.border)
                        .rounded(metrics.radius)
                        .bg(colors.fill)
                        .when(is_focused, |this| {
                            this.child(choice_focus_ring(
                                &self.id,
                                metrics,
                                metrics.radius,
                                colors.focus_ring,
                            ))
                        })
                        .children(mark),
                )
                // Label, description and any extra children share one text column; without text
                // there is no column, so the row is only the box and the gap never applies.
                .when(has_text, |this| {
                    this.child(choice_text_column(
                        &self.id,
                        metrics,
                        label,
                        description,
                        colors.description,
                        self.children,
                    ))
                })
                // Pressing an enabled checkbox focuses it (the focus handle is only tracked when
                // enabled), so Tab navigation continues from the clicked control.
                .when(!self.disabled, |this| {
                    this.on_click({
                        let on_click = self.on_click.clone();
                        move |_, window, cx| {
                            window.prevent_default();
                            Self::handle_click(&on_click, checked, window, cx);
                        }
                    })
                })
                .map(|this| self.tooltip.apply(this)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moon::svg::with_rendered_stroke;
    use std::{cell::RefCell, rc::Rc};

    #[test]
    fn test_moon_checkbox_metrics_match_terminal_palette() {
        let compact = MoonCheckboxMetrics::base_for_size(Size::XSmall);
        assert_eq!(compact.box_size, px(16.));
        assert_eq!(compact.font_size, px(14.));
        assert_eq!(compact.line_height, px(20.));
        assert_eq!(compact.label_weight, FontWeight::MEDIUM);
        assert_eq!(compact.description_weight, FontWeight::NORMAL);
        assert_eq!(compact.gap, px(8.));
        assert_eq!(compact.description_gap, px(0.));
        assert_eq!(compact.radius, px(4.));

        let normal = MoonCheckboxMetrics::base_for_size(Size::Medium);
        assert_eq!(normal.box_size, px(20.));
        assert_eq!(normal.font_size, px(16.));
        assert_eq!(normal.line_height, px(24.));
        assert_eq!(normal.label_weight, FontWeight::MEDIUM);
        assert_eq!(normal.description_weight, FontWeight::NORMAL);
        assert_eq!(normal.gap, px(12.));
        assert_eq!(normal.description_gap, px(2.));
        assert_eq!(normal.radius, px(6.));
    }

    /// Catches `MoonCheckboxMetrics::resolve` letting the theme's text scaling resize a tier: an app
    /// with a "font +3" setting would draw an `Sm` checkbox with a 19px box and a 17px label, the
    /// size of `Md`. The `Sm`/`Md` tiers must follow only the UI zoom, while a `Size::Size` box
    /// keeps following text scaling.
    #[test]
    fn test_checkbox_tiers_ignore_text_scaling_and_follow_ui_zoom() {
        use crate::moon::MoonThemeConfig;

        let font_plus_three = MoonThemeConfig::moon_terminal().with_font_delta(3.0).dark;
        for (size, box_px, font_px, line_px) in
            [(Size::Small, 16., 14., 20.), (Size::Medium, 20., 16., 24.)]
        {
            let metrics = MoonCheckboxMetrics::resolve(size, &font_plus_three);
            assert_eq!(metrics.box_size, px(box_px));
            assert_eq!(metrics.font_size, px(font_px));
            assert_eq!(metrics.line_height, px(line_px));
        }

        let zoomed = MoonThemeConfig::moon_terminal()
            .with_font_delta(3.0)
            .with_ui_scale(1.5)
            .dark;
        let small = MoonCheckboxMetrics::resolve(Size::Small, &zoomed);
        assert_eq!(small.box_size, px(24.));
        assert_eq!(small.font_size, px(21.));
        assert_eq!(small.line_height, px(30.));
        assert_eq!(small.mark_size, px(18.));

        let custom = MoonCheckboxMetrics::resolve(Size::Size(px(12.)), &font_plus_three);
        assert_eq!(custom.font_size, px(font_plus_three.font(9.)));
    }

    /// Catches the check and indeterminate minus icons' stroke not rendering at its reviewed width
    /// (1.67px in the 12px small mark, 2px in the 14px medium mark): `with_rendered_stroke` must
    /// rewrite each shipped icon's `stroke-width` against its 24-unit viewBox and leave the shape
    /// untouched, and the icons must stay parsable, or a checked box paints no mark at all.
    #[test]
    fn test_check_icon_stroke_is_rewritten_to_design_width() {
        let check =
            include_str!("../../moon-ui-components-assets/assets/icons/moon-checkbox-check.svg");
        let minus =
            include_str!("../../moon-ui-components-assets/assets/icons/moon-checkbox-minus.svg");
        for (icon, shape) in [(check, "d=\"M20 6L9 17L4 12\""), (minus, "d=\"M5 12H19\"")] {
            for (size, stroke, file_width) in [
                (Size::Small, px(1.67), "3.3400"),
                (Size::Medium, px(2.), "3.4286"),
            ] {
                let metrics = MoonCheckboxMetrics::base_for_size(size);
                assert_eq!(metrics.mark_stroke, Some(stroke));

                let svg = with_rendered_stroke(icon, stroke.as_f32(), metrics.mark_size.as_f32())
                    .expect("mark icon must have a viewBox");
                assert!(svg.contains(&format!("stroke-width=\"{file_width}\"")));
                assert_eq!(svg.matches("stroke-width=").count(), 1);
                assert!(svg.contains(shape));
            }
        }
    }

    /// Catches the shared checkbox and radio colours drifting from the reviewed design: a checked box
    /// must be one solid tone, border and fill alike, with its mark in the palette ink that reads
    /// best on that tone; an unchecked box keeps the neutral border and fill; disabled boxes fade to
    /// 45%; and the focus ring stays the full tone.
    #[test]
    fn test_choice_colors_fill_a_checked_box_with_its_tone() {
        let p = crate::moon::MoonThemeConfig::moon_terminal().dark.palette;
        for tone in [MoonTone::Info, MoonTone::Warning, MoonTone::Default] {
            let tone_rgb = tone.color(p);
            for (disabled, alpha) in [(false, 1.0), (true, 0.45)] {
                let checked = ChoiceColors::resolve(p, tone, true, disabled);
                assert_eq!(checked.border, rgba_from(tone_rgb, alpha));
                assert_eq!(checked.fill, rgba_from(tone_rgb, alpha));
                assert_eq!(checked.mark, rgba_from(p.ink_on(tone_rgb), alpha));
                assert_eq!(checked.focus_ring, rgba_from(tone_rgb, 1.0));

                let unchecked = ChoiceColors::resolve(p, tone, false, disabled);
                assert_eq!(unchecked.border, rgba_from(p.border, alpha));
                assert_eq!(unchecked.fill, rgba_from(p.shell_high, 0.95 * alpha));
            }
        }
    }

    /// Catches the check mark fading when it should simply show, or showing when it should fade: a
    /// box already checked when it first renders (a checked row scrolling into view) must show its
    /// mark at once, while checking a rendered box must fade the mark in and keep that fade until
    /// the box is unchecked, so the fade is neither cut off nor replayed.
    #[test]
    fn test_mark_fades_in_only_when_checked_after_first_render() {
        assert_eq!(MarkState::initial(true), MarkState::Shown);
        assert_eq!(MarkState::initial(false), MarkState::Hidden);
        assert_eq!(MarkState::Shown.next(true), MarkState::Shown);
        assert_eq!(MarkState::Hidden.next(true), MarkState::FadingIn);
        assert_eq!(MarkState::FadingIn.next(true), MarkState::FadingIn);
        for state in [MarkState::Hidden, MarkState::Shown, MarkState::FadingIn] {
            assert_eq!(state.next(false), MarkState::Hidden);
        }
    }

    struct ToggledCheckboxHarness {
        checked: bool,
    }

    impl gpui::Render for ToggledCheckboxHarness {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            div().child(Checkbox::new("toggled").checked(self.checked).small())
        }
    }

    /// Catches checking a rendered box failing to draw its mark: when an unchecked box becomes
    /// checked, `checkbox_mark` must draw the fading mark at its reviewed 12px size (not skip it or
    /// panic updating its state mid-render), and must drop the mark again once unchecked.
    #[gpui::test]
    fn test_checking_a_rendered_box_draws_its_mark(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);
        let window = cx.add_window(|_, _| ToggledCheckboxHarness { checked: false });
        let mut cx = gpui::VisualTestContext::from_window(window.into(), cx);
        cx.run_until_parked();
        assert!(cx.debug_bounds("toggled:mark").is_none());

        for checked in [true, false] {
            window
                .update(&mut cx, |view, _, cx| {
                    view.checked = checked;
                    cx.notify();
                })
                .expect("window must stay open");
            cx.run_until_parked();

            let mark = cx.debug_bounds("toggled:mark");
            if checked {
                let mark = mark.expect("a box checked after rendering must draw its mark");
                assert_eq!(mark.size, gpui::size(px(12.), px(12.)));
            } else {
                assert!(mark.is_none(), "an unchecked box must draw no mark");
            }
        }
    }

    struct CheckedCheckboxHarness {
        size: Size,
    }

    impl gpui::Render for CheckedCheckboxHarness {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            div().child(Checkbox::new("probe").checked(true).with_size(self.size))
        }
    }

    /// Catches the check mark drifting from its reviewed size (12px in the 16px small box, 14px in
    /// the 20px medium box) or off the box centre, which a 1px border shifts if the mark offset in
    /// `checkbox_check_icon` is measured from the wrong edge.
    #[gpui::test]
    fn test_checked_mark_is_sized_and_centred_in_its_box(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);
        for (size, box_px, mark_px) in [(Size::Small, 16., 12.), (Size::Medium, 20., 14.)] {
            let window = cx.add_window(move |_, _| CheckedCheckboxHarness { size });
            let mut cx = gpui::VisualTestContext::from_window(window.into(), cx);
            cx.run_until_parked();

            let box_bounds = cx.debug_bounds("probe:box").expect("box must render");
            let mark_bounds = cx.debug_bounds("probe:mark").expect("mark must render");
            assert_eq!(box_bounds.size, gpui::size(px(box_px), px(box_px)));
            assert_eq!(mark_bounds.size, gpui::size(px(mark_px), px(mark_px)));
            assert_eq!(mark_bounds.center(), box_bounds.center());
        }
    }

    struct DescribedCheckboxHarness {
        size: Size,
    }

    impl gpui::Render for DescribedCheckboxHarness {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            div().w(px(240.)).child(
                Checkbox::new("described")
                    .label("Remember me")
                    .description("Save my login details for next time")
                    .with_size(self.size),
            )
        }
    }

    /// Catches a description pulling the box off the label or the reviewed spacing drifting:
    /// `Checkbox::render` must centre the label's line on the box (not on label and description
    /// together), keep the box-to-text gap (8px small, 12px medium), and stack the description
    /// under the label at the description gap (0px small, 2px medium) in the same text column.
    #[gpui::test]
    fn test_description_stacks_under_label_aligned_with_box(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);
        for (size, gap, description_gap) in [(Size::Small, 8., 0.), (Size::Medium, 12., 2.)] {
            let window = cx.add_window(move |_, _| DescribedCheckboxHarness { size });
            let mut cx = gpui::VisualTestContext::from_window(window.into(), cx);
            cx.run_until_parked();

            let box_bounds = cx.debug_bounds("described:box").expect("box must render");
            let label = cx
                .debug_bounds("described:label")
                .expect("label must render");
            let description = cx
                .debug_bounds("described:description")
                .expect("description must render");
            assert_eq!(label.center().y, box_bounds.center().y);
            assert_eq!(description.top() - label.bottom(), px(description_gap));
            assert_eq!(description.left(), label.left());
            assert_eq!(label.left() - box_bounds.right(), px(gap));
        }
    }

    /// Catches the focus ring changing the control's layout or drifting from its reviewed geometry:
    /// on keyboard focus `Checkbox::render` must draw a ring whose outer edge sits 4px outside the
    /// box on every side, while the box and label keep exactly their unfocused bounds.
    #[gpui::test]
    fn test_focus_ring_surrounds_box_without_moving_layout(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);
        for size in [Size::Small, Size::Medium] {
            let window = cx.add_window(move |_, _| DescribedCheckboxHarness { size });
            let mut cx = gpui::VisualTestContext::from_window(window.into(), cx);
            cx.run_until_parked();

            let box_before = cx.debug_bounds("described:box").expect("box must render");
            let label_before = cx
                .debug_bounds("described:label")
                .expect("label must render");
            assert!(cx.debug_bounds("described:focus-ring").is_none());

            cx.update(|window, cx| window.focus_next(cx));
            cx.run_until_parked();

            let ring = cx
                .debug_bounds("described:focus-ring")
                .expect("focused checkbox must draw its ring");
            assert_eq!(cx.debug_bounds("described:box"), Some(box_before));
            assert_eq!(cx.debug_bounds("described:label"), Some(label_before));
            assert_eq!(ring.origin, box_before.origin - gpui::point(px(4.), px(4.)));
            assert_eq!(ring.size, box_before.size + gpui::size(px(8.), px(8.)));
        }
    }

    struct ProbedCheckboxHarness {
        size: Size,
        label: Option<&'static str>,
    }

    impl gpui::Render for ProbedCheckboxHarness {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            let checkbox = Checkbox::new("probed").checked(true).with_size(self.size);
            // A flex row shrinks the probe to its content, so the probe measures the checkbox.
            crate::h_flex().child(div().debug_selector(|| "probed-control".into()).child(
                match self.label {
                    Some(label) => checkbox.label(label),
                    None => checkbox,
                },
            ))
        }
    }

    /// Renders a `ProbedCheckboxHarness` window and returns its visual test context.
    fn probe_checkbox(
        cx: &mut gpui::TestAppContext,
        size: Size,
        label: Option<&'static str>,
    ) -> gpui::VisualTestContext {
        let window = cx.add_window(move |_, _| ProbedCheckboxHarness { size, label });
        let cx = gpui::VisualTestContext::from_window(window.into(), cx);
        cx.run_until_parked();
        cx
    }

    /// Catches `Checkbox::render` reserving text space on a checkbox with no text: adding the text
    /// column (and with it the box-to-text gap) for a missing or empty label would widen a bare
    /// checkbox, such as a tree row's, by 8px (small) or 12px (medium) and push the next cell away.
    #[gpui::test]
    fn test_checkbox_without_text_renders_only_its_box(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);
        for (size, box_px) in [(Size::Small, 16.), (Size::Medium, 20.)] {
            for label in [None, Some("")] {
                let mut cx = probe_checkbox(cx, size, label);
                let control = cx
                    .debug_bounds("probed-control")
                    .expect("probe must render");
                assert_eq!(
                    control.size,
                    gpui::size(px(box_px), px(box_px)),
                    "{size:?} checkbox with label {label:?} must be exactly its box"
                );
            }
        }
    }

    /// Catches a label-only row losing its reviewed line: if the text column stops giving the label
    /// the tier's line height (20px small, 24px medium) or the row stops centring the box on it,
    /// the label sits cramped or the box drifts off the text.
    #[gpui::test]
    fn test_label_only_row_centres_box_on_one_text_line(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);
        for (size, line_px) in [(Size::Small, 20.), (Size::Medium, 24.)] {
            let mut cx = probe_checkbox(cx, size, Some("Only active"));
            let control = cx
                .debug_bounds("probed-control")
                .expect("probe must render");
            let box_bounds = cx.debug_bounds("probed:box").expect("box must render");
            let label = cx.debug_bounds("probed:label").expect("label must render");
            assert_eq!(control.size.height, px(line_px));
            assert_eq!(label.size.height, px(line_px));
            assert_eq!(label.center().y, box_bounds.center().y);
        }
    }

    struct WrappingCheckboxHarness {
        size: Size,
    }

    impl gpui::Render for WrappingCheckboxHarness {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            div().w(px(160.)).child(
                Checkbox::new("wrapping")
                    .label("Close every open position when the stop price is reached")
                    .with_size(self.size),
            )
        }
    }

    /// Catches the box following a label as it wraps: without a description the row must still be
    /// top-aligned, so the box stays centred on the first line (20px small, 24px medium) instead of
    /// sliding to the middle of the wrapped block.
    #[gpui::test]
    fn test_wrapping_label_keeps_box_on_its_first_line(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);
        for (size, line_px) in [(Size::Small, 20.), (Size::Medium, 24.)] {
            let window = cx.add_window(move |_, _| WrappingCheckboxHarness { size });
            let mut cx = gpui::VisualTestContext::from_window(window.into(), cx);
            cx.run_until_parked();

            let box_bounds = cx.debug_bounds("wrapping:box").expect("box must render");
            let label = cx
                .debug_bounds("wrapping:label")
                .expect("label must render");
            assert!(
                label.size.height >= px(line_px * 2.),
                "{size:?} label must wrap for this case to mean anything"
            );
            assert_eq!(box_bounds.center().y, label.top() + px(line_px / 2.));
        }
    }

    struct TwoCheckboxView;

    impl gpui::Render for TwoCheckboxView {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            div()
                .child(Checkbox::new("first").label("First"))
                .child(Checkbox::new("second").label("Second"))
        }
    }

    /// Catches a checkbox that swallows focus on press: clicking it in a window where nothing is
    /// focused must focus it (showing its ring), so Root's Tab handling can then move focus on to
    /// the next checkbox. Without a focused start point Tab never reaches Root at all.
    #[gpui::test]
    fn test_click_focuses_checkbox_and_tab_continues(cx: &mut gpui::TestAppContext) {
        use gpui::AppContext as _;

        cx.update(crate::init);
        let window = cx.add_window(|window, cx| {
            let view = cx.new(|_| TwoCheckboxView);
            crate::Root::new(view, window, cx).bordered(false)
        });
        let mut cx = gpui::VisualTestContext::from_window(window.into(), cx);
        cx.run_until_parked();
        assert!(cx.debug_bounds("first:focus-ring").is_none());

        let label = cx.debug_bounds("first:label").expect("label must render");
        cx.simulate_click(label.center(), gpui::Modifiers::none());
        cx.run_until_parked();
        assert!(cx.debug_bounds("first:focus-ring").is_some());

        cx.simulate_keystrokes("tab");
        cx.run_until_parked();
        assert!(cx.debug_bounds("first:focus-ring").is_none());
        assert!(cx.debug_bounds("second:focus-ring").is_some());
    }

    #[test]
    fn test_checkbox_builder_keeps_longbridge_api() {
        let checkbox = Checkbox::new("moon-checkbox")
            .label("Only active")
            .checked(true)
            .small()
            .disabled(false)
            .tab_index(3)
            .tab_stop(false);

        assert!(checkbox.checked);
        assert_eq!(checkbox.size, Size::Small);
        assert_eq!(checkbox.tab_index, 3);
        assert!(!checkbox.tab_stop);
        assert!(!checkbox.disabled);
    }

    #[gpui::test]
    fn test_checkbox_handle_click_toggles_and_calls_handler(cx: &mut gpui::TestAppContext) {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let window = cx.add_empty_window();

        window.update(|window, cx| {
            let on_click: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>> = Some(Rc::new({
                let seen = seen.clone();
                move |checked, _, _| seen.borrow_mut().push(*checked)
            }));

            Checkbox::handle_click(&on_click, false, window, cx);
            Checkbox::handle_click(&on_click, true, window, cx);
        });

        assert_eq!(&*seen.borrow(), &[true, false]);
    }
}
