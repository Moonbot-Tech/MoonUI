//! Matrix scaffolding shared by the choice-control pages: the text and state axes every checkbox,
//! radio and toggle is shown against, and the card that lays one out.

use super::*;

/// Width of the state names that start each matrix row.
pub(super) const STATE_COLUMN_WIDTH: f32 = 116.0;

/// The text a showcased control carries: one matrix column each.
#[derive(Clone, Copy)]
pub(super) enum ChoiceText {
    None,
    Label,
    LabelAndSupport,
    Support,
}

impl ChoiceText {
    /// Every text combination; checkboxes and radios support them all.
    pub(super) const ALL: [Self; 4] = [
        Self::None,
        Self::Label,
        Self::LabelAndSupport,
        Self::Support,
    ];

    /// Returns the column heading.
    pub(super) fn title(self) -> &'static str {
        match self {
            Self::None => "No text",
            Self::Label => "Label",
            Self::LabelAndSupport => "Label + support",
            Self::Support => "Support only",
        }
    }

    /// Returns the id fragment that keeps each cell's element id unique.
    pub(super) fn key(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Label => "label",
            Self::LabelAndSupport => "label-support",
            Self::Support => "support",
        }
    }

    /// Returns the column width for a checkbox or radio of `size`, wide enough that the shared
    /// label and support text stay on one line.
    pub(super) fn width(self, size: MoonSize) -> f32 {
        let medium = size >= MoonSize::Md;
        match self {
            Self::None => 52.0,
            Self::Label if medium => 148.0,
            Self::Label => 128.0,
            Self::LabelAndSupport | Self::Support if medium => 196.0,
            Self::LabelAndSupport | Self::Support => 172.0,
        }
    }

    /// Returns the column width for a toggle of `size`: a checkbox's column widened by how much
    /// wider the track (36 or 44px) is than a checkbox's box (16 or 20px).
    pub(super) fn toggle_width(self, size: MoonSize) -> f32 {
        self.width(size) + if size >= MoonSize::Md { 24.0 } else { 20.0 }
    }
}

/// The state a showcased control is drawn in: one matrix row each.
#[derive(Clone, Copy)]
pub(super) enum ChoiceState {
    Unchecked,
    Checked,
    Indeterminate,
    Disabled,
    DisabledChecked,
    DisabledIndeterminate,
}

impl ChoiceState {
    /// Every state a checkbox can show.
    pub(super) const CHECKBOX: [Self; 6] = [
        Self::Unchecked,
        Self::Checked,
        Self::Indeterminate,
        Self::Disabled,
        Self::DisabledChecked,
        Self::DisabledIndeterminate,
    ];
    /// Every state a radio can show; radios have no indeterminate state.
    pub(super) const RADIO: [Self; 4] = [
        Self::Unchecked,
        Self::Checked,
        Self::Disabled,
        Self::DisabledChecked,
    ];
    /// Every state a toggle can show, the same as a radio's.
    pub(super) const TOGGLE: [Self; 4] = Self::RADIO;

    /// Returns the row heading.
    pub(super) fn title(self) -> &'static str {
        match self {
            Self::Unchecked => "Unchecked",
            Self::Checked => "Checked",
            Self::Indeterminate => "Indeterminate",
            Self::Disabled => "Disabled",
            Self::DisabledChecked => "Disabled checked",
            Self::DisabledIndeterminate => "Disabled mixed",
        }
    }

    /// Returns the id fragment that keeps each cell's element id unique.
    pub(super) fn key(self) -> &'static str {
        match self {
            Self::Unchecked => "unchecked",
            Self::Checked => "checked",
            Self::Indeterminate => "indeterminate",
            Self::Disabled => "disabled",
            Self::DisabledChecked => "disabled-checked",
            Self::DisabledIndeterminate => "disabled-indeterminate",
        }
    }

    pub(super) fn checked(self) -> bool {
        matches!(self, Self::Checked | Self::DisabledChecked)
    }

    pub(super) fn indeterminate(self) -> bool {
        matches!(self, Self::Indeterminate | Self::DisabledIndeterminate)
    }

    pub(super) fn disabled(self) -> bool {
        matches!(
            self,
            Self::Disabled | Self::DisabledChecked | Self::DisabledIndeterminate
        )
    }
}

/// Returns the lowercase id fragment for `size`.
pub(super) fn size_key(size: MoonSize) -> String {
    format!("{size:?}").to_ascii_lowercase()
}

/// Returns a muted mono caption for matrix headings and case names.
pub(super) fn caption(text: impl Into<SharedString>, cx: &App) -> MoonText {
    MoonText::new(text)
        .uppercase(false)
        .mono(true)
        .font_size(10.5)
        .line_height(13.0)
        .color(MoonPalette::active(cx).text_muted)
}

/// Renders a matrix card: a heading row of `texts` with each column `width` wide, then a row per
/// state whose cells come from `cell`.
pub(super) fn choice_matrix<E: IntoElement>(
    title: &'static str,
    width: impl Fn(ChoiceText) -> f32,
    texts: &[ChoiceText],
    states: &[ChoiceState],
    cell: impl Fn(ChoiceText, ChoiceState) -> E,
    cx: &App,
) -> gpui::Div {
    let heading = h_flex()
        .gap(px(12.0))
        .child(div().w(px(STATE_COLUMN_WIDTH)))
        .children(texts.iter().map(|&text| {
            div()
                .w(px(width(text)))
                .child(caption(text.title(), cx).render())
        }));
    // Rows sit closer than the card's own spacing so both checkbox matrices fit one snapshot.
    let rows = v_flex()
        .gap(px(6.0))
        .child(heading)
        .children(states.iter().map(|&state| {
            h_flex()
                .gap(px(12.0))
                .child(
                    div()
                        .w(px(STATE_COLUMN_WIDTH))
                        .child(caption(state.title(), cx).render()),
                )
                .children(
                    texts
                        .iter()
                        .map(|&text| div().w(px(width(text))).child(cell(text, state))),
                )
        }));
    card(title, cx).child(rows)
}
