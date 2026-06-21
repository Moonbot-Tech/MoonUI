use crate::button::{Button, ButtonRounded, ButtonVariant, ButtonVariants};
use crate::{Disableable, Icon, Selectable, Sizable};
use gpui::prelude::FluentBuilder as _;
use gpui::*;

use super::{
    theme::MoonTheme,
    tokens::{MoonRect, rgb_from},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoonButtonVariant {
    Neutral,
    Panel,
    Soft,
    Blue,
    Amber,
    Green,
    Red,
    Danger,
    OutlineAmber,
    OutlineRed,
    Ghost,
    Bare,
}

impl From<MoonButtonVariant> for ButtonVariant {
    fn from(value: MoonButtonVariant) -> Self {
        match value {
            MoonButtonVariant::Neutral => ButtonVariant::Default,
            MoonButtonVariant::Panel => ButtonVariant::Panel,
            MoonButtonVariant::Soft => ButtonVariant::Soft,
            MoonButtonVariant::Blue => ButtonVariant::Blue,
            MoonButtonVariant::Amber => ButtonVariant::Amber,
            MoonButtonVariant::Green => ButtonVariant::Green,
            MoonButtonVariant::Red => ButtonVariant::Red,
            MoonButtonVariant::Danger => ButtonVariant::Danger,
            MoonButtonVariant::OutlineAmber => ButtonVariant::OutlineAmber,
            MoonButtonVariant::OutlineRed => ButtonVariant::OutlineRed,
            MoonButtonVariant::Ghost => ButtonVariant::Ghost,
            MoonButtonVariant::Bare => ButtonVariant::Bare,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MoonButtonSize {
    Micro,
    Toolbar,
    Action,
    Pill,
    Custom {
        height: f32,
        radius: f32,
        font_size: f32,
        line_height: f32,
        gap: f32,
    },
}

#[derive(Clone, Debug)]
pub struct MoonButtonSegment {
    text: SharedString,
    color: Option<u32>,
    alpha: f32,
    font_size: Option<f32>,
    line_height: Option<f32>,
    tracking: Option<f32>,
    weight: f32,
    mono: Option<bool>,
}

impl MoonButtonSegment {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            color: None,
            alpha: 1.0,
            font_size: None,
            line_height: None,
            tracking: None,
            weight: 400.0,
            mono: None,
        }
    }

    pub fn color(mut self, color: u32) -> Self {
        self.color = Some(color);
        self
    }

    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha;
        self
    }

    pub fn font_size(mut self, font_size: f32) -> Self {
        self.font_size = Some(font_size);
        self
    }

    pub fn line_height(mut self, line_height: f32) -> Self {
        self.line_height = Some(line_height);
        self
    }

    pub fn tracking(mut self, tracking: f32) -> Self {
        self.tracking = Some(tracking);
        self
    }

    pub fn weight(mut self, weight: f32) -> Self {
        self.weight = weight;
        self
    }

    pub fn mono(mut self, mono: bool) -> Self {
        self.mono = Some(mono);
        self
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MoonButtonIconSlot {
    path: &'static str,
    size: f32,
    color: Option<u32>,
    alpha: f32,
}

impl MoonButtonIconSlot {
    pub fn new(path: &'static str) -> Self {
        Self {
            path,
            size: 12.0,
            color: None,
            alpha: 1.0,
        }
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn color(mut self, color: u32) -> Self {
        self.color = Some(color);
        self
    }

    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha;
        self
    }

    fn icon(self, cx: &App) -> Icon {
        let tokens = MoonTheme::active_tokens(cx);
        let mut icon = Icon::default()
            .path(self.path)
            .size(px(tokens.ui(self.size)));
        if let Some(color) = self.color {
            icon = icon.text_color(rgba_from_u32(color, self.alpha));
        }
        icon
    }
}

#[derive(IntoElement)]
pub struct MoonButton {
    id: ElementId,
    bounds: Option<MoonRect>,
    width: Option<f32>,
    full_width: bool,
    segments: Vec<MoonButtonSegment>,
    variant: MoonButtonVariant,
    size: MoonButtonSize,
    selected: bool,
    disabled: bool,
    leading_icon: Option<MoonButtonIconSlot>,
    trailing_icon: Option<MoonButtonIconSlot>,
    loading_icon: Option<MoonButtonIconSlot>,
    loading: bool,
    radius: Option<f32>,
    tooltip: Option<SharedString>,
    on_hover: Option<std::rc::Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
    on_click: Option<std::rc::Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
    tab_index: isize,
    tab_stop: bool,
}

impl MoonButton {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            width: None,
            full_width: false,
            segments: Vec::new(),
            variant: MoonButtonVariant::Neutral,
            size: MoonButtonSize::Toolbar,
            selected: false,
            disabled: false,
            leading_icon: None,
            trailing_icon: None,
            loading_icon: None,
            loading: false,
            radius: None,
            tooltip: None,
            on_hover: None,
            on_click: None,
            tab_index: 0,
            tab_stop: true,
        }
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.segments.push(MoonButtonSegment::new(label));
        self
    }

    pub fn xsmall(self) -> Self {
        self.size(MoonButtonSize::Micro)
    }

    pub fn small(self) -> Self {
        self.size(MoonButtonSize::Action)
    }

    pub fn medium(self) -> Self {
        self.size(MoonButtonSize::Toolbar)
    }

    pub fn primary(self) -> Self {
        self.variant(MoonButtonVariant::Blue)
    }

    pub fn success(self) -> Self {
        self.variant(MoonButtonVariant::Green)
    }

    pub fn warning(self) -> Self {
        self.variant(MoonButtonVariant::Amber)
    }

    pub fn danger(self) -> Self {
        self.variant(MoonButtonVariant::Danger)
    }

    pub fn outline(self) -> Self {
        self.variant(MoonButtonVariant::OutlineAmber)
    }

    pub fn ghost(self) -> Self {
        self.variant(MoonButtonVariant::Ghost)
    }

    pub fn segment(mut self, segment: MoonButtonSegment) -> Self {
        self.segments.push(segment);
        self
    }

    pub fn icon(self, path: &'static str) -> Self {
        self.leading_icon(MoonButtonIconSlot::new(path))
    }

    pub fn leading_icon(mut self, icon: MoonButtonIconSlot) -> Self {
        self.leading_icon = Some(icon);
        self
    }

    pub fn trailing_icon(mut self, icon: MoonButtonIconSlot) -> Self {
        self.trailing_icon = Some(icon);
        self
    }

    pub fn loading_icon(self, path: &'static str) -> Self {
        self.loading_icon_slot(MoonButtonIconSlot::new(path))
    }

    pub fn loading_icon_slot(mut self, icon: MoonButtonIconSlot) -> Self {
        self.loading_icon = Some(icon);
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    pub fn rounded(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    pub fn text_segment(mut self, text: impl Into<SharedString>, color: u32, weight: f32) -> Self {
        self.segments
            .push(MoonButtonSegment::new(text).color(color).weight(weight));
        self
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }

    pub fn variant(mut self, variant: MoonButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: MoonButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn mono(self, _mono: bool) -> Self {
        self
    }

    pub fn tab_index(mut self, tab_index: isize) -> Self {
        self.tab_index = tab_index;
        self
    }

    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(std::rc::Rc::new(handler));
        self
    }

    pub fn on_hover(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_hover = Some(std::rc::Rc::new(handler));
        self
    }

    pub fn render(self) -> impl IntoElement {
        self
    }
}

impl RenderOnce for MoonButton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let mut button = Button::new(self.id)
            .with_variant(self.variant.into())
            .with_size(size_for(self.size))
            .selected(self.selected)
            .disabled(self.disabled)
            .loading(self.loading)
            .tab_index(self.tab_index)
            .tab_stop(self.tab_stop);

        if let Some(radius) = self.radius.or_else(|| custom_radius(self.size)) {
            button = button.rounded(ButtonRounded::Size(px(tokens.ui(radius))));
        }
        if let Some(bounds) = self.bounds {
            button = button
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }
        if let Some(width) = self.width {
            button = button.w(px(width));
        }
        if self.full_width {
            button = button.w_full();
        }
        if let Some(icon) = self.leading_icon {
            button = button.icon(icon.icon(cx));
        }
        if let Some(icon) = self.loading_icon {
            button = button.loading_icon(icon.icon(cx));
        }
        if let Some(tooltip) = self.tooltip {
            button = button.tooltip(tooltip);
        }
        if let Some(on_hover) = self.on_hover {
            button = button.on_hover(move |hovered, window, cx| on_hover(hovered, window, cx));
        }
        if let Some(on_click) = self.on_click {
            button = button.on_click(move |event, window, cx| on_click(event, window, cx));
        }

        let (font_size, line_height, gap) = metrics_for(self.size);
        if self.segments.len() == 1
            && self.segments[0].color.is_none()
            && self.segments[0].font_size.is_none()
            && self.segments[0].line_height.is_none()
            && self.segments[0].tracking.is_none()
            && self.segments[0].mono.is_none()
        {
            button.label(self.segments[0].text.clone())
        } else {
            button.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(tokens.ui(gap)))
                    .children(self.segments.into_iter().map(move |segment| {
                        let text_metrics = tokens.text(
                            segment.font_size.unwrap_or(font_size),
                            segment.line_height.unwrap_or(line_height),
                        );
                        let mut text = div()
                            .text_size(px(text_metrics.font_size))
                            .line_height(px(text_metrics.line_height))
                            .font_weight(FontWeight(segment.weight))
                            .child(segment.text);
                        if let Some(color) = segment.color {
                            text = text.text_color(rgba_from_u32(color, segment.alpha));
                        }
                        text.into_any_element()
                    })),
            )
        }
        .when_some(self.trailing_icon, |this, icon| this.child(icon.icon(cx)))
    }
}

fn size_for(size: MoonButtonSize) -> crate::Size {
    match size {
        MoonButtonSize::Micro => crate::Size::XSmall,
        MoonButtonSize::Action => crate::Size::Small,
        MoonButtonSize::Toolbar => crate::Size::Medium,
        MoonButtonSize::Pill => crate::Size::Large,
        MoonButtonSize::Custom { height, .. } => crate::Size::Size(px(height)),
    }
}

fn custom_radius(size: MoonButtonSize) -> Option<f32> {
    match size {
        MoonButtonSize::Pill => Some(999.0),
        MoonButtonSize::Custom { radius, .. } => Some(radius),
        _ => None,
    }
}

fn metrics_for(size: MoonButtonSize) -> (f32, f32, f32) {
    match size {
        MoonButtonSize::Micro => (10.0, 14.0, 4.0),
        MoonButtonSize::Action => (10.5, 16.0, 5.0),
        MoonButtonSize::Toolbar => (10.0, 16.0, 4.0),
        MoonButtonSize::Pill => (11.0, 16.0, 6.0),
        MoonButtonSize::Custom {
            font_size,
            line_height,
            gap,
            ..
        } => (font_size, line_height, gap),
    }
}

fn rgba_from_u32(color: u32, alpha: f32) -> Hsla {
    let mut color = rgb_from(color);
    color.a *= alpha;
    color
}
