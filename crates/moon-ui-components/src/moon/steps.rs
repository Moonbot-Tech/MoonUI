//! Numbered horizontal step indicator with done, current and pending states.

use gpui::prelude::FluentBuilder as _;
use gpui::*;

use super::{
    foundation::MoonSize,
    text::MoonText,
    theme::MoonTheme,
    tokens::{MoonPalette, MoonTone, rgba_from},
};

/// Minimum design-reference width of one step (marker, label and connector) before the row wraps.
const STEP_MIN_WIDTH: f32 = 120.0;
/// Design-reference length of the connector stub kept beside a step's label.
const STEP_CONNECTOR_MIN: f32 = 16.0;

/// Progress state of one step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoonStepState {
    /// Finished: an accent-filled marker.
    Done,
    /// In progress: an accent-outlined marker and a bold label.
    Current,
    /// Not reached yet: a muted marker and label.
    Pending,
}

/// Resolve a step's state from the index of the current step.
///
/// Args:
///     index: Zero-based step index.
///     current: Index of the current step; at or past the step count every step is done.
///
/// Returns:
///     The step's state.
fn step_state(index: usize, current: usize) -> MoonStepState {
    match index.cmp(&current) {
        std::cmp::Ordering::Less => MoonStepState::Done,
        std::cmp::Ordering::Equal => MoonStepState::Current,
        std::cmp::Ordering::Greater => MoonStepState::Pending,
    }
}

/// Colours one step marker draws with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StepColors {
    marker_fill: Option<u32>,
    marker_border: u32,
    number: u32,
    label: u32,
}

/// Resolve the marker and label colours of a step state.
///
/// Args:
///     state: Step state.
///     accent: Accent colour of done and current steps.
///     p: Palette the control paints with.
///
/// Returns:
///     The colours of the step's marker, number and label.
fn step_colors(state: MoonStepState, accent: u32, p: MoonPalette) -> StepColors {
    match state {
        MoonStepState::Done => StepColors {
            marker_fill: Some(accent),
            marker_border: accent,
            number: p.on_accent,
            label: p.text_soft,
        },
        MoonStepState::Current => StepColors {
            marker_fill: None,
            marker_border: accent,
            number: accent,
            label: p.text,
        },
        MoonStepState::Pending => StepColors {
            marker_fill: None,
            marker_border: p.border,
            number: p.text_muted,
            label: p.text_muted,
        },
    }
}

/// Moon step indicator: numbered steps joined by connectors, laid out in a row that wraps onto
/// further rows when the container is too narrow, or stacked vertically on request.
#[derive(IntoElement)]
pub struct MoonSteps {
    id: ElementId,
    labels: Vec<SharedString>,
    current: usize,
    tone: MoonTone,
    size: Option<MoonSize>,
    vertical: bool,
}

impl MoonSteps {
    /// Create an empty horizontal step indicator whose first step is current.
    ///
    /// Args:
    ///     id: Stable identity for the rendered control.
    ///
    /// Returns:
    ///     A default step-indicator builder.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            labels: Vec::new(),
            current: 0,
            tone: MoonTone::Info,
            size: None,
            vertical: false,
        }
    }

    /// Append one step.
    ///
    /// Args:
    ///     label: Step label; the step's number is drawn in its marker.
    ///
    /// Returns:
    ///     The updated builder.
    pub fn step(mut self, label: impl Into<SharedString>) -> Self {
        self.labels.push(label.into());
        self
    }

    /// Append several steps in order.
    ///
    /// Args:
    ///     labels: Step labels.
    ///
    /// Returns:
    ///     The updated builder.
    pub fn steps(mut self, labels: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.labels.extend(labels.into_iter().map(Into::into));
        self
    }

    /// Set the current step; earlier steps are done and later ones pending.
    ///
    /// Args:
    ///     index: Zero-based current step; the step count or more marks every step done.
    ///
    /// Returns:
    ///     The updated builder.
    pub fn current(mut self, index: usize) -> Self {
        self.current = index;
        self
    }

    /// Set the accent tone of done and current steps.
    ///
    /// Args:
    ///     tone: Accent tone.
    ///
    /// Returns:
    ///     The updated builder.
    pub fn tone(mut self, tone: MoonTone) -> Self {
        self.tone = tone;
        self
    }

    /// Override the density tier; Xs, Sm and Md are supported and larger tiers snap to Md.
    ///
    /// Args:
    ///     size: Requested tier.
    ///
    /// Returns:
    ///     The updated builder.
    pub fn size(mut self, size: MoonSize) -> Self {
        self.size = Some(size);
        self
    }

    /// Stack the steps in one column instead of a wrapping row.
    ///
    /// Args:
    ///     vertical: Whether to stack the steps.
    ///
    /// Returns:
    ///     The updated builder.
    pub fn vertical(mut self, vertical: bool) -> Self {
        self.vertical = vertical;
        self
    }

    /// Return the resolved state of every step, in order.
    ///
    /// Returns:
    ///     One state per step.
    pub fn states(&self) -> Vec<MoonStepState> {
        (0..self.labels.len())
            .map(|index| step_state(index, self.current))
            .collect()
    }
}

impl RenderOnce for MoonSteps {
    /// Render the steps with the active palette and tokens.
    ///
    /// Args:
    ///     cx: Application context used to resolve the active theme.
    ///
    /// Returns:
    ///     The rendered step indicator.
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        let accent = self.tone.color(p);
        let metrics = self
            .size
            .unwrap_or_else(|| tokens.tier())
            .nearest(&[MoonSize::Xs, MoonSize::Sm, MoonSize::Md])
            .control_metrics();
        let marker = tokens.ui(metrics.height - 4.0);
        let hairline = px(tokens.ui(1.0));
        let count = self.labels.len();
        let vertical = self.vertical;

        let mut root = div()
            .id(self.id)
            .flex()
            .when(vertical, |this| this.flex_col().gap(px(tokens.ui(6.0))))
            .when(!vertical, |this| {
                this.flex_row()
                    .flex_wrap()
                    .items_center()
                    .gap_y(px(tokens.ui(6.0)))
            });

        for (ix, label) in self.labels.into_iter().enumerate() {
            let state = step_state(ix, self.current);
            let colors = step_colors(state, accent, p);
            let last = ix + 1 == count;
            let marker_el = div()
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .size(px(marker))
                .rounded_full()
                .border(hairline)
                .border_color(rgba_from(colors.marker_border, 1.0))
                .when_some(colors.marker_fill, |this, fill| {
                    this.bg(rgba_from(fill, 1.0))
                })
                .child(
                    MoonText::new((ix + 1).to_string())
                        .color(colors.number)
                        .font_size(metrics.font_size - 2.0)
                        .line_height(metrics.line_height)
                        .weight(600.0)
                        .mono(true)
                        .uppercase(false)
                        .render(),
                );
            let label_el = MoonText::new(label)
                .color(colors.label)
                .font_size(metrics.font_size - 1.0)
                .line_height(metrics.line_height)
                .weight(if state == MoonStepState::Current {
                    600.0
                } else {
                    400.0
                })
                .uppercase(false)
                .render();
            let connector = div()
                .flex_1()
                .min_w(px(tokens.ui(STEP_CONNECTOR_MIN)))
                .h(hairline)
                .mx(px(tokens.ui(6.0)))
                .bg(rgba_from(
                    if state == MoonStepState::Done {
                        accent
                    } else {
                        p.border
                    },
                    1.0,
                ));

            let mut step = div()
                .debug_selector(move || format!("moon-step:{ix}"))
                .flex()
                .flex_row()
                .items_center()
                .gap(px(tokens.ui(6.0)))
                .child(marker_el)
                .child(label_el);
            if !vertical {
                step = step
                    .flex_grow_1()
                    .min_w(px(tokens.ui(STEP_MIN_WIDTH)))
                    .when(!last, |this| this.child(connector));
            }
            root = root.child(step);
        }

        root
    }
}

#[cfg(test)]
mod tests;
