use std::rc::Rc;

use crate::{
    Disableable, Sizable, Size, StyledExt,
    checkbox::Checkbox,
    moon::{MoonSize, MoonToggle},
    setting::{
        AnySettingField, RenderOptions,
        fields::{SettingFieldRender, get_value, set_value},
    },
};
use gpui::{AnyElement, App, IntoElement, ParentElement as _, StyleRefinement, Window, div};

pub(crate) struct BoolField {
    use_switch: bool,
}

impl BoolField {
    pub(crate) fn new(use_switch: bool) -> Self {
        Self { use_switch }
    }
}

impl SettingFieldRender for BoolField {
    fn render(
        &self,
        field: Rc<dyn AnySettingField>,
        options: &RenderOptions,
        style: &StyleRefinement,
        _: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let checked = get_value::<bool>(&field, cx);
        let set_value = set_value::<bool>(&field, cx);

        div()
            .refine_style(style)
            .child(if self.use_switch {
                MoonToggle::new("check")
                    .checked(checked)
                    .disabled(options.disabled)
                    .size(toggle_tier(options.size))
                    .on_change(move |checked: &bool, _, cx: &mut App| {
                        set_value(*checked, cx);
                    })
                    .into_any_element()
            } else {
                Checkbox::new("check")
                    .checked(checked)
                    .disabled(options.disabled)
                    .with_size(options.size)
                    .on_click(move |checked: &bool, _, cx: &mut App| {
                        set_value(*checked, cx);
                    })
                    .into_any_element()
            })
            .into_any_element()
    }
}

/// Returns the toggle tier a settings field of `size` renders at.
///
/// A settings field carries the shared control size, which spans tiers the toggle does not draw;
/// each one resolves to the nearest toggle tier, and a field sized in pixels takes the tier its
/// height is closer to.
fn toggle_tier(size: Size) -> MoonSize {
    match size {
        Size::XSmall | Size::Small => MoonSize::Sm,
        Size::Medium | Size::Large => MoonSize::Md,
        Size::Size(height) => {
            if height.as_f32() < 22. {
                MoonSize::Sm
            } else {
                MoonSize::Md
            }
        }
    }
}
