use gpui::prelude::FluentBuilder;
use gpui::*;

use super::{
    popover::{MoonPopover, MoonPopoverPlacement},
    text::MoonText,
    tokens::{MoonPalette, rgba_from},
};

pub enum MoonColorPickerEvent {
    Change(Hsla),
}

pub struct MoonColorPickerState {
    value: Hsla,
}

impl EventEmitter<MoonColorPickerEvent> for MoonColorPickerState {}

impl MoonColorPickerState {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            value: rgb(MoonPalette::active(cx).blue).into(),
        }
    }

    pub fn default_value(mut self, value: Hsla) -> Self {
        self.value = value;
        self
    }

    pub fn value(&self) -> Hsla {
        self.value
    }

    fn set_value(&mut self, value: Hsla, cx: &mut Context<Self>) {
        if self.value == value {
            return;
        }
        self.value = value;
        cx.emit(MoonColorPickerEvent::Change(value));
        cx.notify();
    }
}

#[derive(IntoElement)]
pub struct MoonColorPicker {
    id: SharedString,
    state: Entity<MoonColorPickerState>,
    disabled: bool,
    colors: Vec<Hsla>,
}

impl MoonColorPicker {
    pub fn new(state: &Entity<MoonColorPickerState>) -> Self {
        Self {
            id: SharedString::from(format!("moon-color-picker:{}", state.entity_id())),
            state: state.clone(),
            disabled: false,
            colors: Vec::new(),
        }
    }

    pub fn id(mut self, id: impl Into<SharedString>) -> Self {
        self.id = id.into();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn colors(mut self, colors: impl IntoIterator<Item = Hsla>) -> Self {
        self.colors = colors.into_iter().collect();
        self
    }

    fn hex_label(color: Hsla) -> SharedString {
        let rgba = color.to_rgb();
        let rgb = u32::from(rgba) >> 8;
        SharedString::from(format!("#{rgb:06X}"))
    }
}

impl RenderOnce for MoonColorPicker {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = MoonPalette::active(cx);
        let value = self.state.read(cx).value();
        let state = self.state.clone();
        let colors = if self.colors.is_empty() {
            vec![
                rgb(p.blue).into(),
                rgb(p.green).into(),
                rgb(p.red).into(),
                rgb(p.orange).into(),
                rgb(p.amber).into(),
                rgb(p.yellow).into(),
                rgb(p.text).into(),
                rgb(p.text_muted).into(),
                rgb(p.panel).into(),
                rgb(p.shell_high).into(),
            ]
        } else {
            self.colors
        };

        let trigger = div()
            .id(ElementId::from(SharedString::from(format!(
                "{}:trigger",
                self.id
            ))))
            .h(px(26.0))
            .w(px(128.0))
            .rounded(px(4.0))
            .border(px(1.0))
            .border_color(rgba_from(p.border, 1.0))
            .bg(rgba_from(
                p.shell_high,
                if self.disabled { 0.55 } else { 1.0 },
            ))
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(7.0))
            .cursor_default()
            .when(!self.disabled, |this| {
                this.hover(|this| this.border_color(rgba_from(0x343840, 1.0)))
            })
            .child(
                div()
                    .size(px(14.0))
                    .rounded(px(3.0))
                    .border(px(1.0))
                    .border_color(rgba_from(0x000000, 0.38))
                    .bg(value),
            )
            .child(
                MoonText::new(Self::hex_label(value))
                    .color(p.text_soft)
                    .alpha(if self.disabled { 0.45 } else { 1.0 })
                    .font_size(10.0)
                    .line_height(13.0)
                    .weight(500.0)
                    .mono(true)
                    .uppercase(false)
                    .render(),
            );

        let mut grid = div()
            .id(ElementId::from(SharedString::from(format!(
                "{}:grid",
                self.id
            ))))
            .grid()
            .grid_cols(5)
            .gap(px(6.0));

        for (ix, color) in colors.into_iter().enumerate() {
            let state = state.clone();
            grid = grid.child(
                div()
                    .id(ElementId::from(SharedString::from(format!(
                        "{}:color:{ix}",
                        self.id
                    ))))
                    .size(px(22.0))
                    .rounded(px(4.0))
                    .border(px(1.0))
                    .border_color(if color == value {
                        rgba_from(p.blue, 1.0)
                    } else {
                        rgba_from(0x000000, 0.40)
                    })
                    .bg(color)
                    .when(!self.disabled, |this| {
                        this.hover(|this| {
                            this.border_color(rgba_from(p.text, 0.78)).shadow(vec![
                                super::foundation::box_shadow(
                                    px(0.0),
                                    px(0.0),
                                    px(10.0),
                                    px(0.0),
                                    rgba_from(p.blue, 0.18),
                                ),
                            ])
                        })
                        .on_click(move |_, _, cx| {
                            state.update(cx, |state, cx| state.set_value(color, cx));
                        })
                    }),
            );
        }

        MoonPopover::new(self.id)
            .trigger(trigger)
            .content(grid)
            .width(156.0)
            .placement(MoonPopoverPlacement::BottomStart)
            .disabled(self.disabled)
    }
}
