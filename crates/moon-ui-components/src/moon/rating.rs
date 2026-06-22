//! Moon rating control.

use gpui::prelude::FluentBuilder;
use gpui::*;
use std::rc::Rc;

use super::{
    foundation::h_flex,
    text::MoonText,
    theme::MoonTheme,
    tokens::{MoonTone, rgba_from},
};

fn moon_rating_max(max: usize) -> usize {
    max.max(1)
}

fn moon_rating_value(value: usize, max: usize) -> usize {
    value.min(moon_rating_max(max))
}

fn moon_rating_click_value(ix: usize, max: usize, disabled: bool) -> Option<usize> {
    if disabled || ix == 0 {
        None
    } else {
        Some(moon_rating_value(ix, max))
    }
}

#[derive(IntoElement)]
pub struct MoonRating {
    id: SharedString,
    value: usize,
    max: usize,
    disabled: bool,
    tone: MoonTone,
    on_click: Option<Rc<dyn Fn(&usize, &mut Window, &mut App)>>,
}

impl MoonRating {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            value: 0,
            max: 5,
            disabled: false,
            tone: MoonTone::Warning,
            on_click: None,
        }
    }

    pub fn value(mut self, value: usize) -> Self {
        self.value = moon_rating_value(value, self.max);
        self
    }

    pub fn max(mut self, max: usize) -> Self {
        self.max = moon_rating_max(max);
        self.value = moon_rating_value(self.value, self.max);
        self
    }

    pub fn tone(mut self, tone: MoonTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    pub fn render(self) -> impl IntoElement {
        self
    }
}

impl RenderOnce for MoonRating {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        let active = self.tone.color(p);
        let inactive = p.text_muted;
        let disabled = self.disabled;
        let max = moon_rating_max(self.max);
        let value = moon_rating_value(self.value, max);
        let on_click = self.on_click.clone();
        let parent_view = window.current_view();

        h_flex()
            .id(ElementId::from(self.id))
            .gap(px(tokens.ui(2.0)))
            .children((1..=max).map(move |ix| {
                let filled = ix <= value;
                let on_click = on_click.clone();
                div()
                    .id(ix)
                    .px(px(tokens.ui(1.0)))
                    .opacity(if disabled { 0.42 } else { 1.0 })
                    .when(!disabled, |this| this.cursor_pointer())
                    .when(!disabled, |this| {
                        this.hover(|this| this.bg(rgba_from(active, 0.12)))
                            .on_click({
                                let parent_view = parent_view.clone();
                                move |_, window, app| {
                                    if let Some(selected) =
                                        moon_rating_click_value(ix, max, disabled)
                                    {
                                        if let Some(on_click) = &on_click {
                                            on_click(&selected, window, app);
                                        }
                                        app.notify(parent_view);
                                    }
                                }
                            })
                    })
                    .child(
                        MoonText::new(if filled { "★" } else { "☆" })
                            .uppercase(false)
                            .mono(true)
                            .font_size(tokens.text(14.0, 16.0).font_size)
                            .line_height(tokens.text(14.0, 16.0).line_height)
                            .weight(if filled { 800.0 } else { 500.0 })
                            .color(if filled { active } else { inactive })
                            .render(),
                    )
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::{moon_rating_click_value, moon_rating_max, moon_rating_value};

    #[test]
    fn rating_value_and_max_are_clamped() {
        assert_eq!(moon_rating_max(0), 1);
        assert_eq!(moon_rating_value(7, 5), 5);
        assert_eq!(moon_rating_value(0, 0), 0);
    }

    #[test]
    fn rating_click_value_respects_disabled_and_range() {
        assert_eq!(moon_rating_click_value(3, 5, false), Some(3));
        assert_eq!(moon_rating_click_value(8, 5, false), Some(5));
        assert_eq!(moon_rating_click_value(0, 5, false), None);
        assert_eq!(moon_rating_click_value(3, 5, true), None);
    }
}
