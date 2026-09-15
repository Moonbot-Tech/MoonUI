//! The Checkboxes gallery page: checkboxes and radios in every combination of size, text and
//! state, plus tones, text edge cases and live controlled examples.

use super::*;

/// Label shared by the matrix cells that carry one.
const MATRIX_LABEL: &str = "Remember me";
/// Support text shared by the matrix cells that carry it.
const MATRIX_SUPPORT_TEXT: &str = "Keep me signed in";
/// Label of the radio matrix cells that carry one.
const RADIO_LABEL: &str = "Limit order";
/// Width of the state names that start each matrix row.
const STATE_COLUMN_WIDTH: f32 = 116.0;

/// The text a showcased control carries: one matrix column each.
#[derive(Clone, Copy)]
enum ChoiceText {
    None,
    Label,
    LabelAndSupport,
    Support,
}

impl ChoiceText {
    /// Every text combination a checkbox supports.
    const CHECKBOX: [Self; 4] = [
        Self::None,
        Self::Label,
        Self::LabelAndSupport,
        Self::Support,
    ];
    /// Every text combination a radio supports; radios have no support text.
    const RADIO: [Self; 2] = [Self::None, Self::Label];

    /// Returns the column heading.
    fn title(self) -> &'static str {
        match self {
            Self::None => "No text",
            Self::Label => "Label",
            Self::LabelAndSupport => "Label + support",
            Self::Support => "Support only",
        }
    }

    /// Returns the id fragment that keeps each cell's element id unique.
    fn key(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Label => "label",
            Self::LabelAndSupport => "label-support",
            Self::Support => "support",
        }
    }

    /// Returns the column width for a checkbox or radio of `size`, wide enough that the shared
    /// label and support text stay on one line.
    fn width(self, size: MoonSize) -> f32 {
        let medium = size >= MoonSize::Md;
        match self {
            Self::None => 52.0,
            Self::Label if medium => 148.0,
            Self::Label => 128.0,
            Self::LabelAndSupport | Self::Support if medium => 196.0,
            Self::LabelAndSupport | Self::Support => 172.0,
        }
    }
}

/// The state a showcased control is drawn in: one matrix row each.
#[derive(Clone, Copy)]
enum ChoiceState {
    Unchecked,
    Checked,
    Indeterminate,
    Disabled,
    DisabledChecked,
    DisabledIndeterminate,
}

impl ChoiceState {
    /// Every state a checkbox can show.
    const CHECKBOX: [Self; 6] = [
        Self::Unchecked,
        Self::Checked,
        Self::Indeterminate,
        Self::Disabled,
        Self::DisabledChecked,
        Self::DisabledIndeterminate,
    ];
    /// Every state a radio can show; radios have no indeterminate state.
    const RADIO: [Self; 4] = [
        Self::Unchecked,
        Self::Checked,
        Self::Disabled,
        Self::DisabledChecked,
    ];

    /// Returns the row heading.
    fn title(self) -> &'static str {
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
    fn key(self) -> &'static str {
        match self {
            Self::Unchecked => "unchecked",
            Self::Checked => "checked",
            Self::Indeterminate => "indeterminate",
            Self::Disabled => "disabled",
            Self::DisabledChecked => "disabled-checked",
            Self::DisabledIndeterminate => "disabled-indeterminate",
        }
    }

    fn checked(self) -> bool {
        matches!(self, Self::Checked | Self::DisabledChecked)
    }

    fn indeterminate(self) -> bool {
        matches!(self, Self::Indeterminate | Self::DisabledIndeterminate)
    }

    fn disabled(self) -> bool {
        matches!(
            self,
            Self::Disabled | Self::DisabledChecked | Self::DisabledIndeterminate
        )
    }
}

/// Returns the lowercase id fragment for `size`.
fn size_key(size: MoonSize) -> String {
    format!("{size:?}").to_ascii_lowercase()
}

/// Returns a muted mono caption for matrix headings and case names.
fn caption(text: impl Into<SharedString>, cx: &App) -> MoonText {
    MoonText::new(text)
        .uppercase(false)
        .mono(true)
        .font_size(10.5)
        .line_height(13.0)
        .color(MoonPalette::active(cx).text_muted)
}

/// Returns the checkbox for one matrix cell.
///
/// Every cell keeps its own checked state, so an enabled one can be clicked through.
fn matrix_checkbox(size: MoonSize, text: ChoiceText, state: ChoiceState) -> MoonCheckbox {
    let id = format!(
        "checkboxes-checkbox-{}-{}-{}",
        size_key(size),
        text.key(),
        state.key()
    );
    let checkbox = MoonCheckbox::new(SharedString::from(id))
        .size(size)
        .default_checked(state.checked())
        .indeterminate(state.indeterminate())
        .disabled(state.disabled());
    match text {
        ChoiceText::None => checkbox,
        ChoiceText::Label => checkbox.label(MATRIX_LABEL),
        ChoiceText::LabelAndSupport => checkbox
            .label(MATRIX_LABEL)
            .description(MATRIX_SUPPORT_TEXT),
        ChoiceText::Support => checkbox.description(MATRIX_SUPPORT_TEXT),
    }
}

/// Returns the radio for one matrix cell of `size`, drawn in `state`.
fn matrix_radio(size: MoonSize, text: ChoiceText, state: ChoiceState) -> MoonRadio {
    let id = format!(
        "checkboxes-radio-{}-{}-{}",
        size_key(size),
        text.key(),
        state.key()
    );
    let radio = MoonRadio::new(SharedString::from(id))
        .size(radio_size(size))
        .checked(state.checked())
        .disabled(state.disabled());
    match text {
        ChoiceText::Label => radio.label(RADIO_LABEL),
        _ => radio,
    }
}

/// Maps a shared size tier onto the radio's own sizes: `Sm` is `Compact`, `Md` is `Normal`.
fn radio_size(size: MoonSize) -> MoonRadioSize {
    if size >= MoonSize::Md {
        MoonRadioSize::Normal
    } else {
        MoonRadioSize::Compact
    }
}

/// Renders a matrix card: a heading row of `texts`, then a row per state whose cells come from
/// `cell`.
fn choice_matrix<E: IntoElement>(
    title: &'static str,
    size: MoonSize,
    texts: &[ChoiceText],
    states: &[ChoiceState],
    cell: impl Fn(ChoiceText, ChoiceState) -> E,
    cx: &App,
) -> gpui::Div {
    let heading = h_flex()
        .gap(px(12.0))
        .child(div().w(px(STATE_COLUMN_WIDTH)))
        .children(texts.iter().map(|text| {
            div()
                .w(px(text.width(size)))
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
                        .map(|&text| div().w(px(text.width(size))).child(cell(text, state))),
                )
        }));
    card(title, cx).child(rows)
}

impl Gallery {
    /// Renders the Checkboxes page.
    ///
    /// Args:
    ///     cx: Gallery context used for theme lookup and interaction listeners.
    ///
    /// Returns:
    ///     The complete checkboxes section.
    pub(super) fn render_checkboxes(&self, cx: &mut Context<Self>) -> impl IntoElement {
        section("Checkboxes", cx)
            .child(
                h_flex()
                    .items_start()
                    .gap(px(12.0))
                    .child(checkbox_matrix(MoonSize::Sm, "Checkbox / Sm", cx))
                    .child(
                        v_flex()
                            .gap(px(12.0))
                            .child(radio_matrix(MoonSize::Sm, "Radio / Sm (Compact)", cx))
                            .child(radio_matrix(MoonSize::Md, "Radio / Md (Normal)", cx)),
                    ),
            )
            .child(
                h_flex()
                    .items_start()
                    .gap(px(12.0))
                    .child(checkbox_matrix(MoonSize::Md, "Checkbox / Md", cx))
                    .child(tones_card(cx)),
            )
            .child(
                h_flex()
                    .items_start()
                    .gap(px(12.0))
                    .child(self.render_live_choices(cx))
                    .child(text_edge_cases_card(cx)),
            )
    }

    /// Renders controlled checkboxes and radio groups whose values live in the gallery state and
    /// are reported to the event log.
    fn render_live_choices(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let checkbox = |id: &'static str, size: MoonSize, checked: bool| {
            let view = view.clone();
            MoonCheckbox::new(id)
                .size(size)
                .label(format!("Alerts ({size:?})"))
                .description("Held in gallery state")
                .checked(checked)
                .on_change(move |checked, _, app| {
                    let checked = *checked;
                    view.update(app, |this, cx| {
                        match size {
                            MoonSize::Sm => this.checkbox_sm_checked = checked,
                            _ => this.checkbox_md_checked = checked,
                        }
                        this.push_event(format!("MoonCheckbox {size:?}: {checked}"), cx);
                    });
                })
        };
        let radio_group = |size: MoonSize, selected: usize| {
            h_flex().gap(px(14.0)).children(
                ["fast", "balanced", "safe"]
                    .into_iter()
                    .enumerate()
                    .map(|(ix, name)| {
                        let view = view.clone();
                        MoonRadio::new(SharedString::from(format!(
                            "checkboxes-live-radio-{}-{name}",
                            size_key(size)
                        )))
                        .size(radio_size(size))
                        .label(name)
                        .checked(selected == ix)
                        // The last option shows that a disabled radio in a group ignores clicks.
                        .disabled(ix == 2)
                        .on_change(move |_, _, app| {
                            view.update(app, |this, cx| {
                                match size {
                                    MoonSize::Sm => this.radio_sm_index = ix,
                                    _ => this.radio_md_index = ix,
                                }
                                this.push_event(format!("MoonRadio {size:?}: {name}"), cx);
                            });
                        })
                    }),
            )
        };

        card("Live controlled state", cx)
            .w(px(420.0))
            .child(caption("Checkbox values held by the gallery", cx).render())
            .child(
                h_flex()
                    .items_start()
                    .gap(px(18.0))
                    .child(checkbox(
                        "checkboxes-live-checkbox-sm",
                        MoonSize::Sm,
                        self.checkbox_sm_checked,
                    ))
                    .child(checkbox(
                        "checkboxes-live-checkbox-md",
                        MoonSize::Md,
                        self.checkbox_md_checked,
                    )),
            )
            .child(caption("Radio groups (Sm, then Md)", cx).render())
            .child(radio_group(MoonSize::Sm, self.radio_sm_index))
            .child(radio_group(MoonSize::Md, self.radio_md_index))
    }
}

/// Renders the checkbox matrix for `size`: every state against every text combination.
fn checkbox_matrix(size: MoonSize, title: &'static str, cx: &App) -> gpui::Div {
    choice_matrix(
        title,
        size,
        &ChoiceText::CHECKBOX,
        &ChoiceState::CHECKBOX,
        |text, state| matrix_checkbox(size, text, state),
        cx,
    )
}

/// Renders the radio matrix for `size`: every radio state with and without a label.
fn radio_matrix(size: MoonSize, title: &'static str, cx: &App) -> gpui::Div {
    choice_matrix(
        title,
        size,
        &ChoiceText::RADIO,
        &ChoiceState::RADIO,
        |text, state| matrix_radio(size, text, state),
        cx,
    )
}

/// Renders a checked checkbox and radio in every tone, since the tone colours the checked fill.
fn tones_card(cx: &App) -> gpui::Div {
    const TONES: [(MoonTone, &str); 9] = [
        (MoonTone::Info, "Info"),
        (MoonTone::Accent, "Accent"),
        (MoonTone::Positive, "Positive"),
        (MoonTone::Warning, "Warning"),
        (MoonTone::Notice, "Notice"),
        (MoonTone::Danger, "Danger"),
        (MoonTone::Negative, "Negative"),
        (MoonTone::Muted, "Muted"),
        (MoonTone::Default, "Default"),
    ];
    card("Tones", cx).children(TONES.into_iter().map(|(tone, name)| {
        h_flex()
            .gap(px(12.0))
            .child(div().w(px(64.0)).child(caption(name, cx).render()))
            .child(
                MoonCheckbox::new(SharedString::from(format!(
                    "checkboxes-tone-checkbox-{name}"
                )))
                .size(MoonSize::Sm)
                .tone(tone)
                .checked(true),
            )
            .child(
                MoonCheckbox::new(SharedString::from(format!("checkboxes-tone-mixed-{name}")))
                    .size(MoonSize::Sm)
                    .tone(tone)
                    .indeterminate(true),
            )
            .child(
                MoonRadio::new(SharedString::from(format!("checkboxes-tone-radio-{name}")))
                    .size(MoonRadioSize::Compact)
                    .tone(tone)
                    .checked(true),
            )
    }))
}

/// Renders text cases the matrices do not: labels and support text that wrap in a narrow column,
/// and the mono font.
fn text_edge_cases_card(cx: &App) -> gpui::Div {
    const LONG_LABEL: &str = "Close every open position when the stop price is reached";
    const LONG_SUPPORT: &str = "Orders are sent at market, so the fill can differ from the stop";

    let case = |name: &'static str, size: MoonSize, content: MoonCheckbox| {
        v_flex()
            .w(px(220.0))
            .gap(px(6.0))
            .child(caption(format!("{name} / {size:?}"), cx).render())
            .child(content.size(size))
    };
    let column = |size: MoonSize| {
        let key = size_key(size);
        v_flex()
            .gap(px(14.0))
            .child(case(
                "Wrapping label",
                size,
                MoonCheckbox::new(SharedString::from(format!("checkboxes-wrap-label-{key}")))
                    .label(LONG_LABEL)
                    .default_checked(true),
            ))
            .child(case(
                "Wrapping label + support",
                size,
                MoonCheckbox::new(SharedString::from(format!("checkboxes-wrap-support-{key}")))
                    .label(LONG_LABEL)
                    .description(LONG_SUPPORT),
            ))
            .child(case(
                "Mono label + support",
                size,
                MoonCheckbox::new(SharedString::from(format!("checkboxes-mono-{key}")))
                    .label("Risk lock")
                    .description("Mono font")
                    .mono(true)
                    .default_checked(true),
            ))
    };

    card("Text edge cases", cx).child(
        h_flex()
            .items_start()
            .gap(px(18.0))
            .child(column(MoonSize::Sm))
            .child(column(MoonSize::Md)),
    )
}
