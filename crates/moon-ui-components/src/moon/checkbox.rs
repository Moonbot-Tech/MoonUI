use crate::checkbox::Checkbox;
use crate::{Disableable, Sizable};
use gpui::*;

use super::{
    foundation::MoonSize,
    theme::MoonTheme,
    tokens::{MoonRect, MoonTone},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MoonCheckboxSize {
    /// A tier of the shared size scale. Checkboxes come in `Sm` (16px box) and `Md` (20px box);
    /// `Xs` renders as `Sm`, and `Lg` and above render as `Md`.
    Tier(MoonSize),
    Custom {
        box_size: f32,
        font_size: f32,
        line_height: f32,
        gap: f32,
        radius: f32,
    },
}

#[allow(non_upper_case_globals)]
impl MoonCheckboxSize {
    #[deprecated(note = "use `MoonSize::Sm`")]
    pub const Compact: Self = Self::Tier(MoonSize::Sm);
    #[deprecated(note = "use `MoonSize::Md`")]
    pub const Normal: Self = Self::Tier(MoonSize::Md);
}

impl From<MoonSize> for MoonCheckboxSize {
    fn from(size: MoonSize) -> Self {
        Self::Tier(size)
    }
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
    description: Option<SharedString>,
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
            description: None,
            checked: None,
            default_checked: false,
            disabled: false,
            indeterminate: false,
            size: MoonCheckboxSize::Tier(MoonSize::Md),
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

    /// Sets supporting text shown under the label in the muted text colour.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
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

    /// Sets the size: a tier such as `MoonSize::Sm`, or a `MoonCheckboxSize::Custom`.
    pub fn size(mut self, size: impl Into<MoonCheckboxSize>) -> Self {
        self.size = size.into();
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
            .indeterminate(self.indeterminate)
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
        if let Some(description) = self.description {
            checkbox = checkbox.description(description);
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
        MoonCheckboxSize::Tier(tier) => tier_size(tier),
        MoonCheckboxSize::Custom { box_size, .. } => crate::Size::Size(px(box_size)),
    }
}

/// Returns the checkbox size a tier renders at. Checkboxes, and radios with them, come in `Sm` and
/// `Md`; every other tier renders as the nearest of the two.
pub(crate) fn tier_size(tier: MoonSize) -> crate::Size {
    match tier {
        MoonSize::Xs | MoonSize::Sm => crate::Size::Small,
        MoonSize::Md | MoonSize::Lg | MoonSize::Xl | MoonSize::Xxl => crate::Size::Medium,
    }
}

#[cfg(test)]
mod tests;
