//! The Checkboxes gallery page: checkboxes and radios in every combination of size, text and
//! state, plus tones, text edge cases and live controlled examples.

use super::choice::*;
use super::*;
use gpui::AnyElement;

/// Label shared by the matrix cells that carry one.
const MATRIX_LABEL: &str = "Remember me";
/// Support text shared by the matrix cells that carry it.
const MATRIX_SUPPORT_TEXT: &str = "Keep me signed in";
/// Label of the radio matrix cells that carry one.
const RADIO_LABEL: &str = "Limit order";
/// Support text of the radio matrix cells that carry it.
const RADIO_SUPPORT_TEXT: &str = "Fill at my price";

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
        .size(size)
        .checked(state.checked())
        .disabled(state.disabled());
    match text {
        ChoiceText::None => radio,
        ChoiceText::Label => radio.label(RADIO_LABEL),
        ChoiceText::LabelAndSupport => radio.label(RADIO_LABEL).description(RADIO_SUPPORT_TEXT),
        ChoiceText::Support => radio.description(RADIO_SUPPORT_TEXT),
    }
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
        // Both checkbox matrices come first so they fit the page snapshot; the radio groups beside
        // them show radios at both sizes there too.
        section("Checkboxes", cx)
            .child(
                h_flex()
                    .items_start()
                    .gap(px(12.0))
                    .child(checkbox_matrix(MoonSize::Sm, "Checkbox / Sm", cx))
                    .child(tones_card(cx)),
            )
            .child(
                h_flex()
                    .items_start()
                    .gap(px(12.0))
                    .child(checkbox_matrix(MoonSize::Md, "Checkbox / Md", cx))
                    .child(self.render_live_radios(cx)),
            )
            .child(
                h_flex()
                    .items_start()
                    .gap(px(12.0))
                    .child(radio_matrix(MoonSize::Sm, "Radio / Sm", cx))
                    .child(self.render_live_checkboxes(cx)),
            )
            .child(radio_matrix(MoonSize::Md, "Radio / Md", cx))
            .child(text_edge_cases_card(cx))
            .child(self.render_density_radios(cx))
    }

    /// Shows explicit Sm/Md overrides beside an unsized density-following radio.
    fn render_density_radios(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        card("Radio size overrides / density", cx).child(
            h_flex().gap(px(14.0)).flex_wrap().children(
                [
                    (
                        "fast (Sm)",
                        Some(MoonSize::Sm),
                        "Small radio with supporting text",
                    ),
                    (
                        "balanced (Md)",
                        Some(MoonSize::Md),
                        "Medium radio with supporting text",
                    ),
                    ("safe", None, "Active density"),
                ]
                .into_iter()
                .enumerate()
                .map(|(index, (label, tier, description))| {
                    let view = view.clone();
                    MoonRadio::new(format!("density-radio-{index}"))
                        .label(label)
                        .description(description)
                        .when_some(tier, |radio, tier| radio.size(tier))
                        .checked(self.radio_sm_index == index)
                        .disabled(index == 2)
                        .on_change(move |_, _, app| {
                            view.update(app, |this, cx| {
                                this.radio_sm_index = index;
                                this.push_event(format!("MoonRadio: {label}"), cx);
                            });
                        })
                }),
            ),
        )
    }

    /// Renders controlled checkboxes whose values live in the gallery state and are reported to
    /// the event log.
    fn render_live_checkboxes(&self, cx: &mut Context<Self>) -> impl IntoElement {
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

        card("Live checkboxes", cx)
            .child(caption("Values held by the gallery", cx).render())
            .child(checkbox(
                "checkboxes-live-checkbox-sm",
                MoonSize::Sm,
                self.checkbox_sm_checked,
            ))
            .child(checkbox(
                "checkboxes-live-checkbox-md",
                MoonSize::Md,
                self.checkbox_md_checked,
            ))
    }

    /// Renders controlled radio groups, one per size, whose selection lives in the gallery state
    /// and is reported to the event log.
    fn render_live_radios(&self, cx: &mut Context<Self>) -> impl IntoElement {
        const OPTIONS: [(&str, &str); 3] = [
            ("Fast", "Market order"),
            ("Balanced", "Limit at mid"),
            ("Safe", "Unavailable"),
        ];
        let view = cx.entity();
        let radio_group = |size: MoonSize, selected: usize| {
            v_flex()
                .gap(px(6.0))
                .children(
                    OPTIONS
                        .into_iter()
                        .enumerate()
                        .map(|(ix, (name, support))| {
                            let view = view.clone();
                            MoonRadio::new(SharedString::from(format!(
                                "checkboxes-live-radio-{}-{}",
                                size_key(size),
                                name.to_ascii_lowercase()
                            )))
                            .size(size)
                            .label(name)
                            .description(support)
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

        card("Live radio groups", cx)
            .child(caption("Sm", cx).render())
            .child(radio_group(MoonSize::Sm, self.radio_sm_index))
            .child(caption("Md", cx).render())
            .child(radio_group(MoonSize::Md, self.radio_md_index))
    }
}

/// Renders the checkbox matrix for `size`: every state against every text combination.
fn checkbox_matrix(size: MoonSize, title: &'static str, cx: &App) -> gpui::Div {
    choice_matrix(
        title,
        |text| text.width(size),
        &ChoiceText::ALL,
        &ChoiceState::CHECKBOX,
        |text, state| matrix_checkbox(size, text, state),
        cx,
    )
}

/// Renders the radio matrix for `size`: every radio state against every text combination.
fn radio_matrix(size: MoonSize, title: &'static str, cx: &App) -> gpui::Div {
    choice_matrix(
        title,
        |text| text.width(size),
        &ChoiceText::ALL,
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
                    .size(MoonSize::Sm)
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

    let case = |name: &'static str, size: MoonSize, content: AnyElement| {
        v_flex()
            .w(px(220.0))
            .gap(px(6.0))
            .child(caption(format!("{name} / {size:?}"), cx).render())
            .child(content)
    };
    let column = |size: MoonSize| {
        let key = size_key(size);
        v_flex()
            .gap(px(14.0))
            .child(case(
                "Wrapping label",
                size,
                MoonCheckbox::new(SharedString::from(format!("checkboxes-wrap-label-{key}")))
                    .size(size)
                    .label(LONG_LABEL)
                    .default_checked(true)
                    .into_any_element(),
            ))
            .child(case(
                "Wrapping label + support",
                size,
                MoonCheckbox::new(SharedString::from(format!("checkboxes-wrap-support-{key}")))
                    .size(size)
                    .label(LONG_LABEL)
                    .description(LONG_SUPPORT)
                    .into_any_element(),
            ))
            .child(case(
                "Wrapping radio + support",
                size,
                MoonRadio::new(SharedString::from(format!("checkboxes-wrap-radio-{key}")))
                    .size(size)
                    .label(LONG_LABEL)
                    .description(LONG_SUPPORT)
                    .checked(true)
                    .into_any_element(),
            ))
            .child(case(
                "Mono label + support",
                size,
                MoonCheckbox::new(SharedString::from(format!("checkboxes-mono-{key}")))
                    .size(size)
                    .label("Risk lock")
                    .description("Mono font")
                    .mono(true)
                    .default_checked(true)
                    .into_any_element(),
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
