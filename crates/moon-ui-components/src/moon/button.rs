use crate::button::{Button, ButtonRounded, ButtonVariant, ButtonVariants};
use crate::{Disableable, Icon, Selectable, Sizable};
use gpui::prelude::FluentBuilder as _;
use gpui::*;

use super::{
    theme::MoonTheme,
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
    Micro,
    /// Dense terminal/header toolbar control. It keeps the toolbar visually
    /// aligned with 26px segmented controls while still using the toolbar text
    /// metrics and variants.
    ToolbarCompact,
    Toolbar,
    Action,
    Pill,
    /// All metrics are base (unscaled) values — the button scales them with the
    /// theme tokens at render time. Pass design-reference numbers, never values
    /// that were already scaled (double scaling).
    Custom {
        height: f32,
        radius: f32,
        font_size: f32,
        line_height: f32,
        gap: f32,
    },
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

    /// Base (unscaled) font size override — scaled with `tokens.font()` at render
    /// time (default comes from the button size). Pass design-reference values,
    /// never pre-scaled ones (double scaling).
    pub fn font_size(mut self, font_size: f32) -> Self {
        self.font_size = Some(font_size);
        self
    }

    /// Base (unscaled) line height override — scaled at render like
    /// [`Self::font_size`].
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
    size: f32,
    color: Option<u32>,
    alpha: f32,
}

impl MoonButtonIconSlot {
    pub fn new(path: &'static str) -> Self {
        Self {
            path,
            size: 12.0,
            color: None,
            alpha: 1.0,
        }
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
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

    fn icon(self, cx: &App) -> Icon {
        let tokens = MoonTheme::active_tokens(cx);
        let mut icon = Icon::default()
            .path(self.path)
            .size(px(tokens.ui(self.size)));
        if let Some(color) = self.color {
            icon = icon.text_color(rgba_from_u32(color, self.alpha));
        }
        icon
    }
}

#[derive(IntoElement)]
pub struct MoonButton {
    id: ElementId,
    bounds: Option<MoonRect>,
    width: Option<f32>,
    full_width: bool,
    segments: Vec<MoonButtonSegment>,
    variant: MoonButtonVariant,
    size: MoonButtonSize,
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
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            width: None,
            full_width: false,
            segments: Vec::new(),
            variant: MoonButtonVariant::Neutral,
            size: MoonButtonSize::Toolbar,
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
        self.size(MoonButtonSize::Micro)
    }

    pub fn small(self) -> Self {
        self.size(MoonButtonSize::Action)
    }

    pub fn medium(self) -> Self {
        self.size(MoonButtonSize::Toolbar)
    }

    pub fn toolbar_compact(self) -> Self {
        self.size(MoonButtonSize::ToolbarCompact)
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

    pub fn size(mut self, size: MoonButtonSize) -> Self {
        self.size = size;
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
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let mut button = Button::new(self.id)
            .with_variant(self.variant.into())
            .with_size(size_for(self.size))
            .selected(self.selected)
            .disabled(self.disabled)
            .loading(self.loading)
            .tab_index(self.tab_index)
            .tab_stop(self.tab_stop);

        if let Some(radius) = self.radius.or_else(|| custom_radius(self.size)) {
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
        // With no text segments there is nothing for a trailing icon to trail — it simply
        // IS the icon. Promote it to the leading slot so `Button` sees a genuine icon-only
        // button and can take its square path; left where it is, it would be attached as a
        // child and defeat that path exactly like an empty segment container does.
        let (leading_icon, trailing_icon) = match (self.leading_icon, self.trailing_icon) {
            (None, trailing @ Some(_)) if self.segments.is_empty() => (trailing, None),
            pair => pair,
        };
        if let Some(icon) = leading_icon {
            button = button.icon(icon.icon(cx));
        }
        if let Some(icon) = self.loading_icon {
            button = button.loading_icon(icon.icon(cx));
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

        let (font_size, line_height, gap) = metrics_for(self.size);
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
                        let text_metrics = tokens.text(
                            segment.font_size.unwrap_or(font_size),
                            segment.line_height.unwrap_or(line_height),
                        );
                        let mut text = div()
                            .text_size(px(text_metrics.font_size))
                            .line_height(px(text_metrics.line_height))
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
        .when_some(trailing_icon, |this, icon| this.child(icon.icon(cx)))
    }
}

fn size_for(size: MoonButtonSize) -> crate::Size {
    match size {
        MoonButtonSize::Micro => crate::Size::XSmall,
        MoonButtonSize::ToolbarCompact => crate::Size::Small,
        MoonButtonSize::Action => crate::Size::Small,
        MoonButtonSize::Toolbar => crate::Size::Medium,
        MoonButtonSize::Pill => crate::Size::Large,
        MoonButtonSize::Custom { height, .. } => crate::Size::Size(px(height)),
    }
}

fn custom_radius(size: MoonButtonSize) -> Option<f32> {
    match size {
        MoonButtonSize::Pill => Some(999.0),
        MoonButtonSize::Custom { radius, .. } => Some(radius),
        _ => None,
    }
}

fn metrics_for(size: MoonButtonSize) -> (f32, f32, f32) {
    match size {
        MoonButtonSize::Micro => (10.0, 14.0, 4.0),
        MoonButtonSize::ToolbarCompact => (10.0, 16.0, 4.0),
        MoonButtonSize::Action => (10.5, 16.0, 5.0),
        MoonButtonSize::Toolbar => (10.0, 16.0, 4.0),
        MoonButtonSize::Pill => (11.0, 16.0, 6.0),
        MoonButtonSize::Custom {
            font_size,
            line_height,
            gap,
            ..
        } => (font_size, line_height, gap),
    }
}

fn rgba_from_u32(color: u32, alpha: f32) -> Hsla {
    let mut color = rgb_from(color);
    color.a *= alpha;
    color
}

#[cfg(test)]
mod tests {
    use super::{MoonButton, MoonButtonIconSlot, MoonButtonSize, metrics_for, size_for};

    #[test]
    fn moon_button_width_builders_preserve_layout_intent() {
        let fixed = MoonButton::new("fixed").width(42.0);
        assert_eq!(fixed.width, Some(42.0));
        assert!(!fixed.full_width);

        let full = MoonButton::new("full").full_width();
        assert_eq!(full.width, None);
        assert!(full.full_width);
    }

    /// Verifies that compact-toolbar metrics map to the dense base-button size.
    #[test]
    fn toolbar_compact_keeps_terminal_toolbar_dense() {
        assert_eq!(
            metrics_for(MoonButtonSize::ToolbarCompact),
            (10.0, 16.0, 4.0)
        );
        assert_eq!(size_for(MoonButtonSize::ToolbarCompact), crate::Size::Small);
    }

    /// Root view that renders one button and records the laid-out bounds of the wrapper's
    /// direct children — i.e. the button's own box.
    ///
    /// A `MoonButton` cannot be drawn as a bare element: `Button::render` calls
    /// `use_keyed_state`, which needs a real rendering view on the stack.
    struct ButtonHarness {
        build: Box<dyn Fn() -> MoonButton>,
        bounds: std::rc::Rc<std::cell::RefCell<Vec<gpui::Bounds<gpui::Pixels>>>>,
    }

    impl gpui::Render for ButtonHarness {
        fn render(
            &mut self,
            _window: &mut gpui::Window,
            _cx: &mut gpui::Context<Self>,
        ) -> impl gpui::IntoElement {
            use gpui::{ParentElement as _, Styled as _};
            let sink = self.bounds.clone();
            // Start-aligned on both axes so the button shrink-wraps its content instead of
            // being stretched by the root — the measurement must be the button's own box.
            gpui::div()
                .flex()
                .flex_row()
                .items_start()
                .justify_start()
                .on_children_prepainted(move |bounds, _, _| *sink.borrow_mut() = bounds)
                .child((self.build)().render())
        }
    }

    /// Lay the built button out in a real window and return its box.
    fn laid_out_bounds(
        cx: &mut gpui::TestAppContext,
        build: impl Fn() -> MoonButton + 'static,
    ) -> gpui::Bounds<gpui::Pixels> {
        use gpui::AppContext as _;

        cx.update(crate::init);
        let bounds = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let sink = bounds.clone();
        let window = cx.add_window(move |_, _| ButtonHarness {
            build: Box::new(build),
            bounds: sink,
        });
        cx.update_window(window.into(), |_, window, cx| {
            window.draw(cx).clear();
        })
        .unwrap();

        let out = bounds.borrow();
        assert_eq!(out.len(), 1, "expected exactly the button as a child");
        out[0]
    }

    /// The dense-toolbar height, asserted on the rendered button: the height a caller actually
    /// gets is the one worth pinning.
    #[gpui::test]
    fn toolbar_compact_renders_at_dense_height(cx: &mut gpui::TestAppContext) {
        let bounds = laid_out_bounds(cx, || {
            MoonButton::new("dense").size(MoonButtonSize::ToolbarCompact)
        });

        assert_eq!(bounds.size.height, gpui::px(26.0));
    }

    /// An icon-only button must lay out as a square.
    ///
    /// `Button` only takes its square icon-only path when it has neither a label nor any
    /// children, so a non-square box is the observable proof that an empty segment
    /// container is still being emitted alongside the icon. That same phantom child also
    /// puts a flex `gap` between the icon and a zero-width element, which is what shifts
    /// the glyph `gap / 2` left of centre.
    #[gpui::test]
    fn icon_only_button_lays_out_square(cx: &mut gpui::TestAppContext) {
        let bounds = laid_out_bounds(cx, || {
            MoonButton::new("icon-only")
                .size(MoonButtonSize::ToolbarCompact)
                .leading_icon(MoonButtonIconSlot::new("icons/settings.svg"))
        });

        assert_eq!(
            bounds.size.width, bounds.size.height,
            "icon-only button is {:?} — not square, so the empty segment container and its \
             phantom gap are still there",
            bounds.size
        );
    }

    /// A trailing-only button is icon-only too: with no segments there is nothing for the
    /// icon to trail. It must reach the same square layout instead of being attached as a
    /// child, which would defeat `Button`'s icon-only path just like an empty container.
    #[gpui::test]
    fn trailing_only_icon_button_lays_out_square(cx: &mut gpui::TestAppContext) {
        let bounds = laid_out_bounds(cx, || {
            MoonButton::new("trailing-only")
                .size(MoonButtonSize::ToolbarCompact)
                .trailing_icon(MoonButtonIconSlot::new("icons/settings.svg"))
        });

        assert_eq!(
            bounds.size.width, bounds.size.height,
            "trailing-only icon button is {:?} — not square, so the icon is still being \
             attached as a child instead of filling the icon slot",
            bounds.size
        );
    }

    /// Two icons and no text is a genuine two-slot button, not an icon-only one — the
    /// promotion must not collapse it into a square and swallow one of the icons.
    #[gpui::test]
    fn leading_and_trailing_icons_keep_both_slots(cx: &mut gpui::TestAppContext) {
        let bounds = laid_out_bounds(cx, || {
            MoonButton::new("two-icons")
                .size(MoonButtonSize::ToolbarCompact)
                .leading_icon(MoonButtonIconSlot::new("icons/settings.svg"))
                .trailing_icon(MoonButtonIconSlot::new("icons/settings.svg"))
        });

        assert!(
            bounds.size.width > bounds.size.height,
            "two-icon button collapsed to {:?}",
            bounds.size
        );
    }

    /// The icon+label shape (the terminal's "Settings" button) carries one real segment,
    /// so it must keep its wide box and never be pulled into the icon-only square path.
    #[gpui::test]
    fn icon_with_label_button_stays_wide(cx: &mut gpui::TestAppContext) {
        let bounds = laid_out_bounds(cx, || {
            MoonButton::new("icon-and-label")
                .size(MoonButtonSize::ToolbarCompact)
                .leading_icon(MoonButtonIconSlot::new("icons/settings.svg"))
                .text_segment("Settings", 0xFFFFFF, 500.0)
        });

        assert!(
            bounds.size.width > bounds.size.height,
            "labelled button collapsed to {:?}",
            bounds.size
        );
    }
}
