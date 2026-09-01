use gpui::prelude::FluentBuilder;
use gpui::*;
use regex::Regex;

use super::{
    foundation::v_flex,
    input::{MoonInput, MoonInputEvent, MoonInputState},
    popover::{MoonPopover, MoonPopoverPlacement},
    text::MoonText,
    theme::MoonTheme,
    tokens::{MoonPalette, MoonTone, rgba_from},
};

/// Cap on remembered custom colours (most-recent-first). A larger stored/seeded list is silently
/// trimmed on the next `custom_colors`/`set_custom_colors` call — callers that persist a longer
/// list would see it quietly shrink, so keep any persisted cap at or below this one.
const MAX_CUSTOM_COLORS: usize = 20;

/// Height budget for the swatch grid, in design-reference (unscaled) units — roughly 5 rows at
/// the current swatch size/gap. `MoonPopover` renders its content at intrinsic height with no cap
/// of its own (`popover.rs`), so a caller-supplied palette plus up to `MAX_CUSTOM_COLORS` custom
/// entries (e.g. the Badges tab's 65-swatch palette: up to 17 rows) can otherwise run the popup
/// off the bottom of the window in a long scrolling settings list. The grid scrolls past this
/// budget instead of growing without limit.
const GRID_MAX_HEIGHT_UI: f32 = 150.0;

/// Charset gate for the hex field, applied per keystroke via `MoonInputState::pattern`.
///
/// Deliberately permissive (any prefix of 0-6 hex digits, optional leading `#`) rather than a
/// full `#RRGGBB` match: a full-value pattern would reject every partial keystroke while typing
/// (e.g. `#F`), which is not what `is_valid_input` is for. Full-value validation happens only at
/// commit, in [`parse_hex_rgb`].
const HEX_CHARSET: &str = r"^#?[0-9a-fA-F]{0,6}$";

/// Rounding (not truncating) sRGB bytes for a picker colour.
///
/// The single source both `hex_label` and [`parse_hex_rgb`] go through, so the two are exact
/// inverses of each other. The naive `u32::from(rgba) >> 8` conversion this replaces truncates
/// instead of rounding, which used to only mis-label a swatch by up to 1/255 — harmless while the
/// label was read-only, but a value that round-trips through an editable field on every commit
/// must not drift.
fn rgb8_of(color: Hsla) -> [u8; 3] {
    let c: Rgba = color.into();
    [
        (c.r * 255.0).round() as u8,
        (c.g * 255.0).round() as u8,
        (c.b * 255.0).round() as u8,
    ]
}

/// Parse a committed hex field value into a colour.
///
/// Accepts `#RRGGBB` or bare `RRGGBB`, case-insensitive, with surrounding whitespace trimmed.
/// Deliberately rejects an 8-digit `#RRGGBBAA` (unlike the base Longbridge picker): this widget's
/// readout and every consumer are 6-digit/no-alpha, so an accepted 8-digit value could not
/// round-trip back through `hex_label`.
fn parse_hex_rgb(text: &str) -> Option<Hsla> {
    let text = text.trim();
    let digits = text.strip_prefix('#').unwrap_or(text);
    if digits.len() != 6 || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let value = u32::from_str_radix(digits, 16).ok()?;
    Some(rgb(value).into())
}

/// Push a newly-committed colour to the front of a most-recent-first custom-colour list.
///
/// De-duplicates by [`rgb8_of`] (a re-typed colour moves to the front rather than appearing
/// twice) and caps the list at [`MAX_CUSTOM_COLORS`], dropping the oldest entries.
///
/// Returns:
///     Whether the list's content or order actually changed — `false` when `color` was already
///     the front entry, so a caller mirroring this list into persisted state does not write on a
///     no-op commit.
fn push_custom(list: &mut Vec<Hsla>, color: Hsla) -> bool {
    let bytes = rgb8_of(color);
    let already_front = list.first().map(|c| rgb8_of(*c) == bytes).unwrap_or(false);
    if already_front {
        return false;
    }
    list.retain(|c| rgb8_of(*c) != bytes);
    list.insert(0, color);
    list.truncate(MAX_CUSTOM_COLORS);
    true
}

/// Seed/replace `list` from an already most-recent-first ORDERED source (a persisted history, or
/// another picker's current `custom()`), preserving that order.
///
/// `push_custom` inserts one colour at the front, so feeding it a MRU-ordered sequence
/// front-to-back would reverse it (the first, newest element ends up pushed deepest). Processing
/// the source in REVERSE — oldest first — restores the intended order: each push moves the
/// correct next-newest colour back to the front.
fn push_all_custom(list: &mut Vec<Hsla>, colors: impl IntoIterator<Item = Hsla>) {
    let ordered: Vec<Hsla> = colors.into_iter().collect();
    for color in ordered.into_iter().rev() {
        push_custom(list, color);
    }
}

pub enum MoonColorPickerEvent {
    Change(Hsla),
    /// A colour typed into the hex field and committed, newly added to (or moved to the front
    /// of) `custom`. Callers that persist a reuse palette react to this; others may ignore it.
    CustomAdded(Hsla),
}

pub struct MoonColorPickerState {
    value: Hsla,
    /// Colours committed through the hex field, most-recent-first. Session-local unless a caller
    /// seeds/persists it via `custom_colors`/`set_custom_colors` and `CustomAdded`.
    custom: Vec<Hsla>,
    /// Owned rather than transient `window.use_keyed_state`: reacting to its `PressEnter`/`Blur`
    /// needs `cx.subscribe_in`, which only a `Context<Self>` can set up — never available inside
    /// the stateless `MoonColorPicker::render`.
    hex_input: Entity<MoonInputState>,
    /// `default_value`/`set_value` cannot write `hex_input`'s displayed text directly — neither
    /// has a `Window` in hand. They raise this flag instead; `MoonColorPicker::render` drains it
    /// via `sync_hex_input` on every render, where a `Window` is available.
    hex_dirty: bool,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<MoonColorPickerEvent> for MoonColorPickerState {}

impl MoonColorPickerState {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let value = rgb(MoonPalette::active(cx).blue).into();
        let hex_input = cx.new(|cx| {
            MoonInputState::new(window, cx)
                .pattern(Regex::new(HEX_CHARSET).expect("HEX_CHARSET is a valid regex"))
        });
        let _subscriptions = vec![cx.subscribe_in(&hex_input, window, Self::on_hex_event)];
        Self {
            value,
            custom: Vec::new(),
            hex_input,
            hex_dirty: true,
            _subscriptions,
        }
    }

    pub fn default_value(mut self, value: Hsla) -> Self {
        self.value = value;
        self.hex_dirty = true;
        self
    }

    /// Seed the reuse palette shown in the popover (most-recent-first), e.g. from persisted
    /// config. Runs each entry through the same dedupe/cap as a live hex commit.
    pub fn custom_colors(mut self, colors: impl IntoIterator<Item = Hsla>) -> Self {
        push_all_custom(&mut self.custom, colors);
        self
    }

    /// Replace the reuse palette post-construction (e.g. after a sibling picker's hex commit was
    /// persisted and fanned out to this one). Normalizes through the same dedupe/cap; emits no
    /// event — this is the caller pushing state IN, not the widget producing a new colour.
    pub fn set_custom_colors(
        &mut self,
        colors: impl IntoIterator<Item = Hsla>,
        cx: &mut Context<Self>,
    ) {
        self.custom.clear();
        push_all_custom(&mut self.custom, colors);
        cx.notify();
    }

    /// The current reuse palette, most-recent-first.
    pub fn custom(&self) -> &[Hsla] {
        &self.custom
    }

    pub fn value(&self) -> Hsla {
        self.value
    }

    fn set_value(&mut self, value: Hsla, cx: &mut Context<Self>) {
        self.hex_dirty = true;
        if self.value == value {
            return;
        }
        self.value = value;
        cx.emit(MoonColorPickerEvent::Change(value));
        cx.notify();
    }

    /// React to a committed hex-field edit (Enter or blur). The one validation: a value that
    /// parses becomes the picker's value and joins the reuse palette; a value that does not
    /// parse is rejected WITHOUT touching `self.value`/`self.custom` — only the field's own
    /// displayed text reverts (via the `hex_dirty` flag) to whatever colour is still live.
    fn on_hex_event(
        &mut self,
        input: &Entity<MoonInputState>,
        event: &MoonInputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !matches!(
            event,
            MoonInputEvent::PressEnter { .. } | MoonInputEvent::Blur
        ) {
            return;
        }
        let text = input.read(cx).value();
        match parse_hex_rgb(&text) {
            Some(color) => {
                let added = push_custom(&mut self.custom, color);
                self.set_value(color, cx);
                if added {
                    cx.emit(MoonColorPickerEvent::CustomAdded(color));
                }
            }
            None => self.hex_dirty = true,
        }
        self.sync_hex_input(window, cx);
        cx.notify();
    }

    /// Write the live value's hex text into the field when `hex_dirty` is set — after a
    /// `default_value`/`set_value` call, or after a rejected commit reverts the draft.
    fn sync_hex_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.hex_dirty {
            return;
        }
        self.hex_dirty = false;
        let text = hex_label(self.value);
        self.hex_input
            .update(cx, |state, cx| state.set_value(text, window, cx));
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
}

/// Render a colour as the `#RRGGBB` text the hex field shows and [`parse_hex_rgb`] accepts back.
fn hex_label(color: Hsla) -> SharedString {
    let [r, g, b] = rgb8_of(color);
    SharedString::from(format!("#{r:02X}{g:02X}{b:02X}"))
}

impl RenderOnce for MoonColorPicker {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Drain a pending `default_value`/rejected-commit sync before reading anything else, so
        // the trigger label and the hex field never show stale text for one frame.
        self.state
            .update(cx, |state, cx| state.sync_hex_input(window, cx));

        let p = MoonPalette::active(cx);
        let tokens = MoonTheme::active_tokens(cx);
        let value = self.state.read(cx).value();
        let custom = self.state.read(cx).custom().to_vec();
        let state = self.state.clone();
        let fixed = if self.colors.is_empty() {
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
        // Custom (typed) colours lead, so a just-committed hex is immediately reachable without
        // re-typing; a fixed swatch already covered by a custom one is skipped rather than shown
        // twice.
        let custom_bytes: Vec<[u8; 3]> = custom.iter().map(|c| rgb8_of(*c)).collect();
        let colors: Vec<Hsla> = custom
            .into_iter()
            .chain(
                fixed
                    .into_iter()
                    .filter(|c| !custom_bytes.contains(&rgb8_of(*c))),
            )
            .collect();

        let trigger = div()
            .id(ElementId::from(SharedString::from(format!(
                "{}:trigger",
                self.id
            ))))
            .h(px(tokens.fit_height(26.0, 13.0, 6.5)))
            .w(px(128.0))
            .rounded(px(tokens.ui(4.0)))
            .border(px(1.0))
            .border_color(rgba_from(p.border, 1.0))
            .bg(rgba_from(
                p.shell_high,
                if self.disabled { 0.55 } else { 1.0 },
            ))
            .flex()
            .items_center()
            .gap(px(tokens.ui(8.0)))
            .px(px(tokens.ui(7.0)))
            .cursor_default()
            .when(!self.disabled, |this| {
                this.hover(move |this| this.border_color(rgba_from(p.border_hover, 1.0)))
            })
            .child(
                div()
                    .size(px(tokens.ui(14.0)))
                    .rounded(px(tokens.ui(3.0)))
                    .border(px(1.0))
                    .border_color(rgba_from(p.shadow, 0.38))
                    .bg(value),
            )
            .child(
                MoonText::new(hex_label(value))
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
            .gap(px(tokens.ui(6.0)))
            .max_h(px(tokens.ui(GRID_MAX_HEIGHT_UI)))
            .overflow_y_scroll();

        for (ix, color) in colors.into_iter().enumerate() {
            let state = state.clone();
            grid = grid.child(
                div()
                    .id(ElementId::from(SharedString::from(format!(
                        "{}:color:{ix}",
                        self.id
                    ))))
                    .size(px(tokens.ui(22.0)))
                    .rounded(px(tokens.ui(4.0)))
                    .border(px(1.0))
                    .border_color(if color == value {
                        rgba_from(p.blue, 1.0)
                    } else {
                        rgba_from(p.shadow, 0.40)
                    })
                    .bg(color)
                    .when(!self.disabled, |this| {
                        this.hover(|this| {
                            this.border_color(rgba_from(p.text, 0.78)).shadow(vec![
                                super::foundation::box_shadow(
                                    px(0.0),
                                    px(0.0),
                                    px(tokens.ui(10.0)),
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

        let hex_input_state = self.state.read(cx).hex_input.clone();
        let hex_is_invalid = {
            let text = hex_input_state.read(cx).value();
            !text.is_empty() && parse_hex_rgb(&text).is_none()
        };
        let hex_row = div().w_full().child(
            MoonInput::new(SharedString::from(format!("{}:hex", self.id)))
                .state(&hex_input_state)
                .small()
                .mono(true)
                .disabled(self.disabled)
                .when(hex_is_invalid, |this| this.tone(MoonTone::Danger)),
        );

        let content = v_flex().gap(px(tokens.ui(6.0))).child(hex_row).child(grid);

        MoonPopover::new(self.id)
            .trigger(trigger)
            .content(content)
            .width(156.0)
            .placement(MoonPopoverPlacement::BottomStart)
            .disabled(self.disabled)
    }
}

#[cfg(test)]
mod tests;
