use crate::input::{Input, InputEvent, InputState};
use crate::{Selectable, Sizable, Size};
use gpui::prelude::FluentBuilder as _;
use gpui::*;

use super::{
    theme::MoonTheme,
    tokens::{MoonRect, MoonTone},
};

pub type MoonTextAreaEvent = InputEvent;
pub type MoonTextAreaState = InputState;

pub(crate) fn bind_moon_text_area_keys(_cx: &mut App) {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MoonTextAreaSize {
    Normal,
    Formula,
    Custom {
        height: f32,
        font_size: f32,
        line_height: f32,
        pad_x: f32,
        pad_y: f32,
        radius: f32,
    },
}

#[derive(Clone, Copy, Debug)]
struct TextAreaMetrics {
    height: f32,
    font_size: f32,
    line_height: f32,
    pad_x: f32,
    pad_y: f32,
    radius: f32,
    rows: usize,
}

#[derive(IntoElement)]
pub struct MoonTextArea {
    id: SharedString,
    bounds: Option<MoonRect>,
    state: Option<Entity<MoonTextAreaState>>,
    placeholder: SharedString,
    default_value: SharedString,
    disabled: bool,
    submit_on_enter: Option<bool>,
    clean_on_escape: Option<bool>,
    rows: Option<usize>,
    auto_grow_rows: Option<(usize, usize)>,
    selected: bool,
    size: MoonTextAreaSize,
    tone: MoonTone,
    mono: bool,
}

impl MoonTextArea {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            state: None,
            placeholder: SharedString::from(""),
            default_value: SharedString::from(""),
            disabled: false,
            submit_on_enter: None,
            clean_on_escape: None,
            rows: None,
            auto_grow_rows: None,
            selected: false,
            size: MoonTextAreaSize::Normal,
            tone: MoonTone::Warning,
            mono: true,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn state(mut self, state: &Entity<MoonTextAreaState>) -> Self {
        self.state = Some(state.clone());
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn default_value(mut self, value: impl Into<SharedString>) -> Self {
        self.default_value = value.into();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn submit_on_enter(mut self, submit: bool) -> Self {
        self.submit_on_enter = Some(submit);
        self
    }

    pub fn clean_on_escape(mut self) -> Self {
        self.clean_on_escape = Some(true);
        self
    }

    pub fn clean_on_escape_enabled(mut self, clean_on_escape: bool) -> Self {
        self.clean_on_escape = Some(clean_on_escape);
        self
    }

    pub fn rows(mut self, rows: usize) -> Self {
        self.rows = Some(rows.max(1));
        self.auto_grow_rows = None;
        self
    }

    pub fn auto_grow(mut self, min_rows: usize, max_rows: usize) -> Self {
        let min_rows = min_rows.max(1);
        let max_rows = max_rows.max(min_rows);
        self.rows = None;
        self.auto_grow_rows = Some((min_rows, max_rows));
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn size(mut self, size: MoonTextAreaSize) -> Self {
        self.size = size;
        self
    }

    pub fn formula(self) -> Self {
        self.size(MoonTextAreaSize::Formula).auto_grow(3, 10)
    }

    pub fn tone(mut self, tone: MoonTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn mono(mut self, mono: bool) -> Self {
        self.mono = mono;
        self
    }

    fn metrics(&self) -> TextAreaMetrics {
        match self.size {
            MoonTextAreaSize::Normal => TextAreaMetrics {
                height: 58.0,
                font_size: 11.5,
                line_height: 15.0,
                pad_x: 8.0,
                pad_y: 7.0,
                radius: 3.0,
                rows: 3,
            },
            MoonTextAreaSize::Formula => TextAreaMetrics {
                height: 74.0,
                font_size: 12.0,
                line_height: 15.0,
                pad_x: 10.0,
                pad_y: 8.0,
                radius: 3.0,
                rows: 4,
            },
            MoonTextAreaSize::Custom {
                height,
                font_size,
                line_height,
                pad_x,
                pad_y,
                radius,
            } => TextAreaMetrics {
                height,
                font_size,
                line_height,
                pad_x,
                pad_y,
                radius,
                rows: ((height - pad_y * 2.0) / line_height).max(1.0) as usize,
            },
        }
    }
}

impl RenderOnce for MoonTextArea {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let metrics = self.metrics();
        let tokens = MoonTheme::active_tokens(cx);
        let line_height = tokens.line_height(metrics.line_height);
        let height = tokens
            .ui(metrics.height)
            .max(line_height + tokens.ui(metrics.pad_y) * 2.0);
        let rows = self.rows.unwrap_or(metrics.rows).max(1);
        let auto_grow_rows = self.auto_grow_rows;
        let submit_on_enter = self.submit_on_enter;
        let clean_on_escape = self.clean_on_escape;
        let placeholder = self.placeholder.clone();
        let default_value = self.default_value.clone();

        let state = self.state.as_ref().cloned().unwrap_or_else(|| {
            let id = ElementId::from(self.id.clone());
            window.use_keyed_state(id, cx, move |window, cx| {
                let mut state = MoonTextAreaState::new(window, cx)
                    .placeholder(placeholder.clone())
                    .default_value(default_value.clone());
                state = if let Some((min_rows, max_rows)) = auto_grow_rows {
                    state.auto_grow(min_rows, max_rows)
                } else {
                    state.multi_line(true).rows(rows)
                };
                if let Some(submit_on_enter) = submit_on_enter {
                    state = state.submit_on_enter(submit_on_enter);
                }
                if clean_on_escape.unwrap_or(false) {
                    state = state.clean_on_escape();
                }
                state
            })
        });

        state.update(cx, |state, cx| {
            if let Some((min_rows, max_rows)) = auto_grow_rows {
                state.set_auto_grow(min_rows, max_rows, cx);
            } else {
                state.set_multi_line(true, cx);
                state.set_rows(rows, cx);
            }
            if let Some(submit_on_enter) = submit_on_enter {
                state.set_submit_on_enter(submit_on_enter, cx);
            }
            if let Some(clean_on_escape) = clean_on_escape {
                state.set_clean_on_escape(clean_on_escape, cx);
            }
        });

        let mut input = Input::new(&state)
            .with_size(Size::Size(px(metrics.height)))
            .disabled(self.disabled)
            .selected(self.selected)
            .tone(self.tone)
            .h(px(height))
            .rounded(px(tokens.ui(metrics.radius)))
            .text_size(px(tokens.font(metrics.font_size)))
            .line_height(px(line_height))
            .px(px(tokens.ui(metrics.pad_x)))
            .py(px(tokens.ui(metrics.pad_y)))
            .when(self.mono, |this| this.font_family(tokens.font_family(true)));

        if let Some(bounds) = self.bounds {
            input = input
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }

        input
    }
}
