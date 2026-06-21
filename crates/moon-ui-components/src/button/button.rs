use std::rc::Rc;

use crate::{
    ActiveTheme, Disableable, FocusableExt as _, Icon, IconName, Selectable, Sizable, Size,
    StyledExt,
    button::ButtonIcon,
    h_flex,
    moon::MoonTheme,
    moon_skin::{MoonSkinPalette, moon_color},
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
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let style: ButtonVariant = self.variant;
        let clickable = self.clickable();
        let is_disabled = self.disabled;
        let hoverable = self.hoverable();
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
            })
            .when(!self.disabled && !self.selected, |this| {
                this.border_color(normal_style.border)
                    .bg(normal_style.bg)
                    .when(normal_style.underline, |this| this.text_decoration_1())
                    .hover(|this| {
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
struct MoonButtonStyle {
    bg: u32,
    bg_alpha: f32,
    border: u32,
    border_alpha: f32,
    fg: u32,
    fg_alpha: f32,
    hover_bg_alpha: f32,
    active_bg_alpha: f32,
    hover_border_alpha: f32,
    active_border_alpha: f32,
    shadow: bool,
}

#[derive(Clone, Copy)]
enum ButtonVisualState {
    Normal,
    Hovered,
    Active,
    Selected,
    Disabled,
}

impl ButtonVariant {
    fn underline(&self, _: &App) -> bool {
        matches!(self, Self::Link)
    }

    fn moon_style(&self, outline: bool, selected: bool) -> Option<MoonButtonStyle> {
        let p = MoonSkinPalette::TERMINAL;
        let selected_boost = if selected { 0.08 } else { 0.0 };

        match self {
            Self::Default => Some(MoonButtonStyle {
                bg: 0x1F2126,
                bg_alpha: 1.0,
                border: p.border,
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
                bg: p.shell_high,
                bg_alpha: 1.0,
                border: p.border,
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
                bg: 0xFFFFFF,
                bg_alpha: 0.02,
                border: 0xFFFFFF,
                border_alpha: 0.05,
                fg: p.text_soft,
                fg_alpha: 1.0,
                hover_bg_alpha: 0.055,
                active_bg_alpha: 0.035,
                hover_border_alpha: 0.08,
                active_border_alpha: 0.06,
                shadow: false,
            }),
            Self::Primary | Self::Blue | Self::Info => Some(MoonButtonStyle {
                bg: p.blue,
                bg_alpha: if selected {
                    0.18
                } else if outline {
                    0.0
                } else {
                    0.10
                },
                border: p.blue,
                border_alpha: if selected {
                    0.38
                } else if outline {
                    0.35
                } else {
                    0.22
                },
                fg: p.blue,
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
                bg: p.green,
                bg_alpha: if outline { 0.0 } else { 0.14 + selected_boost },
                border: p.green,
                border_alpha: if outline { 0.35 } else { 0.30 },
                fg: p.green,
                fg_alpha: 1.0,
                hover_bg_alpha: 0.22,
                active_bg_alpha: 0.14,
                hover_border_alpha: 0.44,
                active_border_alpha: 0.34,
                shadow: false,
            }),
            Self::Danger | Self::Red | Self::OutlineRed => Some(MoonButtonStyle {
                bg: if matches!(self, Self::OutlineRed) || outline {
                    p.shell
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
                border: p.red,
                border_alpha: if outline { 0.40 } else { 0.38 },
                fg: p.red,
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

    fn resolve(&self, outline: bool, state: ButtonVisualState, cx: &mut App) -> ButtonVariantStyle {
        if let Self::Custom(colors) = self {
            let bg = match state {
                ButtonVisualState::Normal => colors.color,
                ButtonVisualState::Hovered => colors.hover,
                ButtonVisualState::Active | ButtonVisualState::Selected => colors.active,
                ButtonVisualState::Disabled => colors.color.opacity(0.15),
            };
            let fg = if matches!(state, ButtonVisualState::Disabled) {
                cx.theme().muted_foreground.opacity(0.5)
            } else {
                colors.foreground
            };
            return ButtonVariantStyle {
                bg,
                border: if outline {
                    colors.color.opacity(0.4)
                } else {
                    colors.color
                },
                fg,
                underline: self.underline(cx),
                shadow: colors.shadow,
            };
        }

        if matches!(state, ButtonVisualState::Disabled) {
            let p = MoonSkinPalette::TERMINAL;
            return ButtonVariantStyle {
                bg: moon_color(p.panel, 0.32),
                border: moon_color(p.border, 0.42),
                fg: moon_color(p.text_muted, 0.54),
                underline: self.underline(cx),
                shadow: false,
            };
        }

        let selected = matches!(state, ButtonVisualState::Selected);
        let style = self
            .moon_style(outline, selected)
            .unwrap_or_else(|| unreachable!("custom variants are resolved above"));

        let (bg_alpha, border_alpha) = match state {
            ButtonVisualState::Normal | ButtonVisualState::Selected => {
                (style.bg_alpha, style.border_alpha)
            }
            ButtonVisualState::Hovered => (style.hover_bg_alpha, style.hover_border_alpha),
            ButtonVisualState::Active => (style.active_bg_alpha, style.active_border_alpha),
            ButtonVisualState::Disabled => unreachable!(),
        };

        ButtonVariantStyle {
            bg: moon_color(style.bg, bg_alpha),
            border: moon_color(style.border, border_alpha),
            fg: moon_color(style.fg, style.fg_alpha),
            underline: self.underline(cx),
            shadow: style.shadow,
        }
    }

    fn normal(&self, outline: bool, cx: &mut App) -> ButtonVariantStyle {
        self.resolve(outline, ButtonVisualState::Normal, cx)
    }

    fn hovered(&self, outline: bool, cx: &mut App) -> ButtonVariantStyle {
        self.resolve(outline, ButtonVisualState::Hovered, cx)
    }

    fn active(&self, outline: bool, cx: &mut App) -> ButtonVariantStyle {
        self.resolve(outline, ButtonVisualState::Active, cx)
    }

    fn selected(&self, outline: bool, cx: &mut App) -> ButtonVariantStyle {
        self.resolve(outline, ButtonVisualState::Selected, cx)
    }

    fn disabled(&self, outline: bool, cx: &mut App) -> ButtonVariantStyle {
        self.resolve(outline, ButtonVisualState::Disabled, cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn test_button_builder(_cx: &mut gpui::TestAppContext) {
        let button = Button::new("complex-button")
            .label("Save Changes")
            .primary()
            .outline()
            .large()
            .tooltip("Click to save")
            .compact()
            .loading(false)
            .disabled(false)
            .selected(false)
            .tab_index(1)
            .tab_stop(true)
            .dropdown_caret(false)
            .rounded(ButtonRounded::Medium)
            .on_click(|_, _, _| {});

        assert_eq!(button.label, Some("Save Changes".into()));
        assert_eq!(button.variant, ButtonVariant::Primary);
        assert!(button.outline);
        assert_eq!(button.size, Size::Large);
        assert!(button.tooltip.is_some());
        assert!(button.compact);
        assert!(!button.loading);
        assert!(!button.disabled);
        assert!(!button.selected);
        assert_eq!(button.tab_index, 1);
        assert!(button.tab_stop);
        assert!(!button.dropdown_caret);
        assert!(matches!(button.rounded, ButtonRounded::Medium));
    }

    #[gpui::test]
    fn test_button_clickable_logic(_cx: &mut gpui::TestAppContext) {
        // Button with click handler should be clickable
        let clickable = Button::new("test").on_click(|_, _, _| {});
        assert!(clickable.clickable());

        // Disabled button should not be clickable
        let disabled = Button::new("test").disabled(true).on_click(|_, _, _| {});
        assert!(!disabled.clickable());

        // Loading button should not be clickable
        let loading = Button::new("test").loading(true).on_click(|_, _, _| {});
        assert!(!loading.clickable());
    }

    #[gpui::test]
    fn test_button_variant_methods(_cx: &mut gpui::TestAppContext) {
        // Test variant check methods
        assert!(ButtonVariant::Link.is_link());
        assert!(ButtonVariant::Text.is_text());
        assert!(ButtonVariant::Ghost.is_ghost());

        // Test no_padding logic
        assert!(ButtonVariant::Link.no_padding());
        assert!(ButtonVariant::Text.no_padding());
        assert!(!ButtonVariant::Ghost.no_padding());
    }

    #[gpui::test]
    fn test_outline_selected_uses_outline_active_style(cx: &mut gpui::TestAppContext) {
        cx.update(crate::init);
        let window = cx.add_empty_window();
        window.update(|_, cx| {
            let variant = ButtonVariant::Danger;
            let active_style = variant.active(true, cx);
            let selected_style = variant.selected(true, cx);

            assert_eq!(selected_style.bg.a, 0.0);
            assert_eq!(selected_style.border, moon_color(0xFF4A4A, 0.40));
            assert_eq!(selected_style.fg, moon_color(0xFF4A4A, 1.0));
            assert_ne!(selected_style.bg, active_style.bg);
        });
    }

    #[test]
    fn test_moon_button_metrics_match_terminal_palette() {
        let micro = MoonButtonMetrics::base_for_size(Size::XSmall);
        assert_eq!(micro.height, px(18.));
        assert_eq!(micro.radius, px(4.));
        assert_eq!(micro.font_size, px(9.));
        assert_eq!(micro.line_height, px(12.));
        assert_eq!(micro.gap, px(4.));
        assert_eq!(micro.pad_x, px(7.));

        let action = MoonButtonMetrics::base_for_size(Size::Small);
        assert_eq!(action.height, px(26.));
        assert_eq!(action.radius, px(4.));
        assert_eq!(action.font_size, px(10.5));
        assert_eq!(action.line_height, px(14.));
        assert_eq!(action.gap, px(6.));
        assert_eq!(action.pad_x, px(0.));

        let toolbar = MoonButtonMetrics::base_for_size(Size::Medium);
        assert_eq!(toolbar.height, px(28.));
        assert_eq!(toolbar.radius, px(4.));

        let pill = MoonButtonMetrics::base_for_size(Size::Large);
        assert_eq!(pill.height, px(30.));
        assert_eq!(pill.radius, px(15.));
    }

    #[test]
    fn test_moon_button_variant_tokens_match_terminal_palette() {
        let blue = ButtonVariant::Blue.moon_style(false, false).unwrap();
        assert_eq!(blue.bg, MoonSkinPalette::TERMINAL.blue);
        assert_eq!(blue.bg_alpha, 0.10);
        assert_eq!(blue.border_alpha, 0.22);
        assert_eq!(blue.hover_bg_alpha, 0.18);

        let selected_blue = ButtonVariant::Blue.moon_style(false, true).unwrap();
        assert_eq!(selected_blue.bg_alpha, 0.18);
        assert_eq!(selected_blue.border_alpha, 0.38);

        let outline_red = ButtonVariant::OutlineRed.moon_style(false, false).unwrap();
        assert_eq!(outline_red.bg_alpha, 0.0);
        assert_eq!(outline_red.border, MoonSkinPalette::TERMINAL.red);
        assert_eq!(outline_red.fg, MoonSkinPalette::TERMINAL.red);

        let soft = ButtonVariant::Soft.moon_style(false, false).unwrap();
        assert_eq!(soft.bg, 0xFFFFFF);
        assert_eq!(soft.bg_alpha, 0.02);
        assert_eq!(soft.hover_bg_alpha, 0.055);
    }
}
