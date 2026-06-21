use crate::checkbox::Checkbox;
use crate::{Disableable, Sizable};
use gpui::*;

use super::{
    theme::MoonTheme,
    tokens::{MoonRect, MoonTone},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MoonCheckboxSize {
    Compact,
    Normal,
    Custom {
        box_size: f32,
        font_size: f32,
        line_height: f32,
        gap: f32,
        radius: f32,
    },
}

#[derive(Default)]
struct MoonCheckboxState {
    checked: bool,
}

#[derive(IntoElement)]
pub struct MoonCheckbox {
    id: SharedString,
    bounds: Option<MoonRect>,
    label: Option<SharedString>,
    checked: Option<bool>,
    default_checked: bool,
    disabled: bool,
    indeterminate: bool,
    size: MoonCheckboxSize,
    tone: MoonTone,
    mono: bool,
    on_change: Option<std::rc::Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
}

impl MoonCheckbox {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            label: None,
            checked: None,
            default_checked: false,
            disabled: false,
            indeterminate: false,
            size: MoonCheckboxSize::Normal,
            tone: MoonTone::Info,
            mono: false,
            on_change: None,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }

    pub fn default_checked(mut self, checked: bool) -> Self {
        self.default_checked = checked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        self.indeterminate = indeterminate;
        self
    }

    pub fn size(mut self, size: MoonCheckboxSize) -> Self {
        self.size = size;
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

    pub fn on_change(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(std::rc::Rc::new(handler));
        self
    }
}

impl RenderOnce for MoonCheckbox {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state_id = ElementId::from(self.id.clone());
        let state = window.use_keyed_state(state_id.clone(), cx, |_, _| MoonCheckboxState {
            checked: self.default_checked,
        });
        let checked = self.checked.unwrap_or_else(|| state.read(cx).checked) || self.indeterminate;
        let controlled = self.checked.is_some();
        let on_change = self.on_change.clone();
        let state_for_click = state.clone();

        let mut checkbox = Checkbox::new(state_id)
            .checked(checked)
            .disabled(self.disabled)
            .tone(self.tone)
            .mono(self.mono)
            .with_size(size_for(self.size))
            .on_click(move |value, window, cx| {
                if !controlled {
                    state_for_click.update(cx, |state, cx| {
                        state.checked = *value;
                        cx.notify();
                    });
                }
                if let Some(handler) = &on_change {
                    handler(value, window, cx);
                }
            });

        if let Some(label) = self.label {
            checkbox = checkbox.label(label);
        }
        if let Some(bounds) = self.bounds {
            checkbox = checkbox
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }
        if let MoonCheckboxSize::Custom {
            font_size,
            line_height,
            ..
        } = self.size
        {
            let tokens = MoonTheme::active_tokens(cx);
            checkbox = checkbox
                .text_size(px(tokens.font(font_size)))
                .line_height(px(tokens.line_height(line_height)));
        }
        checkbox
    }
}

fn size_for(size: MoonCheckboxSize) -> crate::Size {
    match size {
        MoonCheckboxSize::Compact => crate::Size::Small,
        MoonCheckboxSize::Normal => crate::Size::Medium,
        MoonCheckboxSize::Custom { box_size, .. } => crate::Size::Size(px(box_size)),
    }
}
