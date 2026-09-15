use crate::button::{Button, ButtonRounded, ButtonVariant, ButtonVariants};
use crate::{Disableable, Icon, Selectable, Sizable};
use gpui::prelude::FluentBuilder as _;
use gpui::*;

use super::{
    disclosure::{MoonDisclosureDirection, moon_disclosure_rotation_turns},
    foundation::MoonSize,
    icons::MOON_ICON_CARET_DOWN,
    theme::{MoonTheme, MoonThemeTokens},
    tokens::{MoonRect, rgb_from},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoonButtonVariant {
    Neutral,
    Panel,
    Soft,
    Blue,
    Amber,
    Green,
    Red,
    Danger,
    OutlineAmber,
    OutlineRed,
    Ghost,
    Bare,
}

impl From<MoonButtonVariant> for ButtonVariant {
    fn from(value: MoonButtonVariant) -> Self {
        match value {
            MoonButtonVariant::Neutral => ButtonVariant::Default,
            MoonButtonVariant::Panel => ButtonVariant::Panel,
            MoonButtonVariant::Soft => ButtonVariant::Soft,
            MoonButtonVariant::Blue => ButtonVariant::Blue,
            MoonButtonVariant::Amber => ButtonVariant::Amber,
            MoonButtonVariant::Green => ButtonVariant::Green,
            MoonButtonVariant::Red => ButtonVariant::Red,
            MoonButtonVariant::Danger => ButtonVariant::Danger,
            MoonButtonVariant::OutlineAmber => ButtonVariant::OutlineAmber,
            MoonButtonVariant::OutlineRed => ButtonVariant::OutlineRed,
            MoonButtonVariant::Ghost => ButtonVariant::Ghost,
            MoonButtonVariant::Bare => ButtonVariant::Bare,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MoonButtonSize {
    /// A tier of the shared size scale. Buttons render Xs, Sm, Md and Lg; Xl/Xxl snap to Lg.
    Tier(MoonSize),
    /// Base (unscaled) values, scaled at render; text keeps following `tokens.font()`.
    Custom {
        height: f32,
        radius: f32,
        font_size: f32,
        line_height: f32,
        gap: f32,
    },
}

impl MoonButtonSize {
    pub const SUPPORTED: &'static [MoonSize] =
        &[MoonSize::Xs, MoonSize::Sm, MoonSize::Md, MoonSize::Lg];

    /// The app's density tier snapped to what buttons support.
    ///
    /// Args:
    ///     cx: Application context used to read the active Moon scale.
    ///
    /// Returns:
    ///     `Tier` of `tokens.tier().nearest(SUPPORTED)`.
    pub fn density(cx: &App) -> Self {
        density_button_size(&MoonTheme::active_tokens(cx))
    }

    /// Snap a tier to what buttons support; `Custom` is unchanged.
    ///
    /// Returns:
    ///     `Tier(t.nearest(SUPPORTED))` or the original `Custom`.
    pub fn snapped(self) -> Self {
        match self {
            Self::Tier(tier) => Self::Tier(tier.nearest(Self::SUPPORTED)),
            Self::Custom { .. } => self,
        }
    }

    /// Snapped tier, or `None` for `Custom`.
    ///
    /// Returns:
    ///     The supported `MoonSize` this size renders at, if it is a tier.
    pub fn tier(self) -> Option<MoonSize> {
        match self.snapped() {
            Self::Tier(tier) => Some(tier),
            Self::Custom { .. } => None,
        }
    }

    /// Glyph edge for a tier's icon slots: `tier.control_metrics().font_size`.
    ///
    /// Args:
    ///     tier: Size tier whose control metrics supply the glyph edge.
    ///
    /// Returns:
    ///     Unscaled design-reference icon size in pixels.
    pub fn tier_icon_size(tier: MoonSize) -> f32 {
        tier.control_metrics().font_size
    }
}

impl From<MoonSize> for MoonButtonSize {
    fn from(size: MoonSize) -> Self {
        Self::Tier(size)
    }
}

#[derive(Clone, Debug)]
pub struct MoonButtonSegment {
    text: SharedString,
    color: Option<u32>,
    alpha: f32,
    font_size: Option<f32>,
    line_height: Option<f32>,
    tracking: Option<f32>,
    weight: f32,
    mono: Option<bool>,
}

impl MoonButtonSegment {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            color: None,
            alpha: 1.0,
            font_size: None,
            line_height: None,
            tracking: None,
            weight: 400.0,
            mono: None,
        }
    }

    pub fn color(mut self, color: u32) -> Self {
        self.color = Some(color);
        self
    }

    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha;
        self
    }

    /// Base (unscaled) font size override. A `Tier` size scales this with
    /// `tokens.ui()`; a `Custom` size scales with `tokens.font()`. Pass
    /// design-reference values, never pre-scaled ones (double scaling).
    pub fn font_size(mut self, font_size: f32) -> Self {
        self.font_size = Some(font_size);
        self
    }

    /// Base (unscaled) line height override — scaled at render like
    /// [`Self::font_size`] (`tokens.ui()` on a tier, `tokens.line_height()` on
    /// `Custom`).
    pub fn line_height(mut self, line_height: f32) -> Self {
        self.line_height = Some(line_height);
        self
    }

    pub fn tracking(mut self, tracking: f32) -> Self {
        self.tracking = Some(tracking);
        self
    }

    pub fn weight(mut self, weight: f32) -> Self {
        self.weight = weight;
        self
    }

    pub fn mono(mut self, mono: bool) -> Self {
        self.mono = Some(mono);
        self
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MoonButtonIconSlot {
    path: &'static str,
    size: Option<f32>,
    color: Option<u32>,
    alpha: f32,
    /// Clockwise turns folded into one revolution; zero preserves the untransformed icon path.
    rotation: f32,
}

impl MoonButtonIconSlot {
    pub fn new(path: &'static str) -> Self {
        Self {
            path,
            size: None,
            color: None,
            alpha: 1.0,
            rotation: 0.0,
        }
    }

    /// Build the shared disclosure caret in the pose selected by its direction and state.
    ///
    /// Reusing the disclosure asset and pose mapping keeps button-based expand/collapse controls
    /// aligned with [`MoonDisclosure`](super::MoonDisclosure).
    ///
    /// The slot retains its own default glyph size, subject to the placement-specific override
    /// described by [`Self::size`].
    ///
    /// Args:
    ///     direction: The collapsed/expanded pose pair this control renders.
    ///     expanded: Whether the disclosed content is currently open.
    ///
    /// Returns:
    ///     An icon slot carrying the caret asset already rotated into its pose.
    pub fn caret(direction: MoonDisclosureDirection, expanded: bool) -> Self {
        Self::new(MOON_ICON_CARET_DOWN)
            .rotation(moon_disclosure_rotation_turns(direction, expanded))
    }

    /// Set the base, unscaled glyph edge length.
    ///
    /// For leading and loading slots, the inherited `Button::render` derives an icon size from the
    /// button's metrics and applies it through `ButtonIconVariant::with_size`, replacing this
    /// value. A genuine trailing icon is added as a child instead and retains this size, falling
    /// back to the tier icon size or 12px on `Custom` when unset; a lone trailing icon is promoted
    /// to the leading slot and receives the derived size.
    ///
    /// Args:
    ///     size: Unscaled design-reference glyph edge length.
    ///
    /// Returns:
    ///     This icon slot with the requested glyph size.
    pub fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    /// Rotate the glyph clockwise in turns; `0.25` is a quarter turn and `0.5` is a half turn.
    ///
    /// GPUI's `percentage` represents a fraction of one full circle and debug-asserts that its
    /// input is in `0.0..=1.0`. Folding finite input into `[0.0, 1.0)` therefore accepts equivalent
    /// negative and multi-turn poses without passing an out-of-range value to `percentage`.
    /// Non-finite input cannot describe a pose and is reset to zero.
    ///
    /// When this slot supplies a button's loading icon, `Spinner::render` replaces its static SVG
    /// transformation with the animated rotation on every frame.
    ///
    /// Args:
    ///     rotation: Clockwise turns applied to the glyph.
    ///
    /// Returns:
    ///     This icon slot with the requested rotation.
    pub fn rotation(mut self, rotation: f32) -> Self {
        self.rotation = if rotation.is_finite() {
            rotation.rem_euclid(1.0)
        } else {
            0.0
        };
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

    /// Resolve the slot into a theme-scaled icon.
    ///
    /// A zero rotation deliberately skips `Icon::rotate` so existing unrotated slots retain the
    /// same SVG path without an attached transformation.
    ///
    /// Args:
    ///     cx: Application context used to resolve the active Moon scale.
    ///     fallback: Unscaled glyph edge used when [`Self::size`] was not set.
    ///
    /// Returns:
    ///     The configured icon with scaling, colour, alpha, and any non-zero rotation applied.
    fn icon(self, cx: &App, fallback: f32) -> Icon {
        let tokens = MoonTheme::active_tokens(cx);
        let mut icon = Icon::default()
            .path(self.path)
            .size(px(tokens.ui(self.size.unwrap_or(fallback))));
        if let Some(color) = self.color {
            icon = icon.text_color(rgba_from_u32(color, self.alpha));
        }
        if self.rotation != 0.0 {
            icon = icon.rotate(percentage(self.rotation));
        }
        icon
    }
}

/// Theme-aware Moon button with semantic variants, scaled geometry, and rich content slots.
#[derive(IntoElement)]
pub struct MoonButton {
    id: ElementId,
    bounds: Option<MoonRect>,
    width: Option<f32>,
    full_width: bool,
    padding_x: Option<f32>,
    segments: Vec<MoonButtonSegment>,
    variant: MoonButtonVariant,
    size: Option<MoonButtonSize>,
    pill: bool,
    selected: bool,
    disabled: bool,
    leading_icon: Option<MoonButtonIconSlot>,
    trailing_icon: Option<MoonButtonIconSlot>,
    loading_icon: Option<MoonButtonIconSlot>,
    loading: bool,
    radius: Option<f32>,
    tooltip: Option<SharedString>,
    mono: Option<bool>,
    on_hover: Option<std::rc::Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
    on_click: Option<std::rc::Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
    tab_index: isize,
    tab_stop: bool,
}

impl MoonButton {
    /// Create a button that follows the app's density tier and the neutral variant.
    ///
    /// Args:
    ///     id: Stable element identity used for focus and interaction state.
    ///
    /// Returns:
    ///     A default Moon button builder.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            width: None,
            full_width: false,
            padding_x: None,
            segments: Vec::new(),
            variant: MoonButtonVariant::Neutral,
            size: None,
            pill: false,
            selected: false,
            disabled: false,
            leading_icon: None,
            trailing_icon: None,
            loading_icon: None,
            loading: false,
            radius: None,
            tooltip: None,
            mono: None,
            on_hover: None,
            on_click: None,
            tab_index: 0,
            tab_stop: true,
        }
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.segments.push(MoonButtonSegment::new(label));
        self
    }

    pub fn xsmall(self) -> Self {
        self.size(MoonSize::Xs)
    }

    pub fn small(self) -> Self {
        self.size(MoonSize::Sm)
    }

    pub fn medium(self) -> Self {
        self.size(MoonSize::Md)
    }

    pub fn primary(self) -> Self {
        self.variant(MoonButtonVariant::Blue)
    }

    pub fn success(self) -> Self {
        self.variant(MoonButtonVariant::Green)
    }

    pub fn warning(self) -> Self {
        self.variant(MoonButtonVariant::Amber)
    }

    pub fn danger(self) -> Self {
        self.variant(MoonButtonVariant::Danger)
    }

    pub fn outline(self) -> Self {
        self.variant(MoonButtonVariant::OutlineAmber)
    }

    pub fn ghost(self) -> Self {
        self.variant(MoonButtonVariant::Ghost)
    }

    pub fn segment(mut self, segment: MoonButtonSegment) -> Self {
        self.segments.push(segment);
        self
    }

    pub fn icon(self, path: &'static str) -> Self {
        self.leading_icon(MoonButtonIconSlot::new(path))
    }

    pub fn leading_icon(mut self, icon: MoonButtonIconSlot) -> Self {
        self.leading_icon = Some(icon);
        self
    }

    pub fn trailing_icon(mut self, icon: MoonButtonIconSlot) -> Self {
        self.trailing_icon = Some(icon);
        self
    }

    pub fn loading_icon(self, path: &'static str) -> Self {
        self.loading_icon_slot(MoonButtonIconSlot::new(path))
    }

    pub fn loading_icon_slot(mut self, icon: MoonButtonIconSlot) -> Self {
        self.loading_icon = Some(icon);
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    pub fn rounded(mut self, radius: f32) -> Self {
        self.radius = Some(radius);
        self
    }

    /// Override horizontal content padding in UI-scaled design units.
    ///
    /// The override leaves the size preset's height, typography, and icon gap unchanged. Omitting
    /// it preserves the preset's native padding.
    ///
    /// Args:
    ///     padding_x: Horizontal inset on each side at the reference UI scale.
    ///
    /// Returns:
    ///     The updated button.
    pub fn padding_x(mut self, padding_x: f32) -> Self {
        self.padding_x = Some(padding_x.max(0.0));
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    pub fn text_segment(mut self, text: impl Into<SharedString>, color: u32, weight: f32) -> Self {
        self.segments
            .push(MoonButtonSegment::new(text).color(color).weight(weight));
        self
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }

    pub fn variant(mut self, variant: MoonButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set the button size to a shared tier or a `Custom` metrics block.
    ///
    /// Args:
    ///     size: A `MoonSize` tier or a `MoonButtonSize`.
    ///
    /// Returns:
    ///     The button with an explicit size (no longer following density).
    pub fn size(mut self, size: impl Into<MoonButtonSize>) -> Self {
        self.size = Some(size.into());
        self
    }

    /// Pill shape: 999 radius on any size. Wins over the size's radius.
    ///
    /// Returns:
    ///     The button with pill rounding.
    pub fn pill(mut self) -> Self {
        self.pill = true;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn mono(mut self, mono: bool) -> Self {
        self.mono = Some(mono);
        self
    }

    pub fn tab_index(mut self, tab_index: isize) -> Self {
        self.tab_index = tab_index;
        self
    }

    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(std::rc::Rc::new(handler));
        self
    }

    pub fn on_hover(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_hover = Some(std::rc::Rc::new(handler));
        self
    }

    pub fn render(self) -> impl IntoElement {
        self
    }
}

impl RenderOnce for MoonButton {
    /// Render the configured button with theme-scaled geometry and interaction handlers.
    ///
    /// Args:
    ///     _window: Window that owns the rendered button.
    ///     cx: Application context used to resolve active Moon theme tokens.
    ///
    /// Returns:
    ///     The rendered one-shot button element.
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let size = resolve_button_size(self.size, &tokens);
        let icon_fallback = match size {
            MoonButtonSize::Tier(tier) => MoonButtonSize::tier_icon_size(tier),
            MoonButtonSize::Custom { .. } => 12.0,
        };
        let mut button = Button::new(self.id)
            .with_variant(self.variant.into())
            .selected(self.selected)
            .disabled(self.disabled)
            .loading(self.loading)
            .tab_index(self.tab_index)
            .tab_stop(self.tab_stop);

        match size {
            MoonButtonSize::Tier(tier) => {
                button = button
                    .with_moon_metrics(tier_button_metrics(tier, &tokens))
                    .with_size(crate::Size::from(size));
            }
            MoonButtonSize::Custom { height, .. } => {
                button = button.with_size(crate::Size::Size(px(height)));
            }
        }

        let radius = if self.pill {
            Some(999.0)
        } else {
            self.radius.or_else(|| custom_radius(size))
        };
        if let Some(radius) = radius {
            button = button.rounded(ButtonRounded::Size(px(tokens.ui(radius))));
        }
        if let Some(bounds) = self.bounds {
            button = button
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }
        if let Some(width) = self.width {
            button = button.w(px(width));
        }
        if self.full_width {
            button = button.w_full();
        }
        if let Some(padding_x) = self.padding_x {
            button = button.px(px(tokens.ui(padding_x)));
        }
        // With no text segments there is nothing for a trailing icon to trail — it simply
        // IS the icon. Promote it to the leading slot so `Button` sees a genuine icon-only
        // button and can take its square path; left where it is, it would be attached as a
        // child and defeat that path exactly like an empty segment container does.
        let (leading_icon, trailing_icon) = match (self.leading_icon, self.trailing_icon) {
            (None, trailing @ Some(_)) if self.segments.is_empty() => (trailing, None),
            pair => pair,
        };
        if let Some(icon) = leading_icon {
            button = button.icon(icon.icon(cx, icon_fallback));
        }
        if let Some(icon) = self.loading_icon {
            button = button.loading_icon(icon.icon(cx, icon_fallback));
        }
        if let Some(tooltip) = self.tooltip {
            button = button.tooltip(tooltip);
        }
        if let Some(on_hover) = self.on_hover {
            button = button.on_hover(move |hovered, window, cx| on_hover(hovered, window, cx));
        }
        if let Some(on_click) = self.on_click {
            button = button.on_click(move |event, window, cx| on_click(event, window, cx));
        }

        let (font_size, line_height, gap) = button_text_metrics(size);
        let button_mono = self.mono;
        if self.segments.is_empty() {
            // Icon-only: emit NO segment container at all. An empty one is not harmless —
            // it keeps `Button` off its square icon-only path (which requires no label AND
            // no children), and it becomes a second flex item, so the row's `gap` is
            // inserted between the icon and a zero-width element. That offsets the centred
            // content block by `gap`, leaving the glyph `gap / 2` left of true centre.
            button
        } else if self.segments.len() == 1
            && self.segments[0].color.is_none()
            && self.segments[0].font_size.is_none()
            && self.segments[0].line_height.is_none()
            && self.segments[0].tracking.is_none()
            && self.segments[0].mono.is_none()
            && button_mono.is_none()
        {
            button.label(self.segments[0].text.clone())
        } else {
            button.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(tokens.ui(gap)))
                    .children(self.segments.into_iter().map(move |segment| {
                        let mono = segment.mono.or(button_mono).unwrap_or(false);
                        let (seg_font, seg_line) = match size {
                            MoonButtonSize::Tier(_) => (
                                tokens.ui(segment.font_size.unwrap_or(font_size)),
                                tokens.ui(segment.line_height.unwrap_or(line_height)),
                            ),
                            MoonButtonSize::Custom { .. } => {
                                let text_metrics = tokens.text(
                                    segment.font_size.unwrap_or(font_size),
                                    segment.line_height.unwrap_or(line_height),
                                );
                                (text_metrics.font_size, text_metrics.line_height)
                            }
                        };
                        let mut text = div()
                            .text_size(px(seg_font))
                            .line_height(px(seg_line))
                            .font_weight(FontWeight(segment.weight))
                            .when(mono, |this| this.font_family(tokens.font_family(true)))
                            .child(segment.text);
                        if let Some(color) = segment.color {
                            text = text.text_color(rgba_from_u32(color, segment.alpha));
                        }
                        text.into_any_element()
                    })),
            )
        }
        // Still attached whenever the trailing icon is genuinely trailing something — text
        // segments, or a leading icon beside it. Only the trailing-ONLY case is absent here,
        // because the promotion above moved that icon into the leading slot.
        .when_some(trailing_icon, |this, icon| {
            this.child(icon.icon(cx, icon_fallback))
        })
    }
}

/// Resolve a Moon button size to the base size its geometry is keyed by.
///
/// Public because a non-button control may need to line up with a button: it names the button
/// size it stands beside, and the widget resolves the same metrics the button would draw.
/// Tiers are snapped before mapping so `Xl`/`Xxl` never escape as an out-of-range native size.
impl From<MoonButtonSize> for crate::Size {
    fn from(size: MoonButtonSize) -> Self {
        match size.snapped() {
            MoonButtonSize::Tier(MoonSize::Xs) => crate::Size::XSmall,
            MoonButtonSize::Tier(MoonSize::Sm) => crate::Size::Small,
            MoonButtonSize::Tier(MoonSize::Md) => crate::Size::Medium,
            MoonButtonSize::Tier(MoonSize::Lg | MoonSize::Xl | MoonSize::Xxl) => crate::Size::Large,
            MoonButtonSize::Custom { height, .. } => crate::Size::Size(px(height)),
        }
    }
}

fn custom_radius(size: MoonButtonSize) -> Option<f32> {
    match size {
        MoonButtonSize::Custom { radius, .. } => Some(radius),
        MoonButtonSize::Tier(_) => None,
    }
}

/// Resolve the size a button will render at.
///
/// `None` follows the app's density tier, snapped to what buttons support. An explicit
/// `Tier` is snapped the same way; `Custom` is unchanged.
///
/// Args:
///     size: Optional explicit size from the builder.
///     tokens: Active theme tokens, used for the density default.
///
/// Returns:
///     A snapped `Tier` or the original `Custom`.
pub(super) fn resolve_button_size(
    size: Option<MoonButtonSize>,
    tokens: &MoonThemeTokens,
) -> MoonButtonSize {
    match size {
        None => density_button_size(tokens),
        Some(size) => size.snapped(),
    }
}

/// Snap the active density tier to what buttons support.
fn density_button_size(tokens: &MoonThemeTokens) -> MoonButtonSize {
    MoonButtonSize::Tier(tokens.tier().nearest(MoonButtonSize::SUPPORTED))
}

/// Build already-scaled native metrics for a tier. Every field goes through `tokens.ui()`;
/// this must not call `scaled()`, which would font-scale text and clamp the icon.
fn tier_button_metrics(
    tier: MoonSize,
    tokens: &MoonThemeTokens,
) -> crate::button::MoonButtonMetrics {
    let m = tier.control_metrics();
    crate::button::MoonButtonMetrics {
        height: px(tokens.ui(m.height)),
        radius: px(tokens.ui(m.radius)),
        font_size: px(tokens.ui(m.font_size)),
        line_height: px(tokens.ui(m.line_height)),
        gap: px(tokens.ui(m.gap)),
        pad_x: px(tokens.ui(m.pad_x)),
        icon_size: px(tokens.ui(MoonButtonSize::tier_icon_size(tier))),
    }
}

/// Return the design-reference typography and content gap for a Moon button size.
///
/// Args:
///     size: Button size whose text is rendered or measured.
///
/// Returns:
///     Font size, line height, and gap in design-reference pixels.
pub(super) fn button_text_metrics(size: MoonButtonSize) -> (f32, f32, f32) {
    match size {
        MoonButtonSize::Tier(tier) => {
            let m = tier.control_metrics();
            (m.font_size, m.line_height, m.gap)
        }
        MoonButtonSize::Custom {
            font_size,
            line_height,
            gap,
            ..
        } => (font_size, line_height, gap),
    }
}

/// Resolve the rendered width occupied by a native leading icon and its following gap.
///
/// A tier reserves `ui(tier_icon_size) + ui(gap)`. A `Custom` size keeps the native clamp
/// from the height-derived font. Dropdown text fitting uses this helper before Button layout
/// exists so its bounded labels leave the same amount of room.
///
/// Args:
///     size: Moon button size mapped to the native Button preset.
///     tokens: Active theme tokens used by the native icon and gap metrics.
///
/// Returns:
///     Rendered width reserved before the button's label content.
pub(super) fn button_leading_icon_reservation(
    size: MoonButtonSize,
    tokens: &MoonThemeTokens,
) -> f32 {
    match size {
        MoonButtonSize::Tier(tier) => {
            let m = tier.control_metrics();
            tokens.ui(MoonButtonSize::tier_icon_size(tier)) + tokens.ui(m.gap)
        }
        MoonButtonSize::Custom { height, .. } => {
            (tokens.font(height * 0.4) + 1.0).clamp(10.0, 14.0) + tokens.ui(6.0)
        }
    }
}

fn rgba_from_u32(color: u32, alpha: f32) -> Hsla {
    let mut color = rgb_from(color);
    color.a *= alpha;
    color
}

#[cfg(test)]
mod tests;
