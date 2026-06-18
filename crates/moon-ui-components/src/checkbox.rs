use std::{rc::Rc, time::Duration};

use crate::{
    Disableable, FocusableExt, Selectable, Sizable, Size, StyledExt as _,
    moon_skin::{MoonSkinPalette, moon_color},
    text::Text,
    tooltip::ComponentTooltip,
    v_flex,
};
use gpui::{
    Animation, AnimationExt, AnyElement, App, Div, ElementId, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, StyleRefinement, Styled,
    Window, div, prelude::FluentBuilder as _, px, relative, svg,
};

/// A Checkbox element.
#[derive(IntoElement)]
pub struct Checkbox {
    id: ElementId,
    base: Div,
    style: StyleRefinement,
    label: Option<Text>,
    children: Vec<AnyElement>,
    checked: bool,
    disabled: bool,
    size: Size,
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
            children: Vec::new(),
            checked: false,
            disabled: false,
            size: Size::default(),
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

    /// Set the checked state for the checkbox.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
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

#[derive(Clone, Copy)]
struct MoonCheckboxMetrics {
    box_size: gpui::Pixels,
    font_size: gpui::Pixels,
    line_height: gpui::Pixels,
    gap: gpui::Pixels,
    radius: gpui::Pixels,
}

impl MoonCheckboxMetrics {
    fn for_size(size: Size) -> Self {
        match size {
            Size::XSmall | Size::Small => Self {
                box_size: px(12.),
                font_size: px(9.5),
                line_height: px(12.),
                gap: px(6.),
                radius: px(3.),
            },
            Size::Size(box_size) => Self {
                box_size,
                font_size: box_size * 0.75,
                line_height: box_size,
                gap: px(6.),
                radius: box_size * 0.25,
            },
            Size::Medium | Size::Large => Self {
                box_size: px(14.),
                font_size: px(10.5),
                line_height: px(13.),
                gap: px(7.),
                radius: px(3.5),
            },
        }
    }
}

pub(crate) fn checkbox_check_icon(
    id: ElementId,
    size: Size,
    checked: bool,
    disabled: bool,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    let toggle_state = window.use_keyed_state(id, cx, |_, _| checked);
    let p = MoonSkinPalette::TERMINAL;
    let metrics = MoonCheckboxMetrics::for_size(size);
    let mark_size = px((metrics.box_size.as_f32() - 2.0).max(9.0));
    let mark_offset = (metrics.box_size - mark_size) * 0.5;
    let color = moon_color(p.blue, if disabled { 0.45 } else { 1.0 });

    svg()
        .absolute()
        .top(mark_offset)
        .left(mark_offset)
        .size(mark_size)
        .text_color(color)
        .map(|this| match checked {
            // Load via external_path (filesystem) like the other Moon icons — the terminal
            // registers no gpui AssetSource, so IconName::Check.path() ("icons/check.svg")
            // resolves to nothing and the box renders empty.
            true => this.external_path(crate::moon::MOON_ICON_CHECK),
            _ => this,
        })
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
        let checked = self.checked;
        let metrics = MoonCheckboxMetrics::for_size(self.size);
        let p = MoonSkinPalette::TERMINAL;
        let box_alpha = if self.disabled { 0.45 } else { 1.0 };
        let label_alpha = if self.disabled { 0.45 } else { 1.0 };

        let focus_handle = window
            .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        let is_focused = focus_handle.is_focused(window);

        let border_color = moon_color(
            if checked { p.blue } else { p.border },
            if checked { 0.75 * box_alpha } else { box_alpha },
        );
        let bg_color = moon_color(
            if checked { p.blue } else { p.shell_high },
            if checked { 0.22 * box_alpha } else { 0.95 * box_alpha },
        );
        let label_color = if self.disabled {
            moon_color(p.text_muted, label_alpha)
        } else {
            moon_color(p.text_soft, label_alpha)
        };

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
                .items_center()
                .line_height(metrics.line_height)
                .text_size(metrics.font_size)
                .text_color(label_color)
                .rounded(px(4.))
                .when(!self.disabled, |this| {
                    this.hover(|this| this.bg(moon_color(0xFFFFFF, 0.025)))
                        .active(|this| this.bg(moon_color(0xFFFFFF, 0.015)))
                })
                .focus_ring(is_focused, px(2.), window, cx)
                .refine_style(&self.style)
                .child(
                    div()
                        .relative()
                        .size(metrics.box_size)
                        .flex_shrink_0()
                        .border_1()
                        .border_color(border_color)
                        .rounded(metrics.radius)
                        .bg(bg_color)
                        .child(checkbox_check_icon(
                            self.id,
                            self.size,
                            checked,
                            self.disabled,
                            window,
                            cx,
                        )),
                )
                .when(self.label.is_some() || !self.children.is_empty(), |this| {
                    this.child(
                        v_flex()
                            .flex_1()
                            .overflow_hidden()
                            .line_height(relative(1.2))
                            .gap_1()
                            .map(|this| {
                                if let Some(label) = self.label {
                                    this.child(
                                        div()
                                            .size_full()
                                            .text_color(label_color)
                                            .line_height(relative(1.))
                                            .child(label),
                                    )
                                } else {
                                    this
                                }
                            })
                            .children(self.children),
                    )
                })
                .on_mouse_down(gpui::MouseButton::Left, |_, window, _| {
                    // Avoid focus on mouse down.
                    window.prevent_default();
                })
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

    #[test]
    fn test_moon_checkbox_metrics_match_terminal_palette() {
        let compact = MoonCheckboxMetrics::for_size(Size::XSmall);
        assert_eq!(compact.box_size, px(12.));
        assert_eq!(compact.font_size, px(9.5));
        assert_eq!(compact.line_height, px(12.));
        assert_eq!(compact.gap, px(6.));
        assert_eq!(compact.radius, px(3.));

        let normal = MoonCheckboxMetrics::for_size(Size::Medium);
        assert_eq!(normal.box_size, px(14.));
        assert_eq!(normal.font_size, px(10.5));
        assert_eq!(normal.line_height, px(13.));
        assert_eq!(normal.gap, px(7.));
        assert_eq!(normal.radius, px(3.5));
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
}
