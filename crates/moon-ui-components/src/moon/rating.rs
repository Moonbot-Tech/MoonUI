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
        self.value = value.min(self.max);
        self
    }

    pub fn max(mut self, max: usize) -> Self {
        self.max = max.max(1);
        self.value = self.value.min(self.max);
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
        let on_click = self.on_click.clone();
        let parent_view = window.current_view();

        h_flex()
            .id(ElementId::from(self.id))
            .gap(px(tokens.ui(2.0)))
            .children((1..=self.max).map(move |ix| {
                let filled = ix <= self.value;
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
                                    if let Some(on_click) = &on_click {
                                        on_click(&ix, window, app);
                                    }
                                    app.notify(parent_view);
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
