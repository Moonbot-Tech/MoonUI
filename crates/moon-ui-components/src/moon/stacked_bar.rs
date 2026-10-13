//! Segmented progress bar: several coloured segments laid end to end against a total.

use gpui::prelude::FluentBuilder as _;
use gpui::*;

use super::{
    theme::MoonTheme,
    tokens::{MoonTone, rgba_from},
    tooltip::MoonTooltipView,
};

/// One coloured part of a [`MoonStackedBar`].
#[derive(Clone, Debug)]
pub struct MoonBarSegment {
    value: f64,
    tone: MoonTone,
    color: Option<u32>,
    label: Option<SharedString>,
}

impl MoonBarSegment {
    /// Create a segment worth `value` units, painted with the Info tone.
    ///
    /// Args:
    ///     value: Amount this segment represents; negative or non-finite amounts draw nothing.
    ///
    /// Returns:
    ///     A segment builder.
    pub fn new(value: f64) -> Self {
        Self {
            value,
            tone: MoonTone::Info,
            color: None,
            label: None,
        }
    }

    /// Paint the segment with a palette tone.
    ///
    /// Args:
    ///     tone: Segment tone.
    ///
    /// Returns:
    ///     The updated segment.
    pub fn tone(mut self, tone: MoonTone) -> Self {
        self.tone = tone;
        self
    }

    /// Paint the segment with an explicit colour, overriding the tone.
    ///
    /// Args:
    ///     color: `0xRRGGBB` colour.
    ///
    /// Returns:
    ///     The updated segment.
    pub fn color(mut self, color: u32) -> Self {
        self.color = Some(color);
        self
    }

    /// Show a tooltip, such as `Logs 1.2 GB`, while the segment is hovered.
    ///
    /// Args:
    ///     label: Tooltip text.
    ///
    /// Returns:
    ///     The updated segment.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/// Resolve each segment's share of the bar width.
///
/// The denominator is the larger of `total` and the segments' sum, so segments that overflow the
/// total still fit the bar instead of spilling past it, and a total larger than the sum leaves the
/// rest of the track empty.
///
/// Args:
///     values: Segment amounts in order.
///     total: Optional capacity the bar represents.
///
/// Returns:
///     One fraction in `0.0..=1.0` per segment; all zero when nothing positive is left to show.
fn segment_fractions(values: &[f64], total: Option<f64>) -> Vec<f32> {
    let clean = |value: f64| {
        if value.is_finite() && value > 0.0 {
            value
        } else {
            0.0
        }
    };
    let sum: f64 = values.iter().copied().map(clean).sum();
    let denominator = total.map_or(sum, |total| clean(total).max(sum));
    values
        .iter()
        .map(|value| {
            if denominator > 0.0 {
                (clean(*value) / denominator) as f32
            } else {
                0.0
            }
        })
        .collect()
}

/// Moon stacked bar: coloured segments summing toward a total on one rounded track.
#[derive(IntoElement)]
pub struct MoonStackedBar {
    id: ElementId,
    segments: Vec<MoonBarSegment>,
    total: Option<f64>,
    height: f32,
    radius: Option<f32>,
}

impl MoonStackedBar {
    /// Create an empty 6px bar whose total is the sum of its segments.
    ///
    /// Args:
    ///     id: Stable identity for the rendered bar.
    ///
    /// Returns:
    ///     A stacked-bar builder.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            segments: Vec::new(),
            total: None,
            height: 6.0,
            radius: None,
        }
    }

    /// Append one segment.
    ///
    /// Args:
    ///     segment: Segment to draw after the existing ones.
    ///
    /// Returns:
    ///     The updated bar.
    pub fn segment(mut self, segment: MoonBarSegment) -> Self {
        self.segments.push(segment);
        self
    }

    /// Append several segments in order.
    ///
    /// Args:
    ///     segments: Segments to draw after the existing ones.
    ///
    /// Returns:
    ///     The updated bar.
    pub fn segments(mut self, segments: impl IntoIterator<Item = MoonBarSegment>) -> Self {
        self.segments.extend(segments);
        self
    }

    /// Set the capacity the full bar represents, such as a disk size.
    ///
    /// Args:
    ///     total: Capacity; the segments' sum is used when it is larger.
    ///
    /// Returns:
    ///     The updated bar.
    pub fn total(mut self, total: f64) -> Self {
        self.total = Some(total);
        self
    }

    /// Set the design-reference bar height.
    ///
    /// Args:
    ///     height: Height before UI scaling.
    ///
    /// Returns:
    ///     The updated bar.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Set the design-reference corner radius; half the height by default.
    ///
    /// Args:
    ///     radius: Radius before UI scaling.
    ///
    /// Returns:
    ///     The updated bar.
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
        self
    }
}

impl RenderOnce for MoonStackedBar {
    /// Render the track and its segments with the active palette.
    ///
    /// Args:
    ///     cx: Application context used to resolve the active theme.
    ///
    /// Returns:
    ///     The rendered bar.
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        let height = tokens.ui(self.height);
        let radius = self
            .radius
            .map(|radius| tokens.ui(radius))
            .unwrap_or(height * 0.5);
        let values: Vec<f64> = self.segments.iter().map(|segment| segment.value).collect();
        let fractions = segment_fractions(&values, self.total);

        div()
            .id(self.id)
            .w_full()
            .h(px(height))
            .flex()
            .flex_row()
            .overflow_hidden()
            .rounded(px(radius))
            .bg(rgba_from(p.border, 0.6))
            .children(self.segments.into_iter().zip(fractions).enumerate().map(
                |(ix, (segment, fraction))| {
                    let color = segment.color.unwrap_or_else(|| segment.tone.color(p));
                    div()
                        .id(("stacked-bar-segment", ix))
                        .debug_selector(move || format!("moon-stacked-bar:{ix}"))
                        .flex_none()
                        .h_full()
                        .w(relative(fraction))
                        .bg(rgba_from(color, 1.0))
                        .when_some(segment.label, |this, label| {
                            this.tooltip(move |_window, cx| {
                                cx.new(|_| MoonTooltipView::new(label.clone())).into()
                            })
                        })
                },
            ))
    }
}

#[cfg(test)]
mod tests;
