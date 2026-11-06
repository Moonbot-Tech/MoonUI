//! Focusable, clickable card that hosts arbitrary children and shows a selected state.

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use std::rc::Rc;

use super::{
    foundation::box_shadow,
    skin::{MoonFacetKind, facet, facet_glow},
    theme::{MoonTheme, MoonThemeTokens},
    tokens::{MoonPalette, MoonTone, rgba_from},
};

type MoonCardClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// Design-reference corner radius of the card.
const CARD_RADIUS: f32 = 8.0;
/// Design-reference inner padding of the card.
const CARD_PADDING: f32 = 10.0;

/// Outline colours of a card in one interaction state.
#[derive(Clone, Copy, Debug, PartialEq)]
struct CardChrome {
    border: u32,
    /// Ring drawn outside the border without moving the content: colour, alpha, spread.
    ring: Option<(u32, f32, f32)>,
}

/// Resolve the card outline for its selection and focus state.
///
/// The selected ring and the focus ring are both box shadows, so neither changes the card's size
/// or shifts its children when the state flips.
///
/// Args:
///     selected: Whether the card is the chosen one.
///     focused: Whether the card has keyboard focus.
///     accent: Accent colour of the selected state.
///     p: Palette the card paints with.
///
/// Returns:
///     The border colour and optional outer ring.
fn card_chrome(selected: bool, focused: bool, accent: u32, p: MoonPalette) -> CardChrome {
    let ring = if selected {
        Some((accent, 1.0, 1.0))
    } else if focused {
        Some((accent, 0.55, 2.0))
    } else {
        None
    };
    CardChrome {
        border: if selected { accent } else { p.border },
        ring,
    }
}

/// Alpha of the accent on the edge of a selected faceted card.
const FACET_SELECTED_EDGE_ALPHA: f32 = 0.75;
/// Alpha of the glow around a selected faceted card.
const FACET_SELECTED_GLOW_ALPHA: f32 = 0.2;
/// Blur of that glow as the design gives it, in design-reference pixels.
const FACET_SELECTED_GLOW_BLUR: f32 = 22.0;
/// Alpha of the `overlay` ink on the edge of a faceted card under the pointer.
const FACET_HOVER_EDGE_ALPHA: f32 = 0.22;

/// What a card paints under the Facet skin.
#[derive(Clone, Debug, PartialEq)]
struct CardFacet {
    /// Fill of the card.
    fill: Background,
    /// Colour of the card's one-pixel edge.
    edge: Hsla,
    /// Colour the edge takes under the pointer; applied to an unselected card only.
    hover_edge: Hsla,
    /// Shadows of the card: the card's own, then the selected glow or the focus ring.
    shadows: Vec<BoxShadow>,
}

/// Resolve the look of a card under the Facet skin.
///
/// A faceted card is a card surface resting on its panel. The selected one is told apart by an
/// accent edge and a soft accent glow, not by the hard one-pixel ring of the Flat skin. Keyboard
/// focus on an unselected card keeps its ring, drawn outside the card's own shadows.
///
/// Args:
///     selected: Whether the card is the chosen one.
///     focused: Whether the card has keyboard focus.
///     accent: Accent colour of the selected state.
///     tokens: Active theme tokens; supply the skin, the palette and the UI scale.
///
/// Returns:
///     The faceted look, or `None` under the Flat skin, where [`card_chrome`] alone decides the
///     outline.
fn card_facet(
    selected: bool,
    focused: bool,
    accent: u32,
    tokens: &MoonThemeTokens,
) -> Option<CardFacet> {
    if !tokens.skin.is_facet() {
        return None;
    }
    let p = tokens.palette;
    let card = facet(MoonFacetKind::Card, tokens);
    let mut shadows = card.shadows;
    let edge = if selected {
        shadows.push(facet_glow(
            accent,
            FACET_SELECTED_GLOW_ALPHA,
            FACET_SELECTED_GLOW_BLUR,
            tokens,
        ));
        rgba_from(accent, FACET_SELECTED_EDGE_ALPHA)
    } else {
        if let Some((color, alpha, spread)) = card_chrome(selected, focused, accent, p).ring {
            shadows.push(box_shadow(
                px(0.0),
                px(0.0),
                px(0.0),
                px(tokens.ui(spread)),
                rgba_from(color, alpha),
            ));
        }
        card.edge
    };
    Some(CardFacet {
        fill: card.fill,
        edge,
        hover_edge: rgba_from(p.overlay, FACET_HOVER_EDGE_ALPHA),
        shadows,
    })
}

/// Moon selectable card: a bordered container for arbitrary content, such as a theme preview,
/// that the user picks by click, or by Tab and Space/Enter. The selected card takes an accent
/// border.
#[derive(IntoElement)]
pub struct MoonSelectableCard {
    id: SharedString,
    selected: bool,
    disabled: bool,
    tone: Option<MoonTone>,
    on_click: Option<MoonCardClickHandler>,
    children: Vec<AnyElement>,
}

impl MoonSelectableCard {
    /// Create an unselected card.
    ///
    /// Args:
    ///     id: Stable identity; also keys the card's focus handle.
    ///
    /// Returns:
    ///     A card builder.
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            selected: false,
            disabled: false,
            tone: None,
            on_click: None,
            children: Vec::new(),
        }
    }

    /// Mark the card as the chosen one.
    ///
    /// Args:
    ///     selected: Whether the card is selected.
    ///
    /// Returns:
    ///     The updated card.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Dim the card and ignore clicks and focus.
    ///
    /// Args:
    ///     disabled: Whether the card is disabled.
    ///
    /// Returns:
    ///     The updated card.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Use a tone for the selected border instead of the palette accent.
    ///
    /// Args:
    ///     tone: Accent tone.
    ///
    /// Returns:
    ///     The updated card.
    pub fn tone(mut self, tone: MoonTone) -> Self {
        self.tone = Some(tone);
        self
    }

    /// Handle a click, or Space/Enter while the card is focused; also makes the card focusable.
    ///
    /// Args:
    ///     handler: Click callback.
    ///
    /// Returns:
    ///     The updated card.
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl ParentElement for MoonSelectableCard {
    /// Append children rendered inside the card's padding.
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for MoonSelectableCard {
    /// Render the card with the active palette, focus state and click handling.
    ///
    /// Args:
    ///     window: Window that owns the keyed focus handle.
    ///     cx: Application context.
    ///
    /// Returns:
    ///     The rendered card.
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        let accent = self.tone.map_or(p.accent, |tone| tone.color(p));
        let interactive = !self.disabled && self.on_click.is_some();
        let focus_handle = window
            .use_keyed_state(
                ElementId::from(SharedString::from(format!("{}:focus", self.id))),
                cx,
                |_, cx| cx.focus_handle(),
            )
            .read(cx)
            .clone();
        let focused = interactive && focus_handle.is_focused(window);
        let chrome = card_chrome(self.selected, focused, accent, p);
        // `None` under the Flat skin, where `chrome` alone paints the card.
        let card_facet = card_facet(self.selected, focused, accent, &tokens);
        let facet_hover_edge = card_facet.as_ref().map(|card_facet| card_facet.hover_edge);
        let id = self.id.clone();

        let mut card = div()
            .id(ElementId::from(self.id))
            .debug_selector(move || format!("moon-selectable-card:{id}"))
            .relative()
            .flex()
            .flex_col()
            .p(px(tokens.ui(CARD_PADDING)))
            .rounded(px(tokens.ui(CARD_RADIUS)))
            .border(px(tokens.ui(1.0)))
            .map(|this| match card_facet {
                Some(card_facet) => this
                    .border_color(card_facet.edge)
                    .bg(card_facet.fill)
                    .shadow(card_facet.shadows),
                None => this
                    .border_color(rgba_from(chrome.border, 1.0))
                    .bg(rgba_from(p.panel_high, 1.0))
                    .when_some(chrome.ring, |this, (color, alpha, spread)| {
                        this.shadow(vec![box_shadow(
                            px(0.0),
                            px(0.0),
                            px(0.0),
                            px(tokens.ui(spread)),
                            rgba_from(color, alpha),
                        )])
                    }),
            })
            .when(self.disabled, |this| this.opacity(0.5))
            .when(interactive, |this| {
                this.track_focus(&focus_handle.clone().tab_stop(true))
                    .cursor_pointer()
                    .when(!self.selected, |this| match facet_hover_edge {
                        Some(edge) => this.hover(move |this| this.border_color(edge)),
                        None => this
                            .hover(move |this| this.border_color(rgba_from(p.border_hover, 1.0))),
                    })
            })
            .children(self.children);

        if let Some(on_click) = self.on_click.filter(|_| interactive) {
            // Pressing the card focuses it, so Space/Enter and Tab continue from the picked card.
            card = card
                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    window.focus(&focus_handle, cx);
                })
                .on_click(move |event, window, cx| on_click(event, window, cx));
        }
        card
    }
}

#[cfg(test)]
mod tests;
