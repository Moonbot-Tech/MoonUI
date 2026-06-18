use gpui::prelude::FluentBuilder;
use gpui::*;

use super::tokens::MoonRect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoonWindowChromeButton {
    Minimize,
    Maximize,
    Close,
}

impl MoonWindowChromeButton {
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
}

pub struct MoonWindowChrome {
    id: ElementId,
    bounds: MoonRect,
    drag: Option<MoonRect>,
    controls: Option<MoonRect>,
    buttons: Vec<MoonWindowChromeButton>,
    button_width: f32,
}

impl MoonWindowChrome {
    pub fn new(id: impl Into<ElementId>, bounds: MoonRect) -> Self {
        Self {
            id: id.into(),
            bounds,
            drag: None,
            controls: None,
            buttons: Vec::new(),
            button_width: 32.0,
        }
    }

    pub fn drag_bounds(mut self, drag: MoonRect) -> Self {
        self.drag = Some(drag);
        self
    }

    pub fn no_drag_region(mut self) -> Self {
        self.drag = None;
        self
    }

    pub fn controls_bounds(mut self, controls: MoonRect) -> Self {
        self.controls = Some(controls);
        self
    }

    pub fn no_controls(mut self) -> Self {
        self.controls = None;
        self.buttons.clear();
        self
    }

    pub fn button_width(mut self, button_width: f32) -> Self {
        self.button_width = button_width;
        self
    }

    pub fn buttons(mut self, buttons: impl IntoIterator<Item = MoonWindowChromeButton>) -> Self {
        self.buttons = buttons.into_iter().collect();
        self
    }

    pub fn render(self) -> impl IntoElement {
        let mut root = div()
            .id(self.id)
            .absolute()
            .left(px(self.bounds.x))
            .top(px(self.bounds.y))
            .w(px(self.bounds.w))
            .h(px(self.bounds.h));

        if let Some(drag) = self.drag {
            root = root.child(
                div()
                    .absolute()
                    .left(px(drag.x))
                    .top(px(drag.y))
                    .w(px(drag.w))
                    .h(px(drag.h))
                    .window_control_area(WindowControlArea::Drag)
                    .on_mouse_down(MouseButton::Left, |event, window, _| {
                        if event.click_count >= 2 {
                            window.titlebar_double_click();
                        } else {
                            window.start_window_move();
                        }
                    }),
            );
        }

        if let Some(controls) = self.controls {
            let mut control_row = div()
                .absolute()
                .left(px(controls.x))
                .top(px(controls.y))
                .w(px(controls.w))
                .h(px(controls.h));

            for (index, button) in self.buttons.into_iter().enumerate() {
                let x = index as f32 * self.button_width;
                control_row = control_row.child(
                    div()
                        .absolute()
                        .left(px(x))
                        .top(px(0.0))
                        .w(px(self.button_width))
                        .h(px(controls.h))
                        .window_control_area(button.control_area())
                        .when(cfg!(not(target_os = "windows")), |this| {
                            this.on_mouse_down(MouseButton::Left, move |_, window, _| {
                                button.invoke(window);
                            })
                        }),
                );
            }

            root = root.child(control_row);
        }

        root
    }
}
