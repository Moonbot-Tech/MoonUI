use std::time::{Duration, Instant};

use gpui::prelude::FluentBuilder;
use gpui::*;

use crate::checkbox::{ChoiceColors, MoonCheckboxMetrics, choice_text_column};

use super::{
    colors::MoonColors,
    foundation::{MoonSize, moon_cubic_bezier, moon_shadow_sm},
    theme::{MoonTheme, MoonThemeTokens},
    tokens::{MoonPalette, MoonRect, MoonTone, rgba_from},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoonToggleLabelSide {
    Left,
    Right,
}

/// A toggle's visual type. The look of each is the component's own; a variant never changes
/// what a toggle does or how it reports a change.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MoonToggleVariant {
    /// The reviewed toggle: a track one text line tall with the thumb inside it.
    #[default]
    Default,
    /// A slimmer toggle: a shorter track behind the same text, with a thumb as tall as the track
    /// itself that carries a 1px outline of its own. `Sm` is a 32x16 track under a 16px thumb and
    /// `Md` a 40x20 track under a 20px thumb, so the thumb rides the track rather than sitting
    /// inside it.
    Slim,
}

impl MoonToggleVariant {
    /// This variant's track, thumb and outline at `tier`, in unscaled design-reference pixels.
    ///
    /// The gap from the thumb to the track's outer edge is whatever the two sizes leave: 2px on
    /// the default toggle, none at all on a slim one, whose thumb is the height of its track.
    ///
    /// Returns:
    ///     The track's width and height, the thumb's size, and the outline's width.
    fn track(self, tier: MoonSize) -> (f32, f32, f32, f32) {
        match (self, tier) {
            (Self::Default, MoonSize::Sm) => (36.0, 20.0, 16.0, 0.5),
            (Self::Default, _) => (44.0, 24.0, 20.0, 0.5),
            // A slim thumb is exactly its track's height, as the design has it. The two then share
            // one silhouette at the end the thumb rests on, where their antialiased edges compound
            // into a harder, stepped curve; that is the design's trade for a thumb that fills the
            // track.
            (Self::Slim, MoonSize::Sm) => (32.0, 16.0, 16.0, 1.0),
            (Self::Slim, _) => (40.0, 20.0, 20.0, 1.0),
        }
    }

    /// Whether the thumb carries an outline of its own.
    ///
    /// A slim toggle's thumb rides over the track, so it is ringed to stay legible against it; the
    /// default toggle's thumb sits inside its track and stays a plain disc.
    fn rings_thumb(self) -> bool {
        match self {
            Self::Default => false,
            Self::Slim => true,
        }
    }

    /// Whether the track clips the thumb and its shadow.
    ///
    /// The default toggle's thumb sits inside its track, so clipping keeps the shadow off the
    /// surface around it. A slim toggle's thumb is as tall as its track and rides over its
    /// outline, so clipping there would flatten the thumb's own shadow against the track it sits
    /// on; it draws unclipped instead.
    fn clips_thumb(self) -> bool {
        match self {
            Self::Default => true,
            Self::Slim => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MoonToggleSize {
    /// A tier of the shared size scale. Toggles come in the checkbox's tiers with the same text and
    /// spacing; `Xs` renders as `Sm`, and `Lg` and above render as `Md`. The track is the variant's
    /// own: `Sm` is 36x20 by default and 32x16 slim, `Md` 44x24 and 40x20.
    Tier(MoonSize),
    Custom {
        track_width: f32,
        track_height: f32,
        thumb_size: f32,
        font_size: f32,
        line_height: f32,
        gap: f32,
    },
}

impl From<MoonSize> for MoonToggleSize {
    fn from(size: MoonSize) -> Self {
        Self::Tier(size)
    }
}

/// Metrics for a `MoonToggle` size, in unscaled design-reference pixels until `resolve`.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct MoonToggleMetrics {
    pub track_width: f32,
    pub track_height: f32,
    pub thumb_size: f32,
    pub font_size: f32,
    pub line_height: f32,
    pub label_weight: f32,
    pub description_weight: f32,
    pub gap: f32,
    /// Space between the label and the description under it.
    pub description_gap: f32,
    /// Width of the variant's outline, drawn inside whichever of the track or the thumb carries
    /// it, so it never adds to that element's size.
    pub border: f32,
    /// Gap from the track edge to the focus ring's outer edge; the ring's stroke lies inside it.
    pub focus_ring_distance: f32,
    pub focus_ring_width: f32,
}

impl MoonToggleSize {
    pub const SUPPORTED_TIERS: [MoonSize; 2] = [MoonSize::Sm, MoonSize::Md];

    /// The app's density tier, snapped to a tier this component supports.
    pub fn density_default(tokens: &MoonThemeTokens) -> Self {
        Self::Tier(tokens.tier().nearest(&Self::SUPPORTED_TIERS))
    }

    /// This size's metrics for the default variant, in unscaled design-reference pixels.
    pub fn reference_metrics(self) -> MoonToggleMetrics {
        self.reference_metrics_for(MoonToggleVariant::Default)
    }

    /// This size's metrics as `variant` draws them, in unscaled design-reference pixels.
    ///
    /// A variant owns the track, the thumb and the outline; the text is the tier's own, so a slim
    /// toggle carries the same label, supporting text and spacing as the toggle beside it.
    pub fn reference_metrics_for(self, variant: MoonToggleVariant) -> MoonToggleMetrics {
        match self {
            Self::Tier(t) => {
                let t = t.nearest(&Self::SUPPORTED_TIERS);
                let c = t.control_metrics();
                let (track_width, track_height, thumb_size, border) = variant.track(t);
                MoonToggleMetrics {
                    track_width,
                    track_height,
                    thumb_size,
                    font_size: c.font_size,
                    line_height: c.line_height,
                    label_weight: 500.0,
                    description_weight: 400.0,
                    gap: c.gap,
                    description_gap: if t == MoonSize::Sm { 0.0 } else { 2.0 },
                    border,
                    focus_ring_distance: 4.0,
                    focus_ring_width: 2.0,
                }
            }
            Self::Custom {
                track_width,
                track_height,
                thumb_size,
                font_size,
                line_height,
                gap,
            } => MoonToggleMetrics {
                track_width,
                track_height,
                thumb_size,
                font_size,
                line_height,
                label_weight: 400.0,
                description_weight: 400.0,
                gap,
                description_gap: 0.0,
                // A custom size sets its own track, so only the outline follows the variant.
                border: variant.track(MoonSize::Md).3,
                focus_ring_distance: 4.0,
                focus_ring_width: 2.0,
            },
        }
    }

    /// Metrics as rendered: a tier follows the UI zoom only, `Custom` also follows text scaling.
    fn resolve(self, variant: MoonToggleVariant, tokens: &MoonThemeTokens) -> MoonToggleMetrics {
        let reference = self.reference_metrics_for(variant);
        match self {
            Self::Tier(_) => reference.zoomed(tokens),
            Self::Custom { .. } => reference.scaled(tokens),
        }
    }
}

impl MoonToggleMetrics {
    /// Scales every length by the UI zoom alone, text included.
    fn zoomed(self, tokens: &MoonThemeTokens) -> Self {
        let ui = |value: f32| tokens.ui(value);
        Self {
            track_width: ui(self.track_width),
            track_height: ui(self.track_height),
            thumb_size: ui(self.thumb_size),
            font_size: ui(self.font_size),
            line_height: ui(self.line_height),
            label_weight: self.label_weight,
            description_weight: self.description_weight,
            gap: ui(self.gap),
            description_gap: ui(self.description_gap),
            border: ui(self.border),
            focus_ring_distance: ui(self.focus_ring_distance),
            focus_ring_width: ui(self.focus_ring_width),
        }
    }

    /// Scales geometry by the UI zoom and text by the theme's text scaling.
    ///
    /// Unlike the checkbox's `scaled`, this does not grow the track with `font_delta` — a track is
    /// not text-bound.
    fn scaled(self, tokens: &MoonThemeTokens) -> Self {
        Self {
            track_width: tokens.ui(self.track_width),
            track_height: tokens.ui(self.track_height),
            thumb_size: tokens.ui(self.thumb_size),
            font_size: tokens.font(self.font_size),
            line_height: tokens.line_height(self.line_height),
            label_weight: self.label_weight,
            description_weight: self.description_weight,
            gap: tokens.ui(self.gap),
            description_gap: tokens.ui(self.description_gap),
            border: tokens.ui(self.border),
            focus_ring_distance: tokens.ui(self.focus_ring_distance),
            focus_ring_width: tokens.ui(self.focus_ring_width),
        }
    }

    /// Returns these metrics as checkbox metrics with the track as the box, so the toggle's text
    /// lays out exactly as a checkbox's or radio's does. Only the box's height takes part in that
    /// layout; the mark fields are the thumb's size and stay unused.
    fn choice(self) -> MoonCheckboxMetrics {
        MoonCheckboxMetrics {
            box_size: px(self.track_height),
            font_size: px(self.font_size),
            line_height: px(self.line_height),
            label_weight: FontWeight(self.label_weight),
            description_weight: FontWeight(self.description_weight),
            gap: px(self.gap),
            description_gap: px(self.description_gap),
            radius: px(self.track_height * 0.5),
            focus_ring_distance: px(self.focus_ring_distance),
            focus_ring_width: px(self.focus_ring_width),
            mark_size: px(self.thumb_size),
            mark_stroke: None,
        }
    }
}

/// How long the thumb takes to travel from one end of the track to the other.
const THUMB_TRAVEL: Duration = Duration::from_millis(150);

/// The curve the thumb travels on: out of the old end quickly, easing into the new one.
const THUMB_TRAVEL_CURVE: [f32; 4] = [0.4, 0.0, 0.2, 1.0];

#[derive(Default)]
struct MoonToggleState {
    checked: bool,
}

/// Where a toggle's thumb is drawn, and whether it is on its way there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ThumbTravel {
    /// Resting at one end, drawn there with no animation. A toggle first rendered checked shows
    /// its thumb at the checked end at once, rather than sliding in from the other.
    Resting(bool),
    /// Travelling to `to` after the toggle changed, having started from `from` at `since`.
    ///
    /// The state stays here once the thumb arrives, so the animation runs to its end instead of
    /// being cut off by the next render; `since` is what tells the two apart.
    Travelling {
        from: bool,
        to: bool,
        since: Instant,
    },
}

impl ThumbTravel {
    /// Returns the state of a toggle first rendered at `checked`.
    fn initial(checked: bool) -> Self {
        Self::Resting(checked)
    }

    /// Returns the state after a render at `checked`.
    ///
    /// A toggle that changes starts travelling; one that changes again mid-flight turns around
    /// from the end it was heading for. A travelling thumb keeps that state once it arrives, so
    /// the animation runs to the end instead of being cut off by the next render.
    fn next(self, checked: bool) -> Self {
        match self {
            Self::Resting(at) if at == checked => self,
            Self::Resting(at) => Self::Travelling {
                from: at,
                to: checked,
                since: Instant::now(),
            },
            Self::Travelling { to, .. } if to == checked => self,
            Self::Travelling { to, .. } => Self::Travelling {
                from: to,
                to: checked,
                since: Instant::now(),
            },
        }
    }

    /// Whether the thumb has arrived, which is what lets the track give its cap up to a thumb that
    /// has come to a stop on it.
    fn settled(self) -> bool {
        match self {
            Self::Resting(_) => true,
            Self::Travelling { since, .. } => since.elapsed() >= THUMB_TRAVEL,
        }
    }

    /// The end the thumb is travelling from, or `None` when it is at rest.
    fn from(self) -> Option<bool> {
        match self {
            Self::Resting(_) => None,
            Self::Travelling { from, .. } => Some(from),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MoonToggleClickPlan {
    next_checked: bool,
    update_internal: bool,
}

fn moon_toggle_click_plan(
    checked: bool,
    controlled: bool,
    disabled: bool,
) -> Option<MoonToggleClickPlan> {
    if disabled {
        None
    } else {
        Some(MoonToggleClickPlan {
            next_checked: !checked,
            update_internal: !controlled,
        })
    }
}

#[derive(IntoElement)]
pub struct MoonToggle {
    id: SharedString,
    bounds: Option<MoonRect>,
    label: Option<SharedString>,
    description: Option<SharedString>,
    label_side: MoonToggleLabelSide,
    variant: MoonToggleVariant,
    checked: Option<bool>,
    default_checked: bool,
    disabled: bool,
    size: Option<MoonToggleSize>,
    tone: Option<MoonTone>,
    mono: bool,
    label_color: Option<u32>,
    on_change: Option<std::rc::Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
}

impl MoonToggle {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            label: None,
            description: None,
            label_side: MoonToggleLabelSide::Right,
            variant: MoonToggleVariant::Default,
            checked: None,
            default_checked: false,
            disabled: false,
            size: None,
            tone: None,
            mono: false,
            label_color: None,
            on_change: None,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets supporting text shown under the label in the muted text colour.
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn label_side(mut self, side: MoonToggleLabelSide) -> Self {
        self.label_side = side;
        self
    }

    /// Sets the visual type: the default toggle, or `Slim`.
    pub fn variant(mut self, variant: MoonToggleVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }

    pub fn default_checked(mut self, checked: bool) -> Self {
        self.default_checked = checked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets the size: a tier such as `MoonSize::Sm`, or a `MoonToggleSize::Custom`.
    pub fn size(mut self, size: impl Into<MoonToggleSize>) -> Self {
        self.size = Some(size.into());
        self
    }

    /// Overrides the label's ink. The description keeps its colour; disabled alpha still applies.
    pub fn label_color(mut self, color: u32) -> Self {
        self.label_color = Some(color);
        self
    }

    /// Fills a checked track with `tone` instead of the brand colour. The tone has no hover
    /// colour of its own, so a toned track keeps its fill under the pointer.
    pub fn tone(mut self, tone: MoonTone) -> Self {
        self.tone = Some(tone);
        self
    }

    pub fn mono(mut self, mono: bool) -> Self {
        self.mono = mono;
        self
    }

    pub fn on_change(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(std::rc::Rc::new(handler));
        self
    }
}

impl RenderOnce for MoonToggle {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state_id = ElementId::from(self.id.clone());
        let state = window.use_keyed_state(state_id, cx, |_, _| MoonToggleState {
            checked: self.default_checked,
        });

        let tokens = MoonTheme::active_tokens(cx);
        let size = self
            .size
            .unwrap_or_else(|| MoonToggleSize::density_default(&tokens));
        let m = size.resolve(self.variant, &tokens);
        let choice = m.choice();
        // The outline is drawn inside the track, so it never adds to the track's size.
        let border = m.border;
        // The track carries no border of its own now, so the ring sits its own distance outside the
        // track's box.
        let inset = -m.focus_ring_distance;
        let p = tokens.palette;
        let checked = self.checked.unwrap_or_else(|| state.read(cx).checked);
        let disabled = self.disabled;
        // The focus ring keeps the tone accent it has always drawn, Info where no tone is set,
        // until the ring has a colour of its own in the design.
        let ring_accent = self.tone.unwrap_or(MoonTone::Info).color(p);
        let parent_view = window.current_view();
        let track_width = m.track_width;
        let track_height = m.track_height;
        let thumb_size = m.thumb_size;
        // The thumb is placed against the track's outer edge, inside a layer that spans it, so the
        // same gap holds whatever the outline's width: 2px on a default toggle, none on a slim one
        // whose thumb is as tall as its track.
        let thumb_gap = (track_height - thumb_size) * 0.5;
        let thumb_left_at = |checked: bool| {
            if checked {
                track_width - thumb_size - thumb_gap
            } else {
                thumb_gap
            }
        };
        let thumb_left = thumb_left_at(checked);
        let roles = MoonColors::active(cx);
        let colors = ToggleColors::resolve(p, roles, self.variant, self.tone, checked, disabled);
        // The label and supporting text take the checkbox's and radio's text colours.
        let text_colors = ChoiceColors::resolve(p, roles, None, checked, disabled);
        let focus_handle = window
            .use_keyed_state(
                ElementId::from(SharedString::from(format!("{}:focus", self.id))),
                cx,
                |_, cx| cx.focus_handle().tab_stop(true),
            )
            .read(cx)
            .clone();
        let is_focused = focus_handle.is_focused(window);
        let label_color = match self.label_color {
            Some(color) => rgba_from(color, if disabled { 0.45 } else { 1.0 }),
            None => text_colors.label,
        };
        let track_selector = format!("{}:track", self.id);
        let thumb_selector = format!("{}:thumb", self.id);
        let thumb_face_selector = format!("{}:thumb-face", self.id);
        let focus_ring_selector = format!("{}:focus-ring", self.id);
        // The whole row is the hit area, so the track takes its hover fill from a pointer anywhere
        // on the row, its label included, rather than only over the track itself.
        let hover_group = SharedString::from(format!("{}:row", self.id));
        let track_hover = colors.track_hover;
        let border_hover = colors.border_hover;
        let thumb_border_hover = colors.thumb_border_hover;
        // Empty text counts as no text, so a bare toggle never gains the text column and its gap.
        let label = self.label.filter(|label| !label.is_empty());
        let description = self
            .description
            .filter(|description| !description.is_empty());
        let has_text = label.is_some() || description.is_some();

        // A thumb that has just changed ends slides between them; one that has not is drawn where
        // it belongs. The travel state keeps the animation alive until the thumb arrives, and the
        // element id carries the end it is heading for, so changing again turns it around instead
        // of continuing the old slide.
        let travel = window.use_keyed_state(
            ElementId::from(SharedString::from(format!("{}:travel", self.id))),
            cx,
            |_, _| ThumbTravel::initial(checked),
        );
        let previous = *travel.read(cx);
        let current = previous.next(checked);
        if current != previous {
            travel.update(cx, |travel, _| *travel = current);
        }
        // A disabled toggle mixes its colours into the surface rather than painting them
        // translucent, so the shadow is the one part still faded: it is translucent by nature.
        let mut thumb_shadow = moon_shadow_sm(roles, &tokens);
        if disabled {
            for layer in &mut thumb_shadow {
                layer.color = layer.color.opacity(ToggleColors::DISABLED_ALPHA);
            }
        }
        // A ringed thumb is one filled disc in the ring's colour with a face laid on top, inset by
        // the ring's width. Drawing the ring as a border instead leaves the disc's own edge and the
        // stroke on one curve: GPUI's quad shader mixes the fill into a 1px stroke wherever its
        // inner and outer bands meet, which thins the ring and steps it around the circle.
        let rings_thumb = self.variant.rings_thumb();
        let ring = if rings_thumb { border } else { 0.0 };
        let thumb = div()
            .debug_selector(|| thumb_selector)
            .absolute()
            .left(px(thumb_left))
            .top(px(thumb_gap))
            .w(px(thumb_size))
            .h(px(thumb_size))
            .rounded(px(thumb_size * 0.5))
            .bg(if rings_thumb {
                colors.thumb_border
            } else {
                colors.thumb
            })
            .when(!disabled && rings_thumb, |this| {
                this.group_hover(hover_group.clone(), move |this| this.bg(thumb_border_hover))
            })
            .shadow(thumb_shadow)
            .when(rings_thumb, |this| {
                this.child(
                    div()
                        .debug_selector(|| thumb_face_selector)
                        .absolute()
                        .top(px(ring))
                        .left(px(ring))
                        .right(px(ring))
                        .bottom(px(ring))
                        .rounded(px((thumb_size - ring * 2.0) * 0.5))
                        .bg(colors.thumb),
                )
            });
        let thumb = match current.from() {
            None => thumb.into_any_element(),
            Some(from) => {
                let from_left = thumb_left_at(from);
                let travelled = thumb_left - from_left;
                let [x1, y1, x2, y2] = THUMB_TRAVEL_CURVE;
                thumb
                    .with_animation(
                        ElementId::from(SharedString::from(format!(
                            "{}:travel:{checked}",
                            self.id
                        ))),
                        Animation::new(THUMB_TRAVEL).with_easing(moon_cubic_bezier(x1, y1, x2, y2)),
                        move |thumb, delta| thumb.left(px(from_left + travelled * delta)),
                    )
                    .into_any_element()
            }
        };

        // A thumb the height of its track has the track's own cap for a silhouette. Painting both
        // lays two antialiased edges on one curve, whose coverage compounds into a stepped edge, so
        // the rail stops at the thumb's centre with a square end and the thumb completes the pill:
        // the thumb's disc is exactly the cap the rail gives up, and it hides the square end. A
        // travelling thumb sits away from both caps, so the rail spans the track whole.
        let radius = track_height * 0.5;
        // A travelling thumb sits away from both caps, so the rail spans the track whole until it
        // arrives. The frame requested here is the one that re-cuts the rail once it has.
        let settled = current.settled();
        if !settled {
            window.request_animation_frame();
        }
        let rail_end =
            (settled && thumb_size >= track_height).then(|| thumb_left + thumb_size * 0.5);
        let (rail_left, rail_width) = match rail_end {
            Some(end) if checked => (0.0, end),
            Some(end) => (end, track_width - end),
            None => (0.0, track_width),
        };
        let (rail_left_radius, rail_right_radius) = match rail_end {
            Some(_) if checked => (radius, 0.0),
            Some(_) => (0.0, radius),
            None => (radius, radius),
        };

        let switch = div()
            .relative()
            .debug_selector(|| track_selector)
            .flex_shrink_0()
            .when(has_text, |this| this.mt(choice.box_offset()))
            .w(px(track_width))
            .h(px(track_height))
            // The rail carries the track's fill and outline. It is a child rather than the track
            // itself so that its own border never insets the layers placed against the track's box,
            // and so that it can give up the cap the thumb draws.
            .child(
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(px(rail_left))
                    .w(px(rail_width))
                    .rounded_tl(px(rail_left_radius))
                    .rounded_bl(px(rail_left_radius))
                    .rounded_tr(px(rail_right_radius))
                    .rounded_br(px(rail_right_radius))
                    .bg(colors.track)
                    .border(px(border))
                    .border_color(colors.border)
                    .when(!disabled, |this| {
                        this.group_hover(hover_group.clone(), move |this| {
                            this.bg(track_hover).border_color(border_hover)
                        })
                    }),
            )
            // The thumb rides in a layer spanning the track. It is its own child rather than a clip
            // on the track, because the focus ring hangs outside the track and must not be clipped
            // with it; whether the layer clips at all is the variant's to say.
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .bottom_0()
                    .rounded(px(track_height * 0.5))
                    .when(self.variant.clips_thumb(), |this| this.overflow_hidden())
                    .child(thumb),
            )
            .when(is_focused, |this| {
                this.child(
                    div()
                        .debug_selector(|| focus_ring_selector)
                        .absolute()
                        .top(px(inset))
                        .left(px(inset))
                        .right(px(inset))
                        .bottom(px(inset))
                        .border(px(m.focus_ring_width))
                        .border_color(rgba_from(ring_accent, 1.0))
                        .rounded(px(m.track_height * 0.5 + m.focus_ring_distance)),
                )
            });

        // The column starts at its content width and only shrinks, wrapping its text, instead of
        // filling the row, so a label on the left stays beside the track.
        let column = has_text.then(|| {
            choice_text_column(
                &self.id,
                choice,
                label,
                description,
                text_colors.description,
                Vec::new(),
            )
            .flex_initial()
        });

        let mut root = div()
            .id(ElementId::from(SharedString::from(format!(
                "{}:root",
                self.id
            ))))
            .group(hover_group.clone())
            .relative()
            .flex()
            .items_center()
            // Rows with text are top-aligned so the track stays on the first line.
            .when(has_text, |this| this.items_start())
            .gap(choice.gap)
            .text_size(choice.font_size)
            .text_color(label_color)
            .when(self.mono, |this| this.font_family(tokens.font_family(true)))
            .when(disabled, |this| this.cursor_default())
            // The row itself paints nothing: hovering or pressing a toggle leaves the surface
            // behind the track and its text alone, as it does on a checkbox or radio.
            .when(!disabled, |this| {
                this.track_focus(&focus_handle).cursor_pointer()
            });

        root = match self.label_side {
            MoonToggleLabelSide::Left => root.children(column).child(switch),
            MoonToggleLabelSide::Right => root.child(switch).children(column),
        };

        if let Some(bounds) = self.bounds {
            root = root
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }

        let focus_handle_for_click = focus_handle.clone();
        root = root.on_mouse_down(MouseButton::Left, move |_, window, cx| {
            cx.stop_propagation();
            if disabled {
                return;
            }
            focus_handle_for_click.focus(window, cx);
            window.prevent_default();
        });

        if !disabled {
            let controlled = self.checked.is_some();
            let on_change = self.on_change.clone();
            root = root.on_click(move |_, window, cx| {
                let Some(plan) = moon_toggle_click_plan(checked, controlled, disabled) else {
                    return;
                };
                if plan.update_internal {
                    state.update(cx, |state, _| {
                        state.checked = plan.next_checked;
                    });
                }
                if let Some(on_change) = on_change.as_ref() {
                    on_change(&plan.next_checked, window, cx);
                }
                cx.notify(parent_view);
            });
        }

        root
    }
}

/// The colours a toggle's track and thumb paint with.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ToggleColors {
    track: Hsla,
    /// The track under the pointer. Equal to `track` where the state has no hover colour of its
    /// own, which leaves the track unchanged on hover.
    track_hover: Hsla,
    /// The track's outline, transparent where the variant outlines its thumb instead.
    border: Hsla,
    /// The track's outline under the pointer. Equal to `border` where the state has no hover
    /// colour of its own, which leaves the outline unchanged on hover.
    border_hover: Hsla,
    thumb: Hsla,
    /// The thumb's own outline, transparent where the variant outlines its track instead.
    thumb_border: Hsla,
    /// The thumb's outline under the pointer, equal to `thumb_border` where nothing changes.
    thumb_border_hover: Hsla,
}

impl ToggleColors {
    /// How much of a disabled toggle's colour survives the mix into the surface behind it.
    const DISABLED_ALPHA: f32 = 0.5;

    /// Returns `color` as a disabled toggle paints it: the colour itself, mixed halfway into
    /// `surface`, and still opaque.
    ///
    /// A disabled toggle cannot simply be drawn translucent. GPUI fades each element on its own
    /// rather than the control as a whole, so a translucent thumb would show the track through it
    /// instead of fading with it, and the thumb would read as a tint of the track. Mixing the
    /// colours instead reproduces what the design shows for a control at half opacity, as long as
    /// the surface behind it is the page's; a toggle on a panel of another colour fades toward the
    /// page's colour rather than the panel's.
    ///
    /// A colour that paints nothing stays transparent, so an outline the variant leaves off does
    /// not come back as a surface-coloured ring.
    fn disabled(color: Hsla, surface: Hsla) -> Hsla {
        if color.a == 0.0 {
            return color;
        }
        let faded = Rgba::from(color).alpha(color.a * Self::DISABLED_ALPHA);
        Rgba::from(surface).blend(faded).into()
    }

    /// Resolves a toggle's colours from the theme's colour roles.
    ///
    /// Both variants fill their track the same way: `bg_tertiary` unchecked, `bg_brand_solid`
    /// checked and `bg_brand_solid_hover` under the pointer. An explicit tone replaces the brand
    /// fill with that tone, which has no hover colour of its own.
    ///
    /// The default toggle outlines an unchecked track in `border_secondary` and leaves a checked one
    /// to its fill alone, with a plain thumb and no hover change. A slim toggle outlines its track
    /// and its thumb in one colour per state: `border_secondary` while off, unchanged under the
    /// pointer, `toggle_slim_border_pressed` once on, and `toggle_slim_border_pressed_hover` when a
    /// checked one is hovered.
    ///
    /// The thumb is `fg_white` in both states, which is white on every bundled theme. A disabled
    /// toggle paints the same colours and dims the track, its outline and its thumb as one piece,
    /// exactly as a disabled checkbox dims its box.
    ///
    /// Args:
    ///     p: The active palette, for a tone's fill and the thumb's shadow.
    ///     roles: The active colour roles, normally `MoonColors::active`.
    ///     variant: The toggle's visual type, which owns the outline.
    ///     tone: The tone set on the toggle, or `None` for the brand fill.
    ///     checked: Whether the toggle is on.
    ///     disabled: Whether the toggle is disabled.
    ///
    /// Returns:
    ///     The colours to paint the track, its outline and the thumb with, and the opacity to
    ///     paint them at.
    fn resolve(
        p: MoonPalette,
        roles: MoonColors,
        variant: MoonToggleVariant,
        tone: Option<MoonTone>,
        checked: bool,
        disabled: bool,
    ) -> Self {
        let (checked_track, checked_track_hover) = match tone {
            Some(tone) => {
                let tone = rgba_from(tone.color(p), 1.0);
                (tone, tone)
            }
            None => (
                roles.bg_brand_solid.into(),
                roles.bg_brand_solid_hover.into(),
            ),
        };
        let unchecked = roles.bg_tertiary.into();
        // A default toggle's checked track is the brand fill alone; only an unchecked one is
        // outlined, and neither changes under the pointer. A slim toggle's outline is its thumb's:
        // `toggle_border` while off, the pressed roles once on, and only then does the pointer
        // change it.
        // A slim toggle draws one outline colour per state, around its track and its thumb alike, so
        // the thumb reads as part of the track it rides rather than as a disc laid over it. The
        // colour is `border_secondary` while the toggle is off and the pressed roles once it is on,
        // which is the only pair that answers to the pointer.
        let slim_outline: (Hsla, Hsla) = if checked {
            (
                roles.toggle_slim_border_pressed.into(),
                roles.toggle_slim_border_pressed_hover.into(),
            )
        } else {
            let border: Hsla = roles.border_secondary.into();
            (border, border)
        };
        let (border, border_hover, thumb_border, thumb_border_hover) = match variant {
            // The default toggle outlines an empty track in `border_secondary` and leaves a checked
            // one to its fill; its thumb sits inside the track and stays a plain disc.
            MoonToggleVariant::Default if checked => (
                transparent_black(),
                transparent_black(),
                transparent_black(),
                transparent_black(),
            ),
            MoonToggleVariant::Default => {
                let border: Hsla = roles.border_secondary.into();
                (border, border, transparent_black(), transparent_black())
            }
            MoonToggleVariant::Slim => (
                slim_outline.0,
                slim_outline.1,
                slim_outline.0,
                slim_outline.1,
            ),
        };
        let colors = Self {
            track: if checked { checked_track } else { unchecked },
            track_hover: if checked {
                checked_track_hover
            } else {
                unchecked
            },
            border,
            border_hover,
            thumb_border,
            thumb_border_hover,
            thumb: roles.fg_white.into(),
        };
        if !disabled {
            return colors;
        }
        let surface: Hsla = roles.bg_primary.into();
        let dim = |color: Hsla| Self::disabled(color, surface);
        Self {
            track: dim(colors.track),
            track_hover: dim(colors.track_hover),
            border: dim(colors.border),
            border_hover: dim(colors.border_hover),
            thumb: dim(colors.thumb),
            thumb_border: dim(colors.thumb_border),
            thumb_border_hover: dim(colors.thumb_border_hover),
        }
    }
}

#[cfg(test)]
mod tests;
