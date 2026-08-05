use gpui::*;

use super::{
    theme::MoonTheme,
    tokens::{MoonRect, rgba_from},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoonSeparatorAxis {
    Horizontal,
    Vertical,
}

#[derive(IntoElement)]
pub struct MoonSeparator {
    axis: MoonSeparatorAxis,
    bounds: Option<MoonRect>,
    color: Option<u32>,
    alpha: f32,
    thickness: f32,
}

impl MoonSeparator {
    pub fn horizontal() -> Self {
        Self::new(MoonSeparatorAxis::Horizontal)
    }

    pub fn vertical() -> Self {
        Self::new(MoonSeparatorAxis::Vertical)
    }

    pub fn new(axis: MoonSeparatorAxis) -> Self {
        Self {
            axis,
            bounds: None,
            color: None,
            alpha: 1.0,
            thickness: 1.0,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn color(mut self, color: u32) -> Self {
        self.color = Some(color);
        self
    }

    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha;
        self
    }

    pub fn thickness(mut self, thickness: f32) -> Self {
        self.thickness = thickness;
        self
    }
}

impl MoonSeparator {
    /// Build the rule itself — the whole of this component's layout.
    ///
    /// Split out of [`RenderOnce::render`] so the sizing can be asserted directly: `render`'s
    /// `impl IntoElement` return cannot be inspected, and a separator that silently collapses to
    /// zero along the axis it spans is invisible rather than wrong-looking, so nothing else would
    /// catch it.
    pub(crate) fn line(self, cx: &App) -> Div {
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        let mut line = div()
            .relative()
            .flex_shrink_0()
            .bg(rgba_from(self.color.unwrap_or(p.border), self.alpha));

        line = match self.axis {
            MoonSeparatorAxis::Horizontal => line.w_full().h(px(tokens.ui(self.thickness))),
            // Cross-axis size comes from `self_stretch`, not `h_full`. A vertical rule separates
            // groups laid out in a ROW, and such a row is almost always content-height: a
            // percentage resolved against an indefinite parent gives zero, so `h_full` drew a
            // hairline of no height and the separator silently disappeared. `align_self: stretch`
            // takes the flex line's cross size instead, and overrides the row's `items_center`
            // for this one item without the caller having to know the row's height.
            //
            // Stretch needs the separator to be a DIRECT child of that row; wrapped in a
            // content-sized div it collapses again, and `bounds` below is the escape hatch for
            // that case (and for a non-flex parent, where `align_self` does nothing).
            MoonSeparatorAxis::Vertical => line.self_stretch().w(px(tokens.ui(self.thickness))),
        };

        if let Some(bounds) = self.bounds {
            line = line
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }

        line
    }
}

impl RenderOnce for MoonSeparator {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        self.line(cx)
    }
}

#[cfg(test)]
mod tests;
