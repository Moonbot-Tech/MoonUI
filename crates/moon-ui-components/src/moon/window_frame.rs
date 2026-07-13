use std::sync::Arc;

use gpui::prelude::FluentBuilder;
use gpui::*;

use super::{
    foundation::{box_shadow, h_flex},
    icons::MOON_ICON_WINDOW_CLOSE,
    theme::MoonTheme,
    tokens::rgba_from,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoonWindowFrameKind {
    Main,
    Tool,
    Popup,
    DetachedPanel,
    DetachedChart,
    Debug,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MoonWindowFrameBrand {
    #[default]
    Default,
    Full,
    Mark,
    None,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MoonWindowFrameControls {
    #[default]
    None,
    Close,
    MinimizeClose,
    MinimizeMaximizeClose,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FrameButton {
    Minimize,
    Maximize,
    Close,
}

impl MoonWindowFrameControls {
    fn buttons(self) -> &'static [FrameButton] {
        match self {
            Self::None => &[],
            Self::Close => &[FrameButton::Close],
            Self::MinimizeClose => &[FrameButton::Minimize, FrameButton::Close],
            Self::MinimizeMaximizeClose => &[
                FrameButton::Minimize,
                FrameButton::Maximize,
                FrameButton::Close,
            ],
        }
    }
}

impl FrameButton {
    fn control_area(self) -> WindowControlArea {
        match self {
            Self::Minimize => WindowControlArea::Min,
            Self::Maximize => WindowControlArea::Max,
            Self::Close => WindowControlArea::Close,
        }
    }

    fn invoke(self, window: &mut Window) {
        match self {
            Self::Minimize => window.minimize_window(),
            Self::Maximize => window.zoom_window(),
            Self::Close => window.remove_window(),
        }
    }

    fn icon(self, default_color: Hsla, hover_color: Hsla, group: SharedString) -> AnyElement {
        match self {
            Self::Minimize => div()
                .w(px(11.0))
                .h(px(1.5))
                .rounded_full()
                .bg(default_color)
                .group_hover(group.as_ref(), move |this| this.bg(hover_color))
                .into_any_element(),
            Self::Maximize => div()
                .w(px(9.0))
                .h(px(9.0))
                .border(px(1.5))
                .border_color(default_color)
                .group_hover(group.as_ref(), move |this| this.border_color(hover_color))
                .into_any_element(),
            Self::Close => svg()
                .w(px(12.0))
                .h(px(12.0))
                .path(MOON_ICON_WINDOW_CLOSE)
                .text_color(default_color)
                .group_hover(group.as_ref(), move |this| this.text_color(hover_color))
                .into_any_element(),
        }
    }
}

pub struct MoonWindowFrame {
    id: SharedString,
    kind: MoonWindowFrameKind,
    width: f32,
    header_h: f32,
    leading_inset: f32,
    right_inset: f32,
    controls: MoonWindowFrameControls,
    button_width: f32,
    button_height: f32,
    button_gap: f32,
    drag_width: Option<f32>,
    show_controls: bool,
    brand: MoonWindowFrameBrand,
}

const MOONBOT_LOGO_SVG: &str = include_str!("../../../../assets/moonbot-logo-full.svg");
const LOGO_SRC_W: f32 = 199.0;
const LOGO_SRC_H: f32 = 43.0;

impl MoonWindowFrame {
    pub fn new(id: impl Into<SharedString>, kind: MoonWindowFrameKind, width: f32) -> Self {
        let (controls, header_h, button_width, button_height, right_inset, drag_width) = match kind
        {
            MoonWindowFrameKind::Main => (
                MoonWindowFrameControls::MinimizeMaximizeClose,
                32.0,
                25.0,
                25.0,
                12.0,
                Some(116.0),
            ),
            MoonWindowFrameKind::Tool | MoonWindowFrameKind::DetachedPanel => (
                MoonWindowFrameControls::MinimizeClose,
                32.0,
                25.0,
                25.0,
                12.0,
                None,
            ),
            MoonWindowFrameKind::Popup => {
                (MoonWindowFrameControls::Close, 32.0, 25.0, 25.0, 12.0, None)
            }
            MoonWindowFrameKind::DetachedChart | MoonWindowFrameKind::Debug => (
                MoonWindowFrameControls::MinimizeClose,
                34.0,
                25.0,
                25.0,
                12.0,
                None,
            ),
        };

        Self {
            id: id.into(),
            kind,
            width,
            header_h,
            leading_inset: if cfg!(target_os = "macos") {
                76.0
            } else {
                12.0
            },
            right_inset,
            controls,
            button_width,
            button_height,
            button_gap: 2.0,
            drag_width,
            show_controls: !cfg!(target_os = "macos"),
            brand: MoonWindowFrameBrand::Default,
        }
    }

    pub fn main(id: impl Into<SharedString>, width: f32) -> Self {
        Self::new(id, MoonWindowFrameKind::Main, width)
    }

    pub fn tool(id: impl Into<SharedString>, width: f32) -> Self {
        Self::new(id, MoonWindowFrameKind::Tool, width)
    }

    pub fn popup(id: impl Into<SharedString>, width: f32) -> Self {
        Self::new(id, MoonWindowFrameKind::Popup, width)
    }

    pub fn detached_chart(id: impl Into<SharedString>, width: f32) -> Self {
        Self::new(id, MoonWindowFrameKind::DetachedChart, width)
    }

    pub fn detached_panel(id: impl Into<SharedString>, width: f32) -> Self {
        Self::new(id, MoonWindowFrameKind::DetachedPanel, width)
    }

    pub fn debug(id: impl Into<SharedString>, width: f32) -> Self {
        Self::new(id, MoonWindowFrameKind::Debug, width)
    }

    pub fn header_height(mut self, header_h: f32) -> Self {
        self.header_h = header_h;
        self
    }

    pub fn leading_inset(mut self, inset: f32) -> Self {
        self.leading_inset = inset;
        self
    }

    pub fn right_inset(mut self, inset: f32) -> Self {
        self.right_inset = inset;
        self
    }

    pub fn controls(mut self, controls: MoonWindowFrameControls) -> Self {
        self.controls = controls;
        self
    }

    pub fn brand(mut self, brand: MoonWindowFrameBrand) -> Self {
        self.brand = brand;
        self
    }

    pub fn button_width(mut self, width: f32) -> Self {
        self.button_width = width;
        self
    }

    pub fn button_height(mut self, height: f32) -> Self {
        self.button_height = height;
        self
    }

    pub fn drag_width(mut self, width: Option<f32>) -> Self {
        self.drag_width = width;
        self
    }

    pub fn show_controls(mut self, show: bool) -> Self {
        self.show_controls = show;
        self
    }

    pub fn leading_inset_value(&self) -> f32 {
        self.leading_inset
    }

    pub fn header_height_value(&self) -> f32 {
        self.header_h
    }

    pub fn controls_width(&self) -> f32 {
        if !self.show_controls {
            return 0.0;
        }
        let count = self.controls.buttons().len() as f32;
        if count == 0.0 {
            0.0
        } else {
            count * self.button_width + (count - 1.0) * self.button_gap
        }
    }

    pub fn visual_controls(&self, cx: &App) -> Div {
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        let side = tokens.ui(self.button_width.min(self.button_height));
        let mut row = h_flex()
            .h(px(side))
            .gap(px(tokens.ui(self.button_gap)))
            .font_family(tokens.font_family(true))
            .text_size(px(tokens.font(11.0)));

        if !self.show_controls {
            return row;
        }

        for button in self.controls.buttons() {
            let button = *button;
            let group = SharedString::from(format!("{}:{button:?}", self.id));
            let is_light = p.is_light();
            let (default_color, hover_color) = match button {
                FrameButton::Close => (
                    rgba_from(if is_light { p.red_text } else { p.orange }, 1.0),
                    rgba_from(p.on_accent, 1.0),
                ),
                FrameButton::Minimize | FrameButton::Maximize => {
                    let control = if is_light { p.text_faint } else { p.text };
                    let hover = if is_light { p.text_dim } else { p.text };
                    (rgba_from(control, 1.0), rgba_from(hover, 1.0))
                }
            };
            row = row.child(
                div()
                    .group(group.as_ref())
                    .w(px(side))
                    .h(px(side))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .text_color(default_color)
                    .window_control_area(button.control_area())
                    .hover(move |s| match button {
                        FrameButton::Close => s
                            .bg(rgba_from(p.red, if is_light { 0.14 } else { 0.5 }))
                            .text_color(rgba_from(p.on_accent, 1.0))
                            .shadow(vec![box_shadow(
                                px(0.0),
                                px(0.0),
                                px(14.0),
                                px(0.0),
                                rgba_from(p.red, if is_light { 0.18 } else { 0.55 }),
                            )]),
                        FrameButton::Minimize | FrameButton::Maximize => s
                            .bg(rgba_from(p.overlay, 0.09))
                            .text_color(rgba_from(if is_light { p.text_dim } else { p.text }, 1.0)),
                    })
                    .when(cfg!(not(target_os = "windows")), move |this| {
                        this.on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                            cx.stop_propagation();
                            button.invoke(window);
                        })
                    })
                    .child(button.icon(default_color, hover_color, group)),
            );
        }
        row
    }

    pub fn visual_brand(&self, cx: &App) -> Div {
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        match self.resolved_brand() {
            MoonWindowFrameBrand::Full => div().flex().items_center().child(logo_full(
                tokens.ui(86.0),
                if p.is_light() { p.text } else { 0xE7E7E7 },
            )),
            MoonWindowFrameBrand::Mark => div()
                .flex()
                .items_center()
                .child(logo_mark(tokens.ui(18.0))),
            MoonWindowFrameBrand::None | MoonWindowFrameBrand::Default => div(),
        }
    }

    pub fn brand_cluster(&self, cx: &App) -> Div {
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        let has_brand = !matches!(self.resolved_brand(), MoonWindowFrameBrand::None);
        self.drag_handle()
            .flex()
            .items_center()
            .gap(px(tokens.ui(8.0)))
            .child(self.visual_brand(cx))
            .when(has_brand, |this| {
                this.child(vline(tokens.ui(16.0), p.border))
            })
    }

    pub fn title_cluster(&self, title: impl Into<SharedString>, cx: &App) -> Div {
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        self.brand_cluster(cx).child(
            div()
                .min_w_0()
                .overflow_hidden()
                .font_family(tokens.font_family(true))
                .text_size(px(tokens.font(11.0)))
                .text_color(rgb(p.text_soft))
                .child(title.into()),
        )
    }

    pub fn drag_handle(&self) -> Div {
        drag_region_div()
    }

    pub fn hit_overlay(&self) -> impl IntoElement {
        let controls_w = self.controls_width();
        let drag_w = self
            .drag_width
            .unwrap_or_else(|| {
                if self.show_controls && controls_w > 0.0 {
                    self.width - self.leading_inset - controls_w - self.right_inset - 8.0
                } else {
                    self.width - self.leading_inset
                }
            })
            .max(0.0)
            .min(self.width.max(0.0));

        let mut root = div()
            .id(self.id.clone())
            .absolute()
            .left(px(0.0))
            .top(px(0.0))
            .w(px(self.width.max(0.0)))
            .h(px(self.header_h));

        if drag_w > 0.0 {
            root = root.child(
                drag_region_div()
                    .absolute()
                    .left(px(self.leading_inset))
                    .top(px(0.0))
                    .w(px(drag_w))
                    .h(px(self.header_h)),
            );
        }

        root
    }

    pub fn kind(&self) -> MoonWindowFrameKind {
        self.kind
    }

    fn resolved_brand(&self) -> MoonWindowFrameBrand {
        match self.brand {
            MoonWindowFrameBrand::Default => match self.kind {
                MoonWindowFrameKind::Main => MoonWindowFrameBrand::Full,
                MoonWindowFrameKind::Tool
                | MoonWindowFrameKind::DetachedPanel
                | MoonWindowFrameKind::DetachedChart
                | MoonWindowFrameKind::Debug => MoonWindowFrameBrand::Mark,
                MoonWindowFrameKind::Popup => MoonWindowFrameBrand::None,
            },
            explicit => explicit,
        }
    }
}

fn logo_full(width: f32, text_color: u32) -> impl IntoElement {
    let text_fill = format!("#{text_color:06X}");
    let svg = MOONBOT_LOGO_SVG.replace(r##"fill="#E7E7E7""##, &format!(r##"fill="{text_fill}""##));

    img(Arc::new(Image::from_bytes(
        ImageFormat::Svg,
        svg.into_bytes(),
    )))
    .w(px(width))
    .h(px(width * (LOGO_SRC_H / LOGO_SRC_W)))
}

fn logo_mark(size: f32) -> impl IntoElement {
    let paths = MOONBOT_LOGO_SVG
        .split_once(r#"<g clip-path="url(#clip0_3800_3393)">"#)
        .and_then(|(_, rest)| rest.split_once("</g>"))
        .map(|(paths, _)| paths)
        .unwrap_or("");
    let svg = format!(
        r#"<svg width="43" height="43" viewBox="0 0 43.5 43" fill="none" xmlns="http://www.w3.org/2000/svg">{paths}</svg>"#
    );
    img(Arc::new(Image::from_bytes(
        ImageFormat::Svg,
        svg.into_bytes(),
    )))
    .w(px(size))
    .h(px(size))
}

fn vline(height: f32, color: u32) -> impl IntoElement {
    div().w(px(1.0)).h(px(height)).bg(rgb(color))
}

fn drag_region_div() -> Div {
    div()
        .window_control_area(WindowControlArea::Drag)
        .on_mouse_down(MouseButton::Left, |event, window, _cx| {
            if event.click_count >= 2 {
                window.titlebar_double_click();
            } else {
                window.start_window_move();
            }
        })
}
