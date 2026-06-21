use std::rc::Rc;

use crate::input::{Input, InputEvent, InputState};
use crate::{Selectable, Sizable, Size};
use gpui::prelude::FluentBuilder as _;
use gpui::*;
use regex::Regex;

use super::{
    theme::MoonTheme,
    tokens::{MoonRect, MoonTone},
};

pub type MoonInputEvent = InputEvent;
pub type MoonInputState = InputState;
pub type MoonInputValidator = Rc<dyn Fn(&str, &mut Context<MoonInputState>) -> bool + 'static>;

pub fn bind_moon_input_keys(_cx: &mut App) {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MoonInputSize {
    Compact,
    Small,
    Normal,
    Custom {
        height: f32,
        radius: f32,
        font_size: f32,
        line_height: f32,
        pad_x: f32,
        pad_y: f32,
        gap: f32,
    },
}

#[derive(IntoElement)]
pub struct MoonInput {
    id: SharedString,
    bounds: Option<MoonRect>,
    state: Option<Entity<MoonInputState>>,
    placeholder: SharedString,
    default_value: SharedString,
    size: MoonInputSize,
    disabled: bool,
    cleanable: bool,
    clean_on_escape: Option<bool>,
    mask_toggle: bool,
    loading: Option<bool>,
    text_align: Option<TextAlign>,
    pattern: Option<Regex>,
    validate: Option<MoonInputValidator>,
    tone: MoonTone,
    mono: bool,
    selected: bool,
    prefix: Option<AnyElement>,
    suffix: Option<AnyElement>,
}

impl MoonInput {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            state: None,
            placeholder: SharedString::from(""),
            default_value: SharedString::from(""),
            size: MoonInputSize::Normal,
            disabled: false,
            cleanable: false,
            clean_on_escape: None,
            mask_toggle: false,
            loading: None,
            text_align: None,
            pattern: None,
            validate: None,
            tone: MoonTone::Info,
            mono: false,
            selected: false,
            prefix: None,
            suffix: None,
        }
    }

    pub fn state(mut self, state: &Entity<MoonInputState>) -> Self {
        self.state = Some(state.clone());
        self
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
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

    pub fn size(mut self, size: MoonInputSize) -> Self {
        self.size = size;
        self
    }

    pub fn small(self) -> Self {
        self.size(MoonInputSize::Small)
    }

    pub fn normal(self) -> Self {
        self.size(MoonInputSize::Normal)
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn cleanable(mut self, cleanable: bool) -> Self {
        self.cleanable = cleanable;
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

    pub fn mask_toggle(mut self) -> Self {
        self.mask_toggle = true;
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = Some(loading);
        self
    }

    pub fn text_align(mut self, align: TextAlign) -> Self {
        self.text_align = Some(align);
        self
    }

    pub fn pattern(mut self, pattern: Regex) -> Self {
        self.pattern = Some(pattern);
        self
    }

    pub fn validate(
        mut self,
        validator: impl Fn(&str, &mut Context<MoonInputState>) -> bool + 'static,
    ) -> Self {
        self.validate = Some(Rc::new(validator));
        self
    }

    pub fn tone(mut self, tone: MoonTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn mono(mut self, mono: bool) -> Self {
        self.mono = mono;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn prefix(mut self, prefix: impl IntoElement) -> Self {
        self.prefix = Some(prefix.into_any_element());
        self
    }

    pub fn suffix(mut self, suffix: impl IntoElement) -> Self {
        self.suffix = Some(suffix.into_any_element());
        self
    }
}

impl RenderOnce for MoonInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let clean_on_escape = self.clean_on_escape;
        let loading = self.loading;
        let text_align = self.text_align;
        let pattern = self.pattern;
        let validate = self.validate.clone();

        let state = self.state.unwrap_or_else(|| {
            cx.new(|cx| {
                let mut state = MoonInputState::new(window, cx)
                    .placeholder(self.placeholder.clone())
                    .default_value(self.default_value.clone());
                if clean_on_escape.unwrap_or(false) {
                    state = state.clean_on_escape();
                }
                if let Some(pattern) = pattern.clone() {
                    state = state.pattern(pattern);
                }
                if let Some(validate) = validate.clone() {
                    state = state.validate(move |text, cx| validate(text, cx));
                }
                state
            })
        });

        if clean_on_escape.is_some() || loading.is_some() || text_align.is_some() {
            state.update(cx, |state, cx| {
                if let Some(clean_on_escape) = clean_on_escape {
                    state.set_clean_on_escape(clean_on_escape, cx);
                }
                if let Some(loading) = loading {
                    state.set_loading(loading, window, cx);
                }
                if let Some(text_align) = text_align {
                    state.set_text_align(text_align, cx);
                }
            });
        }

        let _input_id = self.id;
        let mut input = Input::new(&state)
            .with_size(size_for(self.size))
            .disabled(self.disabled)
            .cleanable(self.cleanable)
            .selected(self.selected)
            .when_some(self.text_align, |this, align| this.text_align(align))
            .when(self.mono, |this| this.font_family(tokens.font_family(true)))
            .when_some(self.prefix, |this, prefix| this.prefix(prefix))
            .when_some(self.suffix, |this, suffix| this.suffix(suffix));
        if self.mask_toggle {
            input = input.mask_toggle();
        }
        if let Some(bounds) = self.bounds {
            input = input
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }
        if let MoonInputSize::Custom {
            height,
            radius,
            font_size,
            line_height,
            pad_x,
            pad_y,
            gap,
        } = self.size
        {
            let line_height = tokens.line_height(line_height);
            let height = tokens.ui(height).max(line_height + tokens.ui(pad_y) * 2.0);
            input = input
                .h(px(height))
                .rounded(px(tokens.ui(radius)))
                .text_size(px(tokens.font(font_size)))
                .line_height(px(line_height))
                .px(px(tokens.ui(pad_x)))
                .py(px(tokens.ui(pad_y)))
                .gap(px(tokens.ui(gap)));
        }
        input
    }
}

fn size_for(size: MoonInputSize) -> Size {
    match size {
        MoonInputSize::Compact => Size::XSmall,
        MoonInputSize::Small => Size::Small,
        MoonInputSize::Normal => Size::Medium,
        MoonInputSize::Custom { height, .. } => Size::Size(px(height)),
    }
}
