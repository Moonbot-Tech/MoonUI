use gpui::*;
use crate::slider::{Slider, SliderEvent, SliderState};

use super::tokens::MoonRect;

pub type MoonSliderEvent = SliderEvent;
pub type MoonSliderState = SliderState;

#[derive(IntoElement)]
pub struct MoonSlider {
    inner: Slider,
    bounds: Option<MoonRect>,
    height: Option<f32>,
}

impl MoonSlider {
    pub fn new(state: &Entity<MoonSliderState>) -> Self {
        Self {
            inner: Slider::new(state),
            bounds: None,
            height: None,
        }
    }

    pub fn id(self, _id: impl Into<SharedString>) -> Self {
        self
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.inner = self.inner.disabled(disabled);
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }
}

impl RenderOnce for MoonSlider {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut slider = self.inner;
        if let Some(bounds) = self.bounds {
            slider = slider
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }
        if let Some(height) = self.height {
            slider = slider.h(px(height));
        }
        slider
    }
}
