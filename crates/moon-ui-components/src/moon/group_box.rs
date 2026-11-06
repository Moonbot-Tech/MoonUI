use gpui::*;

use super::{
    background::MoonBackgroundPolicy,
    skin::{MoonFacetKind, facet},
    text::MoonText,
    theme::{MoonTheme, MoonThemeTokens},
    tokens::{MoonRect, rgba_from},
};

/// What a group box paints under the Facet skin.
#[derive(Clone, Debug, PartialEq)]
struct GroupBoxFacet {
    /// Fill of the box; `None` leaves it as its background policy left it.
    fill: Option<Background>,
    /// Colour of the box's one-pixel edge.
    edge: Hsla,
    /// Shadows of the box.
    shadows: Vec<BoxShadow>,
}

/// Resolve the look of a group box under the Facet skin.
///
/// A box that paints its own fill is a card resting on its panel. A box whose policy leaves it
/// see-through keeps the card's edge and the highlight along its top only — GPUI paints a
/// shadow that falls outside an element under the whole of it, so the card's shadow would show
/// through the box.
///
/// Args:
///     tokens: Active theme tokens; supply the skin, the palette and the UI scale.
///     policy: Background policy of the box.
///
/// Returns:
///     The faceted look, or `None` under the Flat skin, where the box keeps its flat fill and
///     its palette border.
fn group_box_facet(
    tokens: &MoonThemeTokens,
    policy: MoonBackgroundPolicy,
) -> Option<GroupBoxFacet> {
    if !tokens.skin.is_facet() {
        return None;
    }
    let card = facet(MoonFacetKind::Card, tokens);
    let opaque = matches!(policy, MoonBackgroundPolicy::Opaque);
    Some(GroupBoxFacet {
        fill: opaque.then_some(card.fill),
        edge: card.edge,
        shadows: card
            .shadows
            .into_iter()
            .filter(|shadow| opaque || shadow.inset)
            .collect(),
    })
}

#[derive(IntoElement)]
pub struct MoonGroupBox {
    id: SharedString,
    bounds: Option<MoonRect>,
    title: Option<SharedString>,
    children: Vec<AnyElement>,
    background_policy: MoonBackgroundPolicy,
    padding: f32,
    gap: f32,
}

impl MoonGroupBox {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            title: None,
            children: Vec::new(),
            background_policy: MoonBackgroundPolicy::Opaque,
            padding: 10.0,
            gap: 8.0,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn background_policy(mut self, policy: MoonBackgroundPolicy) -> Self {
        self.background_policy = policy;
        self
    }

    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }
}

impl ParentElement for MoonGroupBox {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for MoonGroupBox {
    /// Render the bordered box, its optional title and its children.
    ///
    /// The box paints the flat Moon fill under the Flat skin and a card surface under the Facet
    /// skin; see [`group_box_facet`].
    ///
    /// Args:
    ///     _: Window the box renders in; unused.
    ///     cx: Application context used to resolve the active theme tokens.
    ///
    /// Returns:
    ///     The rendered group box.
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let p = tokens.palette;
        let mut root = div()
            .id(ElementId::from(self.id))
            .relative()
            .rounded(px(tokens.ui(5.0)))
            .border(px(tokens.ui(1.0)))
            .border_color(rgba_from(p.border, 1.0))
            .p(px(tokens.ui(self.padding)))
            .flex()
            .flex_col()
            .gap(px(tokens.ui(self.gap)));
        root = self.background_policy.apply(root, p.shell_high, 0.98);
        // `None` under the Flat skin, which keeps the fill and the border set above.
        if let Some(group_facet) = group_box_facet(&tokens, self.background_policy) {
            root = root
                .border_color(group_facet.edge)
                .shadow(group_facet.shadows);
            if let Some(fill) = group_facet.fill {
                root = root.bg(fill);
            }
        }

        if let Some(bounds) = self.bounds {
            root = root
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }

        if let Some(title) = self.title {
            root = root.child(
                MoonText::new(title)
                    .color(p.text_soft)
                    .font_size(10.5)
                    .line_height(13.0)
                    .weight(600.0)
                    .mono(true)
                    .uppercase(false)
                    .render(),
            );
        }

        root.children(self.children)
    }
}

#[cfg(test)]
mod tests;
