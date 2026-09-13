use std::{
    collections::HashMap,
    rc::Rc,
    sync::{Arc, LazyLock, Mutex},
    time::Duration,
};

use crate::{
    Disableable, Selectable, Sizable, Size, StyledExt as _,
    moon::MoonTone,
    moon::{MoonPalette, MoonTheme, MoonThemeTokens, rgba_from},
    text::Text,
    tooltip::ComponentTooltip,
    v_flex,
};
use gpui::{
    Animation, AnimationExt, AnyElement, App, Div, ElementId, FontWeight, InteractiveElement,
    IntoElement, ParentElement, RenderOnce, SharedString, StatefulInteractiveElement,
    StyleRefinement, Styled, TransformationMatrix, Window, canvas, div,
    prelude::FluentBuilder as _, px, relative,
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
    pub fn label(mut self, label: impl Into<Text>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set supporting text shown under the label in the muted text colour.
    ///
    /// With a description the box aligns to the label's line instead of centring on both lines.
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

// Mark icons for the checked and indeterminate states. Their shapes come from the files; their
// stroke width is rewritten per size at paint time and their colour is applied as a tint.
const CHECK_ICON: &str = "icons/moon-checkbox-check.svg";
const MINUS_ICON: &str = "icons/moon-checkbox-minus.svg";

/// Stroke-rewritten mark icon text by atlas key, so each SVG is edited once per stroke ratio
/// rather than on every paint.
static STROKED_MARK_ICONS: LazyLock<Mutex<HashMap<SharedString, Arc<[u8]>>>> =
    LazyLock::new(Default::default);

#[derive(Clone, Copy)]
struct MoonCheckboxMetrics {
    box_size: gpui::Pixels,
    /// Font size and line height shared by the label and the description.
    font_size: gpui::Pixels,
    line_height: gpui::Pixels,
    label_weight: FontWeight,
    description_weight: FontWeight,
    gap: gpui::Pixels,
    /// Space between the label and the description under it.
    description_gap: gpui::Pixels,
    radius: gpui::Pixels,
    /// Gap from the box edge to the focus ring's outer edge; the ring's stroke lies inside it.
    focus_ring_distance: gpui::Pixels,
    focus_ring_width: gpui::Pixels,
    mark_size: gpui::Pixels,
    /// Rendered stroke width of the mark; `None` keeps the stroke authored in the icon file.
    mark_stroke: Option<gpui::Pixels>,
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
    fn resolve(size: Size, tokens: &MoonThemeTokens) -> Self {
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
}

/// Rewrites every `stroke-width` in `svg` so the stroke renders `stroke` wide when the SVG is
/// drawn `drawn_size` wide, scaling against the SVG's own `viewBox`.
///
/// Returns `None` when `svg` has no parsable `viewBox` width or an unterminated `stroke-width`.
fn with_rendered_stroke(svg: &str, stroke: f32, drawn_size: f32) -> Option<String> {
    let view_box_width: f32 = svg
        .split_once("viewBox=\"")?
        .1
        .split('"')
        .next()?
        .split_whitespace()
        .nth(2)?
        .parse()
        .ok()?;
    let width = format!("{:.4}", stroke * view_box_width / drawn_size);

    let mut rewritten = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some((before, after)) = rest.split_once("stroke-width=\"") {
        rewritten.push_str(before);
        rewritten.push_str("stroke-width=\"");
        rewritten.push_str(&width);
        rewritten.push('"');
        rest = after.split_once('"')?.1;
    }
    rewritten.push_str(rest);
    Some(rewritten)
}

/// Returns the sprite-atlas key and SVG bytes to paint the mark `icon` with a `stroke`-wide stroke
/// at `mark_size`. Without a `stroke` the icon is painted as authored, and the bytes are `None` so
/// the atlas loads the file itself.
///
/// Returns `None` when the icon asset cannot be loaded or rewritten, in which case nothing paints.
fn mark_icon_svg(
    icon: &'static str,
    stroke: Option<gpui::Pixels>,
    mark_size: gpui::Pixels,
    cx: &App,
) -> Option<(SharedString, Option<Arc<[u8]>>)> {
    let Some(stroke) = stroke else {
        return Some((icon.into(), None));
    };
    // The ratio, not the pixel width, names the variant: UI scaling changes both the stroke and
    // the mark, and the atlas key already carries the drawn size.
    let key = SharedString::from(format!(
        "{icon}#stroke={:.4}",
        stroke.as_f32() / mark_size.as_f32()
    ));
    let mut cache = STROKED_MARK_ICONS.lock().ok()?;
    if let Some(bytes) = cache.get(&key) {
        return Some((key, Some(bytes.clone())));
    }
    let source = cx.asset_source().load(icon).ok()??;
    let svg = with_rendered_stroke(
        std::str::from_utf8(&source).ok()?,
        stroke.as_f32(),
        mark_size.as_f32(),
    )?;
    let bytes: Arc<[u8]> = svg.into_bytes().into();
    cache.insert(key.clone(), bytes.clone());
    Some((key, Some(bytes)))
}

/// Renders the check mark SVG centred in the box and fades it in or out when the checked state
/// flips.
pub(crate) fn checkbox_check_icon(
    id: ElementId,
    size: Size,
    checked: bool,
    disabled: bool,
    checked_color: u32,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    checkbox_mark(
        id,
        size,
        checked,
        disabled,
        checked_color,
        CHECK_ICON,
        window,
        cx,
    )
}

/// Renders the mark `icon` centred in the box and fades it in or out when the checked state flips.
#[allow(clippy::too_many_arguments)]
fn checkbox_mark(
    id: ElementId,
    size: Size,
    checked: bool,
    disabled: bool,
    checked_color: u32,
    icon: &'static str,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let selector = format!("{id}:mark");
    let toggle_state = window.use_keyed_state(id, cx, |_, _| checked);
    let metrics = MoonCheckboxMetrics::for_size(size, cx);
    let color = rgba_from(checked_color, if disabled { 0.45 } else { 1.0 });
    // Absolute insets start inside the box's 1px border (`border_1` on both the checkbox and radio
    // boxes), so centring within the outer box takes that border back off.
    let mark_offset = (metrics.box_size - metrics.mark_size) * 0.5 - px(1.);

    div()
        .debug_selector(|| selector)
        .absolute()
        .top(mark_offset)
        .left(mark_offset)
        .size(metrics.mark_size)
        .child(
            canvas(
                |_, _, _| {},
                move |bounds, _, window, cx| {
                    // Loaded from the embedded MoonAssets source by asset path, so the mark also
                    // renders in distributed builds.
                    if checked
                        && let Some((key, bytes)) =
                            mark_icon_svg(icon, metrics.mark_stroke, metrics.mark_size, cx)
                    {
                        _ = window.paint_svg(
                            bounds,
                            key,
                            bytes.as_deref(),
                            TransformationMatrix::default(),
                            color.into(),
                            cx,
                        );
                    }
                },
            )
            .size_full(),
        )
        .map(|this| {
            if !disabled && checked != *toggle_state.read(cx) {
                let duration = Duration::from_secs_f64(0.25);
                cx.spawn({
                    let toggle_state = toggle_state.clone();
                    async move |cx| {
                        cx.background_executor().timer(duration).await;
                        _ = toggle_state.update(cx, |this, _| *this = checked);
                    }
                })
                .detach();

                this.with_animation(
                    ElementId::NamedInteger("toggle".into(), checked as u64),
                    Animation::new(Duration::from_secs_f64(0.25)),
                    move |this, delta| {
                        this.opacity(if checked { 1.0 * delta } else { 1.0 - delta })
                    },
                )
                .into_any_element()
            } else {
                this.into_any_element()
            }
        })
}

impl RenderOnce for Checkbox {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let checked = self.checked || self.indeterminate;
        let metrics = MoonCheckboxMetrics::for_size(self.size, cx);
        let p = MoonPalette::active(cx);
        let checked_tone = self.tone.unwrap_or(MoonTone::Info).color(p);
        let box_alpha = if self.disabled { 0.45 } else { 1.0 };
        let label_alpha = if self.disabled { 0.45 } else { 1.0 };

        let focus_handle = window
            .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        let is_focused = focus_handle.is_focused(window);

        // A checked box is one solid tone, border and fill alike, so the mark takes the palette
        // ink that reads best on that tone rather than the tone itself.
        let (border_color, bg_color) = if checked {
            let tone = rgba_from(checked_tone, box_alpha);
            (tone, tone)
        } else {
            (
                rgba_from(p.border, box_alpha),
                rgba_from(p.shell_high, 0.95 * box_alpha),
            )
        };
        let label_color = if self.disabled {
            rgba_from(p.text_muted, label_alpha)
        } else {
            rgba_from(p.text_soft, label_alpha)
        };
        let description_color = rgba_from(p.text_muted, label_alpha);
        let has_description = self.description.is_some();
        // Rows with a description are top-aligned; centre the label's first line on the box by
        // pushing whichever of the two is shorter down by half the difference.
        let box_offset = ((metrics.line_height - metrics.box_size) * 0.5).max(px(0.));
        let text_offset = ((metrics.box_size - metrics.line_height) * 0.5).max(px(0.));
        let focus_ring_selector = format!("{}:focus-ring", self.id);
        let label_selector = format!("{}:label", self.id);
        let description_selector = format!("{}:description", self.id);

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
                .gap(metrics.gap)
                .map(|this| {
                    if has_description {
                        this.items_start()
                    } else {
                        this.items_center()
                    }
                })
                .line_height(metrics.line_height)
                .text_size(metrics.font_size)
                .text_color(label_color)
                .when(self.mono, |this| {
                    this.font_family(MoonTheme::active_tokens(cx).font_family(true))
                })
                .rounded(px(4.))
                .when(!self.disabled, |this| this.cursor_pointer())
                .refine_style(&self.style)
                .child(
                    div()
                        .debug_selector(|| format!("{}:box", self.id))
                        .relative()
                        .when(has_description, |this| this.mt(box_offset))
                        .size(metrics.box_size)
                        .flex_shrink_0()
                        .border_1()
                        .border_color(border_color)
                        .rounded(metrics.radius)
                        .bg(bg_color)
                        .when(is_focused, |this| {
                            // An absolute overlay, so the ring never changes the control's size.
                            // Insets start inside the box's 1px border, hence the extra pixel.
                            let inset = -(metrics.focus_ring_distance + px(1.));
                            this.child(
                                div()
                                    .debug_selector(|| focus_ring_selector)
                                    .absolute()
                                    .top(inset)
                                    .left(inset)
                                    .right(inset)
                                    .bottom(inset)
                                    .border(metrics.focus_ring_width)
                                    .border_color(rgba_from(checked_tone, 1.0))
                                    .rounded(metrics.radius + metrics.focus_ring_distance),
                            )
                        })
                        .child(checkbox_mark(
                            self.id,
                            self.size,
                            checked,
                            self.disabled,
                            p.ink_on(checked_tone),
                            if self.indeterminate {
                                MINUS_ICON
                            } else {
                                CHECK_ICON
                            },
                            window,
                            cx,
                        )),
                )
                .when(
                    self.label.is_some() || self.description.is_some() || !self.children.is_empty(),
                    |this| {
                        this.child(
                            v_flex()
                                .flex_1()
                                .overflow_hidden()
                                .line_height(relative(1.2))
                                .gap_1()
                                .when(has_description, |this| this.mt(text_offset))
                                .when(self.label.is_some() || has_description, |this| {
                                    this.child(
                                        v_flex()
                                            .gap(metrics.description_gap)
                                            .when_some(self.label, |this, label| {
                                                this.child(
                                                    div()
                                                        .debug_selector(|| label_selector)
                                                        .size_full()
                                                        .text_color(label_color)
                                                        .font_weight(metrics.label_weight)
                                                        .line_height(metrics.line_height)
                                                        .child(label),
                                                )
                                            })
                                            .when_some(self.description, |this, description| {
                                                this.child(
                                                    div()
                                                        .debug_selector(|| description_selector)
                                                        .text_color(description_color)
                                                        .font_weight(metrics.description_weight)
                                                        .line_height(metrics.line_height)
                                                        .child(description),
                                                )
                                            }),
                                    )
                                })
                                .children(self.children),
                        )
                    },
                )
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
