//! The Toggles gallery page: every toggle variant and size against every state and text
//! combination, plus label side, text edge cases and live controlled toggles.

use super::choice::*;
use super::*;
use gpui::AnyElement;

/// Label of the matrix cells that carry one.
const TOGGLE_LABEL: &str = "Live prices";
/// Support text of the matrix cells that carry it.
const TOGGLE_SUPPORT_TEXT: &str = "Stream every tick";

/// Every matrix the page shows: a variant at a size, with the id fragment and heading that name
/// it. `Slim` draws as `Default` until its own design is specified, so today the two read alike.
const MATRICES: [(MoonToggleVariant, &str, MoonSize, &str); 4] = [
    (
        MoonToggleVariant::Default,
        "default",
        MoonSize::Sm,
        "Toggle / Default / Sm",
    ),
    (
        MoonToggleVariant::Default,
        "default",
        MoonSize::Md,
        "Toggle / Default / Md",
    ),
    (
        MoonToggleVariant::Slim,
        "slim",
        MoonSize::Sm,
        "Toggle / Slim / Sm",
    ),
    (
        MoonToggleVariant::Slim,
        "slim",
        MoonSize::Md,
        "Toggle / Slim / Md",
    ),
];

/// Returns the toggle for one matrix cell: `variant` at `size`, carrying `text` and drawn in
/// `state`. `variant_key` keeps each cell's element id unique.
///
/// Every cell keeps its own checked state, so an enabled one can be clicked through.
fn matrix_toggle(
    variant: MoonToggleVariant,
    variant_key: &str,
    size: MoonSize,
    text: ChoiceText,
    state: ChoiceState,
) -> MoonToggle {
    let id = format!(
        "toggles-{}-{}-{}-{}",
        variant_key,
        size_key(size),
        text.key(),
        state.key()
    );
    let toggle = MoonToggle::new(SharedString::from(id))
        .size(size)
        .variant(variant)
        .default_checked(state.checked())
        .disabled(state.disabled());
    match text {
        ChoiceText::None => toggle,
        ChoiceText::Label => toggle.label(TOGGLE_LABEL),
        ChoiceText::LabelAndSupport => toggle.label(TOGGLE_LABEL).description(TOGGLE_SUPPORT_TEXT),
        ChoiceText::Support => toggle.description(TOGGLE_SUPPORT_TEXT),
    }
}

/// Renders one variant's matrix at `size`: every toggle state against every text combination.
fn toggle_matrix(
    variant: MoonToggleVariant,
    variant_key: &'static str,
    size: MoonSize,
    title: &'static str,
    cx: &App,
) -> gpui::Div {
    choice_matrix(
        title,
        move |text| text.toggle_width(size),
        &ChoiceText::ALL,
        &ChoiceState::TOGGLE,
        move |text, state| matrix_toggle(variant, variant_key, size, text, state),
        cx,
    )
}

/// Renders a toggle with its text on either side, with and without supporting text: the track
/// stays beside the label instead of moving to the end of the row.
fn label_side_card(cx: &App) -> gpui::Div {
    let case = |name: &'static str, side: MoonToggleLabelSide, support: bool| {
        let id = format!(
            "toggles-side-{}-{}",
            match side {
                MoonToggleLabelSide::Left => "left",
                MoonToggleLabelSide::Right => "right",
            },
            if support { "support" } else { "label" }
        );
        v_flex()
            .gap(px(6.0))
            .child(caption(name, cx).render())
            .child(
                MoonToggle::new(SharedString::from(id))
                    .size(MoonSize::Md)
                    .label_side(side)
                    .label(TOGGLE_LABEL)
                    .default_checked(true)
                    .when(support, |toggle| toggle.description(TOGGLE_SUPPORT_TEXT)),
            )
    };

    card("Label side", cx).child(
        h_flex()
            .items_start()
            .gap(px(18.0))
            .flex_wrap()
            .child(case("Right", MoonToggleLabelSide::Right, false))
            .child(case("Right + support", MoonToggleLabelSide::Right, true))
            .child(case("Left", MoonToggleLabelSide::Left, false))
            .child(case("Left + support", MoonToggleLabelSide::Left, true)),
    )
}

/// Renders text cases the matrices do not: label and support text that wrap in a narrow column,
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
                MoonToggle::new(SharedString::from(format!("toggles-wrap-label-{key}")))
                    .size(size)
                    .label(LONG_LABEL)
                    .default_checked(true)
                    .into_any_element(),
            ))
            .child(case(
                "Wrapping label + support",
                size,
                MoonToggle::new(SharedString::from(format!("toggles-wrap-support-{key}")))
                    .size(size)
                    .label(LONG_LABEL)
                    .description(LONG_SUPPORT)
                    .into_any_element(),
            ))
            .child(case(
                "Mono label + support",
                size,
                MoonToggle::new(SharedString::from(format!("toggles-mono-{key}")))
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

/// Shows explicit Sm and Md overrides beside an unsized toggle, which follows the active density.
fn density_card(cx: &App) -> gpui::Div {
    const CASES: [(&str, Option<MoonSize>, &str); 3] = [
        ("fast (Sm)", Some(MoonSize::Sm), "Small toggle"),
        ("balanced (Md)", Some(MoonSize::Md), "Medium toggle"),
        ("safe", None, "Active density"),
    ];

    card("Toggle size overrides / density", cx).child(
        h_flex()
            .gap(px(14.0))
            .flex_wrap()
            .children(
                CASES
                    .into_iter()
                    .enumerate()
                    .map(|(index, (label, size, description))| {
                        MoonToggle::new(SharedString::from(format!("toggles-density-{index}")))
                            .label(label)
                            .description(description)
                            .when_some(size, |toggle, size| toggle.size(size))
                            .default_checked(index == 1)
                            // The last case shows that a disabled toggle ignores clicks.
                            .disabled(index == 2)
                    }),
            ),
    )
}

impl Gallery {
    /// Renders the Toggles page.
    ///
    /// Args:
    ///     cx: Gallery context used for theme lookup and interaction listeners.
    ///
    /// Returns:
    ///     The complete toggles section.
    pub(super) fn render_toggles(&self, cx: &mut Context<Self>) -> impl IntoElement {
        // The matrices lead the page so the default variant is what the page snapshot opens on.
        section("Toggles", cx)
            .children(
                MATRICES.into_iter().map(|(variant, key, size, title)| {
                    toggle_matrix(variant, key, size, title, cx)
                }),
            )
            .child(
                h_flex()
                    .items_start()
                    .gap(px(12.0))
                    .child(label_side_card(cx))
                    .child(self.render_live_toggles(cx)),
            )
            .child(text_edge_cases_card(cx))
            .child(density_card(cx))
    }

    /// Renders controlled toggles whose values live in the gallery state and are reported to the
    /// event log.
    fn render_live_toggles(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let toggle = |id: &'static str, size: MoonSize, checked: bool| {
            let view = view.clone();
            MoonToggle::new(id)
                .size(size)
                .label(format!("Alerts ({size:?})"))
                .description("Held in gallery state")
                .checked(checked)
                .on_change(move |checked, _, app| {
                    let checked = *checked;
                    view.update(app, |this, cx| {
                        match size {
                            MoonSize::Sm => this.toggle_sm_checked = checked,
                            _ => this.toggle_md_checked = checked,
                        }
                        this.push_event(format!("MoonToggle {size:?}: {checked}"), cx);
                    });
                })
        };

        card("Live toggles", cx)
            .child(caption("Values held by the gallery", cx).render())
            .child(toggle(
                "toggles-live-sm",
                MoonSize::Sm,
                self.toggle_sm_checked,
            ))
            .child(toggle(
                "toggles-live-md",
                MoonSize::Md,
                self.toggle_md_checked,
            ))
    }
}
