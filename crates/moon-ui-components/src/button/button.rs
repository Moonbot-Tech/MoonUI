use std::rc::Rc;

use crate::{
    ActiveTheme, Disableable, FocusableExt as _, Icon, IconName, Selectable, Sizable, Size,
    StyledExt,
    button::ButtonIcon,
    h_flex,
    moon::{MoonPalette, MoonTheme, rgba_from},
    tooltip::{ManagedTooltipExt as _, Tooltip},
};
use gpui::{
    AnyElement, App, ClickEvent, Corners, Div, Edges, ElementId, Hsla, InteractiveElement,
    Interactivity, IntoElement, MouseButton, ParentElement, Pixels, RenderOnce, SharedString,
    Stateful, StatefulInteractiveElement as _, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder as _, px, relative,
};

#[derive(Default, Clone, Copy)]
pub enum ButtonRounded {
    None,
    Small,
    #[default]
    Medium,
    Large,
    Size(Pixels),
}

impl From<Pixels> for ButtonRounded {
    fn from(px: Pixels) -> Self {
        ButtonRounded::Size(px)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ButtonCustomVariant {
    color: Hsla,
    foreground: Hsla,
    shadow: bool,
    hover: Hsla,
    active: Hsla,
}

pub trait ButtonVariants: Sized {
    fn with_variant(self, variant: ButtonVariant) -> Self;

    /// With the primary style for the Button.
    fn primary(self) -> Self {
        self.with_variant(ButtonVariant::Primary)
    }

    /// With the secondary style for the Button.
    fn secondary(self) -> Self {
        self.with_variant(ButtonVariant::Secondary)
    }

    /// With the Moon panel style for the Button.
    fn panel(self) -> Self {
        self.with_variant(ButtonVariant::Panel)
    }

    /// With the Moon soft style for the Button.
    fn soft(self) -> Self {
        self.with_variant(ButtonVariant::Soft)
    }

    /// With the Moon blue accent style for the Button.
    fn blue(self) -> Self {
        self.with_variant(ButtonVariant::Blue)
    }

    /// With the Moon amber accent style for the Button.
    fn amber(self) -> Self {
        self.with_variant(ButtonVariant::Amber)
    }

    /// With the Moon green accent style for the Button.
    fn green(self) -> Self {
        self.with_variant(ButtonVariant::Green)
    }

    /// With the Moon red accent style for the Button.
    fn red(self) -> Self {
        self.with_variant(ButtonVariant::Red)
    }

    /// With the danger style for the Button.
    fn danger(self) -> Self {
        self.with_variant(ButtonVariant::Danger)
    }

    /// With the warning style for the Button.
    fn warning(self) -> Self {
        self.with_variant(ButtonVariant::Warning)
    }

    /// With the success style for the Button.
    fn success(self) -> Self {
        self.with_variant(ButtonVariant::Success)
    }

    /// With the info style for the Button.
    fn info(self) -> Self {
        self.with_variant(ButtonVariant::Info)
    }

    /// With the ghost style for the Button.
    fn ghost(self) -> Self {
        self.with_variant(ButtonVariant::Ghost)
    }

    /// With the Moon amber outline style for the Button.
    fn outline_amber(self) -> Self {
        self.with_variant(ButtonVariant::OutlineAmber)
    }

    /// With the Moon red outline style for the Button.
    fn outline_red(self) -> Self {
        self.with_variant(ButtonVariant::OutlineRed)
    }

    /// With the Moon bare style for chrome/icon buttons.
    fn bare(self) -> Self {
        self.with_variant(ButtonVariant::Bare)
    }

    /// With the link style for the Button.
    fn link(self) -> Self {
        self.with_variant(ButtonVariant::Link)
    }

    /// With the text style for the Button, it will no padding look like a normal text.
    fn text(self) -> Self {
        self.with_variant(ButtonVariant::Text)
    }

    /// With the custom style for the Button.
    fn custom(self, style: ButtonCustomVariant) -> Self {
        self.with_variant(ButtonVariant::Custom(style))
    }
}

impl ButtonCustomVariant {
    pub fn new(cx: &App) -> Self {
        Self {
            color: cx.theme().transparent,
            foreground: cx.theme().foreground,
            hover: cx.theme().transparent,
            active: cx.theme().transparent,
            shadow: false,
        }
    }

    /// Set background color, default is transparent.
    pub fn color(mut self, color: Hsla) -> Self {
        self.color = color;
        self
    }

    /// Set foreground color, default is theme foreground.
    pub fn foreground(mut self, color: Hsla) -> Self {
        self.foreground = color;
        self
    }

    /// Set hover background color, default is transparent.
    pub fn hover(mut self, color: Hsla) -> Self {
        self.hover = color;
        self
    }

    /// Set active background color, default is transparent.
    pub fn active(mut self, color: Hsla) -> Self {
        self.active = color;
        self
    }

    /// Set shadow, default is false.
    pub fn shadow(mut self, shadow: bool) -> Self {
        self.shadow = shadow;
        self
    }
}

/// The variant of the Button.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum ButtonVariant {
    #[default]
    Default,
    Primary,
    Secondary,
    Panel,
    Soft,
    Blue,
    Amber,
    Green,
    Red,
    Danger,
    OutlineAmber,
    OutlineRed,
    Info,
    Success,
    Warning,
    Ghost,
    Link,
    Text,
    Bare,
    Custom(ButtonCustomVariant),
}

impl ButtonVariant {
    #[inline]
    pub fn is_link(&self) -> bool {
        matches!(self, Self::Link)
    }

    #[inline]
    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text)
    }

    #[inline]
    pub fn is_ghost(&self) -> bool {
        matches!(self, Self::Ghost)
    }

    #[inline]
    fn no_padding(&self) -> bool {
        self.is_link() || self.is_text()
    }
}

#[derive(Clone, Copy)]
struct MoonButtonMetrics {
    height: Pixels,
    radius: Pixels,
    font_size: Pixels,
    line_height: Pixels,
    gap: Pixels,
    pad_x: Pixels,
}

impl MoonButtonMetrics {
    fn for_size(size: Size, cx: &App) -> Self {
        Self::base_for_size(size).scaled(cx)
    }

    fn base_for_size(size: Size) -> Self {
        match size {
            Size::XSmall => Self {
                height: px(18.),
                radius: px(4.),
                font_size: px(9.),
                line_height: px(12.),
                gap: px(4.),
                pad_x: px(7.),
            },
            Size::Small => Self {
                height: px(26.),
                radius: px(4.),
                font_size: px(10.5),
                line_height: px(14.),
                gap: px(6.),
                pad_x: px(0.),
            },
            Size::Medium => Self {
                height: px(28.),
                radius: px(4.),
                font_size: px(11.),
                line_height: px(14.),
                gap: px(6.),
                pad_x: px(0.),
            },
            Size::Large => Self {
                height: px(30.),
                radius: px(15.),
                font_size: px(11.5),
                line_height: px(14.),
                gap: px(6.),
                pad_x: px(0.),
            },
            Size::Size(height) => Self {
                height,
                radius: px(4.),
                font_size: height * 0.4,
                line_height: height * 0.55,
                gap: px(6.),
                pad_x: px(0.),
            },
        }
    }

    fn scaled(self, cx: &App) -> Self {
        let tokens = MoonTheme::active_tokens(cx);
        let base_height = self.height.as_f32();
        let base_line_height = self.line_height.as_f32();
        let base_pad_y = ((base_height - base_line_height) * 0.5).max(0.0);
        let line_height = tokens.line_height(base_line_height);
        Self {
            height: px(tokens
                .ui(base_height)
                .max(line_height + tokens.ui(base_pad_y) * 2.0)),
            radius: px(tokens.ui(self.radius.as_f32())),
            font_size: px(tokens.font(self.font_size.as_f32())),
            line_height: px(line_height),
            gap: px(tokens.ui(self.gap.as_f32())),
            pad_x: px(tokens.ui(self.pad_x.as_f32())),
        }
    }
}

/// A Button element.
#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    base: Stateful<Div>,
    style: StyleRefinement,
    icon: Option<ButtonIcon>,
    label: Option<SharedString>,
    children: Vec<AnyElement>,
    disabled: bool,
    pub(crate) selected: bool,
    variant: ButtonVariant,
    rounded: ButtonRounded,
    outline: bool,
    border_corners: Corners<bool>,
    border_edges: Edges<bool>,
    dropdown_caret: bool,
    size: Size,
    compact: bool,
    tooltip: Option<(
        SharedString,
        Option<(Rc<Box<dyn gpui::Action>>, Option<SharedString>)>,
    )>,
    tooltip_builder: Option<Rc<dyn Fn(&mut Window, &mut App) -> gpui::AnyView>>,
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
    on_hover: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
    loading: bool,
    loading_icon: Option<Icon>,

    tab_index: isize,
    tab_stop: bool,
}

impl From<Button> for AnyElement {
    fn from(button: Button) -> Self {
        button.into_any_element()
    }
}

impl Button {
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();

        Self {
            id: id.clone(),
            // ID must be set after div is created;
            // `dropdown_menu` uses this id to create the popup menu.
            base: div().flex_shrink_0().id(id),
            style: StyleRefinement::default(),
            icon: None,
            label: None,
            disabled: false,
            selected: false,
            variant: ButtonVariant::default(),
            rounded: ButtonRounded::Medium,
            border_corners: Corners {
                top_left: true,
                top_right: true,
                bottom_right: true,
                bottom_left: true,
            },
            border_edges: Edges::all(true),
            size: Size::Medium,
            tooltip: None,
            tooltip_builder: None,
            on_click: None,
            on_hover: None,
            loading: false,
            compact: false,
            outline: false,
            children: Vec::new(),
            loading_icon: None,
            dropdown_caret: false,
            tab_index: 0,
            tab_stop: true,
        }
    }

    /// Set the outline style of the Button.
    pub fn outline(mut self) -> Self {
        self.outline = true;
        self
    }

    /// Set the border radius of the Button.
    pub fn rounded(mut self, rounded: impl Into<ButtonRounded>) -> Self {
        self.rounded = rounded.into();
        self
    }

    /// Set the border corners side of the Button.
    pub(crate) fn border_corners(mut self, corners: impl Into<Corners<bool>>) -> Self {
        self.border_corners = corners.into();
        self
    }

    /// Set the border edges of the Button.
    pub(crate) fn border_edges(mut self, edges: impl Into<Edges<bool>>) -> Self {
        self.border_edges = edges.into();
        self
    }

    /// Set label to the Button, if no label is set, the button will be in Icon Button mode.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the icon of the button, if the Button have no label, the button well in Icon Button mode.
    pub fn icon(mut self, icon: impl Into<ButtonIcon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set the tooltip of the button.
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some((tooltip.into(), None));
        self
    }

    /// Set the tooltip of the button with action to show keybinding.
    pub fn tooltip_with_action(
        mut self,
        tooltip: impl Into<SharedString>,
        action: &dyn gpui::Action,
        context: Option<&str>,
    ) -> Self {
        self.tooltip = Some((
            tooltip.into(),
            Some((
                Rc::new(action.boxed_clone()),
                context.map(|c| c.to_string().into()),
            )),
        ));
        self
    }

    /// Set true to show the loading indicator.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Set the button to compact mode, then padding will be reduced.
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    /// Add click handler.
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Add hover handler, the bool parameter indicates whether the mouse is hovering.
    pub fn on_hover(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_hover = Some(Rc::new(handler));
        self
    }

    /// Set the loading icon of the button, it will be used when loading is true.
    ///
    /// Default is a spinner icon.
    pub fn loading_icon(mut self, icon: impl Into<Icon>) -> Self {
        self.loading_icon = Some(icon.into());
        self
    }

    /// Set the tab index of the button, it will be used to focus the button by tab key.
    ///
    /// Default is 0.
    pub fn tab_index(mut self, tab_index: isize) -> Self {
        self.tab_index = tab_index;
        self
    }

    /// Set the tab stop of the button, if true, the button will be focusable by tab key.
    ///
    /// Default is true.
    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        self
    }

    /// Set to show a dropdown caret icon at the end of the button.
    pub fn dropdown_caret(mut self, dropdown_caret: bool) -> Self {
        self.dropdown_caret = dropdown_caret;
        self
    }

    #[inline]
    fn clickable(&self) -> bool {
        !(self.disabled || self.loading) && self.on_click.is_some()
    }

    #[inline]
    fn hoverable(&self) -> bool {
        !(self.disabled || self.loading) && self.on_hover.is_some()
    }
}

impl Disableable for Button {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Selectable for Button {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl Sizable for Button {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl ButtonVariants for Button {
    fn with_variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl Styled for Button {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Button {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements)
    }
}

impl InteractiveElement for Button {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl RenderOnce for Button {
    /// Render the configured button with theme-resolved resting and pointer-interaction states.
    ///
    /// The window supplies focus state, the app supplies theme tokens, and the returned element
    /// owns all mouse, focus, tooltip, and click wiring for this one-shot button.
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let style: ButtonVariant = self.variant;
        let clickable = self.clickable();
        let is_disabled = self.disabled;
        let hoverable = self.hoverable();
        let pointer_feedback = pointer_feedback_enabled(self.disabled, self.loading);
        let metrics = MoonButtonMetrics::for_size(self.size, cx);
        let normal_style = style.normal(self.outline, cx);
        let icon_size = Size::Size(px((metrics.font_size.as_f32() + 1.0).clamp(10.0, 14.0)));

        let focus_handle = window
            .use_keyed_state(self.id.clone(), cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        let is_focused = focus_handle.is_focused(window);

        let rounding = match self.rounded {
            ButtonRounded::Small => px(2.),
            ButtonRounded::Medium => metrics.radius,
            ButtonRounded::Large => metrics.height * 0.5,
            ButtonRounded::Size(px) => px,
            ButtonRounded::None => Pixels::ZERO,
        };

        self.base
            .when(!self.disabled, |this| {
                this.track_focus(
                    &focus_handle
                        .tab_index(self.tab_index)
                        .tab_stop(self.tab_stop),
                )
            })
            .cursor_default()
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .cursor_default()
            .when(self.variant.is_link(), |this| this.cursor_pointer())
            .when(cx.theme().shadow && normal_style.shadow, |this| {
                this.shadow_xs()
            })
            .when(!style.no_padding(), |this| {
                if self.label.is_none() && self.children.is_empty() {
                    this.size(metrics.height)
                } else {
                    this.h(metrics.height)
                        .px(metrics.pad_x)
                        .when(self.compact, |this| this.min_w(metrics.height))
                }
            })
            .when(self.border_corners.top_left, |this| {
                this.rounded_tl(rounding)
            })
            .when(self.border_corners.top_right, |this| {
                this.rounded_tr(rounding)
            })
            .when(self.border_corners.bottom_left, |this| {
                this.rounded_bl(rounding)
            })
            .when(self.border_corners.bottom_right, |this| {
                this.rounded_br(rounding)
            })
            .when(!self.variant.is_link() && !self.variant.is_text(), |this| {
                this.when(self.border_edges.left, |this| this.border_l_1())
                    .when(self.border_edges.right, |this| this.border_r_1())
                    .when(self.border_edges.top, |this| this.border_t_1())
                    .when(self.border_edges.bottom, |this| this.border_b_1())
            })
            .text_color(normal_style.fg)
            .when(self.selected, |this| {
                let selected_style = style.selected(self.outline, cx);
                this.bg(selected_style.bg)
                    .border_color(selected_style.border)
                    .text_color(selected_style.fg)
                    .when(pointer_feedback, |this| {
                        this.hover(|this| {
                            let hover_style = style.selected_hovered(self.outline, cx);
                            this.bg(hover_style.bg)
                                .border_color(hover_style.border)
                                .text_color(hover_style.fg)
                        })
                        .active(|this| {
                            let active_style = style.selected_active(self.outline, cx);
                            this.bg(active_style.bg)
                                .border_color(active_style.border)
                                .text_color(active_style.fg)
                        })
                    })
            })
            .when(!self.disabled && !self.selected, |this| {
                this.border_color(normal_style.border)
                    .bg(normal_style.bg)
                    .when(normal_style.underline, |this| this.text_decoration_1())
                    .when(pointer_feedback, |this| {
                        this.hover(|this| {
                            let hover_style = style.hovered(self.outline, cx);
                            this.bg(hover_style.bg)
                                .border_color(hover_style.border)
                                .text_color(hover_style.fg)
                        })
                        .active(|this| {
                            let active_style = style.active(self.outline, cx);
                            this.bg(active_style.bg)
                                .border_color(active_style.border)
                                .text_color(active_style.fg)
                        })
                    })
            })
            .when(self.disabled, |this| {
                let disabled_style = style.disabled(self.outline, cx);
                this.bg(disabled_style.bg)
                    .text_color(disabled_style.fg)
                    .border_color(disabled_style.border)
                    .shadow_none()
            })
            .refine_style(&self.style)
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                // Stop handle any click event when disabled.
                // To avoid handle dropdown menu open when button is disabled.
                if is_disabled {
                    cx.stop_propagation();
                    return;
                }

                // Avoid focus on mouse down.
                window.prevent_default();

                // Pressing a button must not start the window-level text selection.
                crate::global_state::GlobalState::suppress_text_selection(cx);
            })
            .when_some(self.on_click, |this, on_click| {
                this.on_click(move |event, window, cx| {
                    // Stop handle any click event when disabled.
                    // To avoid handle dropdown menu open when button is disabled.
                    if !clickable {
                        cx.stop_propagation();
                        return;
                    }

                    on_click(event, window, cx);
                })
            })
            .when_some(self.on_hover.filter(|_| hoverable), |this, on_hover| {
                this.on_hover(move |hovered, window, cx| {
                    on_hover(hovered, window, cx);
                })
            })
            .child({
                h_flex()
                    .id("label")
                    .size_full()
                    .items_center()
                    .justify_center()
                    .text_size(metrics.font_size)
                    .line_height(metrics.line_height)
                    .gap(metrics.gap)
                    .when_some(self.icon, |this, icon| {
                        this.child(
                            icon.loading_icon(self.loading_icon)
                                .loading(self.loading)
                                .with_size(icon_size),
                        )
                    })
                    .when_some(self.label, |this, label| {
                        this.child(div().flex_none().line_height(relative(1.)).child(label))
                    })
                    .children(self.children)
                    .when(self.dropdown_caret, |this| {
                        this.justify_between().child(
                            Icon::new(IconName::ChevronDown).xsmall().text_color(
                                match self.disabled {
                                    true => normal_style.fg.opacity(0.3),
                                    false => normal_style.fg.opacity(0.5),
                                },
                            ),
                        )
                    })
            })
            .when(self.loading && !self.disabled, |this| {
                this.bg(normal_style.bg.opacity(0.8))
                    .border_color(normal_style.border.opacity(0.8))
                    .text_color(normal_style.fg.opacity(0.8))
            })
            .map(|this| {
                if let Some(builder) = self.tooltip_builder {
                    this.managed_tooltip(move |window, cx| builder(window, cx))
                } else if let Some((tooltip, action)) = self.tooltip {
                    this.managed_tooltip(move |window, cx| {
                        Tooltip::new(tooltip.clone())
                            .when_some(action.clone(), |this, (action, context)| {
                                this.action(
                                    action.boxed_clone().as_ref(),
                                    context.as_ref().map(|c| c.as_ref()),
                                )
                            })
                            .build(window, cx)
                    })
                } else {
                    this
                }
            })
            .focus_ring(is_focused, px(0.), window, cx)
    }
}

struct ButtonVariantStyle {
    bg: Hsla,
    border: Hsla,
    fg: Hsla,
    underline: bool,
    shadow: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct MoonButtonStyle {
    pub(crate) bg: u32,
    pub(crate) bg_alpha: f32,
    pub(crate) border: u32,
    pub(crate) border_alpha: f32,
    pub(crate) fg: u32,
    pub(crate) fg_alpha: f32,
    pub(crate) hover_bg_alpha: f32,
    pub(crate) active_bg_alpha: f32,
    pub(crate) hover_border_alpha: f32,
    pub(crate) active_border_alpha: f32,
    pub(crate) shadow: bool,
}

/// Internal visual states used to resolve one button variant.
#[derive(Clone, Copy)]
enum ButtonVisualState {
    Normal,
    Hovered,
    Active,
    Selected,
    SelectedHovered,
    SelectedActive,
    Disabled,
}

impl ButtonVisualState {
    /// Return whether this state belongs to a selected button.
    fn is_selected(self) -> bool {
        matches!(
            self,
            Self::Selected | Self::SelectedHovered | Self::SelectedActive
        )
    }

    /// Return whether this state represents pointer hover.
    fn is_hovered(self) -> bool {
        matches!(self, Self::Hovered | Self::SelectedHovered)
    }

    /// Return whether this state represents a pressed pointer.
    fn is_active(self) -> bool {
        matches!(self, Self::Active | Self::SelectedActive)
    }
}

/// Return whether a button may show pointer interaction feedback.
///
/// Disabled and loading controls deliberately remain visually inert even when a wrapper owns the
/// click handler, so eligibility cannot depend on `Button::on_click` alone.
fn pointer_feedback_enabled(disabled: bool, loading: bool) -> bool {
    !disabled && !loading
}

impl ButtonVariant {
    /// Return whether the variant uses the shared neutral interaction surface.
    fn uses_neutral_interaction_surface(&self) -> bool {
        matches!(
            self,
            Self::Default | Self::Panel | Self::Secondary | Self::Soft | Self::Ghost | Self::Bare
        )
    }

    fn underline(&self, _: &App) -> bool {
        matches!(self, Self::Link)
    }

    pub(crate) fn moon_style(
        &self,
        p: MoonPalette,
        outline: bool,
        selected: bool,
    ) -> Option<MoonButtonStyle> {
        let selected_boost = if selected { 0.08 } else { 0.0 };
        let light = p.is_light();
        let info = if light { p.accent } else { p.blue };
        let success_bg = if light { p.green_btn } else { p.green };
        let success_fg = if light { p.green_text } else { p.green };
        let danger_fg = if light { p.red_text } else { p.red };

        match self {
            Self::Default => Some(MoonButtonStyle {
                bg: if light { p.surface } else { 0x1F2126 },
                bg_alpha: 1.0,
                border: if light { p.border_soft } else { p.border },
                border_alpha: 1.0,
                fg: p.text,
                fg_alpha: 0.86,
                hover_bg_alpha: 1.0,
                active_bg_alpha: 0.82,
                hover_border_alpha: 1.0,
                active_border_alpha: 1.0,
                shadow: false,
            }),
            Self::Panel => Some(MoonButtonStyle {
                bg: if light { p.surface } else { p.shell_high },
                bg_alpha: 1.0,
                border: if light { p.border_soft } else { p.border },
                border_alpha: 1.0,
                fg: p.text,
                fg_alpha: 1.0,
                hover_bg_alpha: 1.0,
                active_bg_alpha: 0.82,
                hover_border_alpha: 1.0,
                active_border_alpha: 1.0,
                shadow: false,
            }),
            Self::Secondary | Self::Soft => Some(MoonButtonStyle {
                bg: if light { p.surface } else { 0xFFFFFF },
                bg_alpha: if light { 1.0 } else { 0.02 },
                border: if light { p.border_soft } else { 0xFFFFFF },
                border_alpha: if light { 1.0 } else { 0.05 },
                fg: p.text_soft,
                fg_alpha: 1.0,
                hover_bg_alpha: if light { 1.0 } else { 0.055 },
                active_bg_alpha: if light { 0.86 } else { 0.035 },
                hover_border_alpha: if light { 1.0 } else { 0.08 },
                active_border_alpha: if light { 1.0 } else { 0.06 },
                shadow: false,
            }),
            Self::Primary | Self::Blue | Self::Info => Some(MoonButtonStyle {
                bg: info,
                bg_alpha: if selected {
                    0.18
                } else if outline {
                    0.0
                } else {
                    0.10
                },
                border: info,
                border_alpha: if selected {
                    0.38
                } else if outline {
                    0.35
                } else {
                    0.22
                },
                fg: info,
                fg_alpha: 1.0,
                hover_bg_alpha: 0.18,
                active_bg_alpha: 0.12,
                hover_border_alpha: 0.42,
                active_border_alpha: 0.30,
                shadow: selected,
            }),
            Self::Warning | Self::Amber | Self::OutlineAmber => Some(MoonButtonStyle {
                bg: if matches!(self, Self::OutlineAmber) || outline {
                    p.shell_high
                } else {
                    p.amber
                },
                bg_alpha: if matches!(self, Self::OutlineAmber) || outline {
                    0.0
                } else if selected {
                    0.18
                } else {
                    0.10
                },
                border: p.amber,
                border_alpha: if selected {
                    0.38
                } else if outline {
                    0.35
                } else {
                    0.22
                },
                fg: if matches!(self, Self::OutlineAmber) {
                    p.text
                } else {
                    p.amber
                },
                fg_alpha: 1.0,
                hover_bg_alpha: if matches!(self, Self::OutlineAmber) || outline {
                    0.04
                } else {
                    0.18
                },
                active_bg_alpha: if matches!(self, Self::OutlineAmber) || outline {
                    0.025
                } else {
                    0.12
                },
                hover_border_alpha: if matches!(self, Self::OutlineAmber) || outline {
                    0.48
                } else {
                    0.42
                },
                active_border_alpha: if matches!(self, Self::OutlineAmber) || outline {
                    0.40
                } else {
                    0.30
                },
                shadow: selected,
            }),
            Self::Success | Self::Green => Some(MoonButtonStyle {
                bg: success_bg,
                bg_alpha: if outline {
                    0.0
                } else if light && selected {
                    1.0
                } else {
                    0.14 + selected_boost
                },
                border: if light && outline {
                    p.green
                } else {
                    success_bg
                },
                border_alpha: if outline { 0.35 } else { 0.30 },
                fg: if light && selected && !outline {
                    p.on_accent
                } else {
                    success_fg
                },
                fg_alpha: 1.0,
                hover_bg_alpha: if light && selected { 0.92 } else { 0.22 },
                active_bg_alpha: if light && selected { 0.84 } else { 0.14 },
                hover_border_alpha: 0.44,
                active_border_alpha: 0.34,
                shadow: false,
            }),
            Self::Danger | Self::Red | Self::OutlineRed => Some(MoonButtonStyle {
                bg: if matches!(self, Self::OutlineRed) || outline {
                    if light { p.surface } else { p.shell }
                } else {
                    p.red
                },
                bg_alpha: if matches!(self, Self::OutlineRed) || outline {
                    0.0
                } else if matches!(self, Self::Danger) {
                    0.14
                } else {
                    0.10
                },
                border: if light && (matches!(self, Self::OutlineRed) || outline) {
                    p.red_soft_bd
                } else {
                    p.red
                },
                border_alpha: if outline { 0.40 } else { 0.38 },
                fg: danger_fg,
                fg_alpha: 1.0,
                hover_bg_alpha: if matches!(self, Self::OutlineRed) || outline {
                    0.08
                } else {
                    0.22
                },
                active_bg_alpha: if matches!(self, Self::OutlineRed) || outline {
                    0.04
                } else {
                    0.14
                },
                hover_border_alpha: if matches!(self, Self::OutlineRed) || outline {
                    0.52
                } else {
                    0.48
                },
                active_border_alpha: if matches!(self, Self::OutlineRed) || outline {
                    0.42
                } else {
                    0.36
                },
                shadow: matches!(self, Self::Danger),
            }),
            Self::Ghost => Some(MoonButtonStyle {
                bg: p.shell_high,
                bg_alpha: 0.0,
                border: p.border,
                border_alpha: 0.0,
                fg: p.text_muted,
                fg_alpha: 0.78,
                hover_bg_alpha: 0.35,
                active_bg_alpha: 0.18,
                hover_border_alpha: 0.0,
                active_border_alpha: 0.0,
                shadow: false,
            }),
            Self::Bare | Self::Text | Self::Link => Some(MoonButtonStyle {
                bg: p.shell_high,
                bg_alpha: 0.0,
                border: p.border,
                border_alpha: 0.0,
                fg: p.text,
                fg_alpha: 0.86,
                hover_bg_alpha: 0.0,
                active_bg_alpha: 0.0,
                hover_border_alpha: 0.0,
                active_border_alpha: 0.0,
                shadow: false,
            }),
            Self::Custom(_) => None,
        }
    }

    /// Resolve a built-in Moon variant against an explicit palette.
    ///
    /// Keeping this palette-driven makes the dark and light interaction states independently
    /// testable instead of relying on whichever global theme happens to be active.
    fn resolve_moon(
        &self,
        p: MoonPalette,
        outline: bool,
        state: ButtonVisualState,
    ) -> ButtonVariantStyle {
        if matches!(state, ButtonVisualState::Disabled) {
            return ButtonVariantStyle {
                bg: rgba_from(p.panel, 0.32),
                border: rgba_from(p.border, 0.42),
                fg: rgba_from(p.text_muted, 0.54),
                underline: matches!(self, Self::Link),
                shadow: false,
            };
        }

        let selected = state.is_selected();

        if matches!(self, Self::Default) && selected {
            let bg_alpha = if state.is_hovered() {
                p.accent_tint_a + 0.06
            } else if state.is_active() {
                p.accent_tint_a + 0.03
            } else {
                p.accent_tint_a
            };
            return ButtonVariantStyle {
                bg: rgba_from(p.accent, bg_alpha.min(1.0)),
                border: rgba_from(p.accent, 1.0),
                fg: rgba_from(p.selected_fg(), 1.0),
                underline: false,
                shadow: false,
            };
        }

        let style = self
            .moon_style(p, outline, selected)
            .unwrap_or_else(|| unreachable!("custom variants are resolved above"));

        let (mut bg, mut border, mut bg_alpha, mut border_alpha) = match state {
            ButtonVisualState::Normal | ButtonVisualState::Selected => {
                (style.bg, style.border, style.bg_alpha, style.border_alpha)
            }
            ButtonVisualState::Hovered | ButtonVisualState::SelectedHovered => (
                style.bg,
                style.border,
                style.hover_bg_alpha,
                style.hover_border_alpha,
            ),
            ButtonVisualState::Active | ButtonVisualState::SelectedActive => (
                style.bg,
                style.border,
                style.active_bg_alpha,
                style.active_border_alpha,
            ),
            ButtonVisualState::Disabled => unreachable!(),
        };

        if self.uses_neutral_interaction_surface() && (state.is_hovered() || state.is_active()) {
            bg = p.overlay;
            border = p.border_hover;
            if state.is_hovered() {
                bg_alpha = if p.is_light() { 0.07 } else { 0.10 };
            } else {
                bg_alpha = if p.is_light() { 0.11 } else { 0.06 };
            }
            border_alpha = 1.0;
        }

        ButtonVariantStyle {
            bg: rgba_from(bg, bg_alpha),
            border: rgba_from(border, border_alpha),
            fg: rgba_from(style.fg, style.fg_alpha),
            underline: matches!(self, Self::Link),
            shadow: style.shadow,
        }
    }

    /// Resolve custom colors or delegate a built-in variant to the active Moon palette.
    fn resolve(&self, outline: bool, state: ButtonVisualState, cx: &mut App) -> ButtonVariantStyle {
        if let Self::Custom(colors) = self {
            let bg = match state {
                ButtonVisualState::Normal => colors.color,
                ButtonVisualState::Hovered => colors.hover,
                ButtonVisualState::Active
                | ButtonVisualState::Selected
                | ButtonVisualState::SelectedHovered
                | ButtonVisualState::SelectedActive => colors.active,
                ButtonVisualState::Disabled => colors.color.opacity(0.15),
            };
            let fg = if matches!(state, ButtonVisualState::Disabled) {
                cx.theme().muted_foreground.opacity(0.5)
            } else {
                colors.foreground
            };
            return ButtonVariantStyle {
                bg,
                border: if matches!(state, ButtonVisualState::SelectedHovered) {
                    if outline {
                        colors.hover.opacity(0.4)
                    } else {
                        colors.hover
                    }
                } else if outline {
                    colors.color.opacity(0.4)
                } else {
                    colors.color
                },
                fg,
                underline: self.underline(cx),
                shadow: colors.shadow,
            };
        }

        self.resolve_moon(MoonPalette::active(cx), outline, state)
    }

    /// Resolve the resting, unselected style.
    fn normal(&self, outline: bool, cx: &mut App) -> ButtonVariantStyle {
        self.resolve(outline, ButtonVisualState::Normal, cx)
    }

    /// Resolve the hovered, unselected style.
    fn hovered(&self, outline: bool, cx: &mut App) -> ButtonVariantStyle {
        self.resolve(outline, ButtonVisualState::Hovered, cx)
    }

    /// Resolve the pressed, unselected style.
    fn active(&self, outline: bool, cx: &mut App) -> ButtonVariantStyle {
        self.resolve(outline, ButtonVisualState::Active, cx)
    }

    /// Resolve the resting, selected style.
    fn selected(&self, outline: bool, cx: &mut App) -> ButtonVariantStyle {
        self.resolve(outline, ButtonVisualState::Selected, cx)
    }

    /// Resolve the hovered, selected style without losing selection feedback.
    fn selected_hovered(&self, outline: bool, cx: &mut App) -> ButtonVariantStyle {
        self.resolve(outline, ButtonVisualState::SelectedHovered, cx)
    }

    /// Resolve the pressed, selected style without losing selection feedback.
    fn selected_active(&self, outline: bool, cx: &mut App) -> ButtonVariantStyle {
        self.resolve(outline, ButtonVisualState::SelectedActive, cx)
    }

    /// Resolve the disabled style.
    fn disabled(&self, outline: bool, cx: &mut App) -> ButtonVariantStyle {
        self.resolve(outline, ButtonVisualState::Disabled, cx)
    }
}

#[cfg(test)]
mod tests;
