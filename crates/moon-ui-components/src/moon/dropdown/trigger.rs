//! Dropdown trigger fitting, builder state, and anchored-popup composition.

use super::*;

/// Resolve and, when necessary, truncate one plain dropdown trigger label.
///
/// Args:
///     label: Full caller-supplied label without the component-owned suffix.
///     suffix: Required suffix, normally the dropdown caret.
///     width: Trigger width policy.
///     tokens: Active theme tokens.
///     font_size: Design-reference trigger font size.
///     reserved_content_width: Rendered leading content and gap width excluded from text fitting.
///     measure: Width function matching the rendered trigger font.
///
/// Returns:
///     The fitted label including `suffix` and an optional explicit rendered width.
pub(super) fn fit_dropdown_trigger_label(
    label: &str,
    suffix: &str,
    width: MoonDropdownTriggerWidth,
    tokens: &MoonThemeTokens,
    font_size: f32,
    reserved_content_width: f32,
    measure: impl Fn(&str) -> f32,
) -> (SharedString, Option<f32>) {
    let full = format!("{label}{suffix}");
    match width {
        MoonDropdownTriggerWidth::Intrinsic => return (SharedString::from(full), None),
        MoonDropdownTriggerWidth::Rendered(width) => {
            return (SharedString::from(full), Some(width));
        }
        MoonDropdownTriggerWidth::Scaled(_) | MoonDropdownTriggerWidth::Fit { .. } => {}
    }

    let text_scale = tokens.font(font_size) / font_size.max(1.0);
    let scaled = |value: f32| value * text_scale;
    let visual_padding = tokens.ui(DROPDOWN_TRIGGER_PAD_X) + reserved_content_width;
    let minimum_text = if label.is_empty() {
        suffix.to_string()
    } else {
        format!("\u{2026}{suffix}")
    };
    let minimum_width = visual_padding + measure(&minimum_text);
    match width {
        MoonDropdownTriggerWidth::Scaled(width) => {
            let width = scaled(width).max(minimum_width);
            let text_width = (width - visual_padding).max(0.0);
            let fitted = fit_text_with_suffix(label, suffix, text_width, measure).0;
            (SharedString::from(fitted), Some(width))
        }
        MoonDropdownTriggerWidth::Fit { min, max } => {
            let min = scaled(min).max(minimum_width);
            let max = scaled(max).max(min);
            let natural = (measure(&full) + visual_padding).ceil();
            let width = natural.clamp(min, max);
            let text_width = (width - visual_padding).max(0.0);
            let fitted = fit_text_with_suffix(label, suffix, text_width, measure).0;
            (SharedString::from(fitted), Some(width))
        }
        MoonDropdownTriggerWidth::Intrinsic | MoonDropdownTriggerWidth::Rendered(_) => {
            unreachable!("unmeasured trigger width handled before text measurement")
        }
    }
}

/// Retained open state for an uncontrolled dropdown trigger.
#[derive(Default)]
pub(super) struct MoonDropdownState {
    pub(super) open: bool,
}

#[derive(IntoElement)]
/// Moon-styled dropdown with component-owned trigger caret and width policies.
pub struct MoonDropdown {
    id: SharedString,
    bounds: Option<MoonRect>,
    label: SharedString,
    segments: Vec<MoonButtonSegment>,
    items: Vec<MoonMenuItem>,
    menu_layout: MenuLayoutFingerprint,
    trigger_variant: MoonButtonVariant,
    trigger_size: MoonButtonSize,
    trigger_leading_icon: Option<MoonButtonIconSlot>,
    trigger_width: MoonDropdownTriggerWidth,
    trigger_caret: bool,
    selected: bool,
    disabled: bool,
    default_open: bool,
    controlled_open: Option<bool>,
    menu_width: MoonMenuWidth,
    menu_offset_x: f32,
    menu_offset_y: f32,
    menu_size: MoonMenuSize,
    menu_max_height: Option<MoonMenuMaxHeight>,
    close_on_select: bool,
    on_select: Option<MoonSelectHandler>,
    on_open_change: Option<std::rc::Rc<dyn Fn(bool, &mut Window, &mut App)>>,
    menu_header: Option<std::rc::Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>>,
    menu_header_height: f32,
}

impl MoonDropdown {
    /// Create a dropdown with an intrinsic trigger and legacy rendered menu width.
    ///
    /// Args:
    ///     id: Stable element identity shared by the trigger and popup.
    ///
    /// Returns:
    ///     A default dropdown builder.
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            label: SharedString::from(""),
            segments: Vec::new(),
            items: Vec::new(),
            menu_layout: MenuLayoutFingerprint::new(),
            trigger_variant: MoonButtonVariant::Neutral,
            trigger_size: MoonButtonSize::Toolbar,
            trigger_leading_icon: None,
            trigger_width: MoonDropdownTriggerWidth::Intrinsic,
            trigger_caret: false,
            selected: false,
            disabled: false,
            default_open: false,
            controlled_open: None,
            menu_width: MoonMenuWidth::Rendered(160.0),
            menu_offset_x: 0.0,
            menu_offset_y: 4.0,
            menu_size: MoonMenuSize::Normal,
            menu_max_height: None,
            close_on_select: true,
            on_select: None,
            on_open_change: None,
            menu_header: None,
            menu_header_height: 0.0,
        }
    }

    /// Set explicit trigger bounds used by external fitted-layout callers.
    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    /// Set the plain-text trigger label.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }

    /// Append one rich segment to the trigger label.
    pub fn segment(mut self, segment: MoonButtonSegment) -> Self {
        self.segments.push(segment);
        self
    }

    /// Append one dropdown row and update its popup-layout signature in the same pass.
    ///
    /// Args:
    ///     item: Menu row to append.
    ///
    /// Returns:
    ///     The updated dropdown.
    pub fn item(mut self, item: MoonMenuItem) -> Self {
        self.menu_layout.push(item.kind);
        self.items.push(item);
        self
    }

    /// Append dropdown rows while accumulating their popup-layout signature.
    ///
    /// Args:
    ///     items: Menu rows in display order.
    ///
    /// Returns:
    ///     The updated dropdown.
    pub fn items(mut self, items: impl IntoIterator<Item = MoonMenuItem>) -> Self {
        for item in items {
            self.menu_layout.push(item.kind);
            self.items.push(item);
        }
        self
    }

    /// Set the Moon button visual variant used by the trigger.
    pub fn trigger_variant(mut self, variant: MoonButtonVariant) -> Self {
        self.trigger_variant = variant;
        self
    }

    /// Set the Moon button size used by the trigger.
    pub fn trigger_size(mut self, size: MoonButtonSize) -> Self {
        self.trigger_size = size;
        self
    }

    /// Set the trigger's leading icon from an asset path.
    ///
    /// Args:
    ///     path: Static asset path resolved by the Moon icon renderer.
    ///
    /// Returns:
    ///     The updated dropdown.
    pub fn trigger_icon(self, path: &'static str) -> Self {
        self.trigger_leading_icon(MoonButtonIconSlot::new(path))
    }

    /// Set the trigger's leading icon slot.
    ///
    /// Args:
    ///     icon: Configured Moon button icon slot rendered before trigger content.
    ///
    /// Returns:
    ///     The updated dropdown.
    pub fn trigger_leading_icon(mut self, icon: MoonButtonIconSlot) -> Self {
        self.trigger_leading_icon = Some(icon);
        self
    }

    /// Set a legacy rendered trigger width.
    ///
    /// Args:
    ///     width: Trigger width in rendered pixels.
    ///
    /// Returns:
    ///     The updated dropdown.
    pub fn trigger_width(mut self, width: f32) -> Self {
        self.trigger_width = MoonDropdownTriggerWidth::Rendered(width);
        self
    }

    /// Set a fixed design-reference trigger width scaled with the trigger's text metrics.
    ///
    /// Plain labels are truncated inside the resolved visual padding; segmented icon triggers
    /// receive only the scaled width.
    ///
    /// Args:
    ///     width: Trigger width at the configured font reference size.
    ///
    /// Returns:
    ///     The updated dropdown.
    pub fn trigger_width_scaled(mut self, width: f32) -> Self {
        self.trigger_width = MoonDropdownTriggerWidth::Scaled(width);
        self
    }

    /// Fit a plain-label trigger between font-scaled design-reference bounds.
    ///
    /// Args:
    ///     min_width: Minimum trigger width at the configured font reference size.
    ///     max_width: Maximum trigger width at the configured font reference size.
    ///
    /// Returns:
    ///     The updated dropdown.
    pub fn fit_trigger_width(mut self, min_width: f32, max_width: f32) -> Self {
        self.trigger_width = MoonDropdownTriggerWidth::Fit {
            min: min_width,
            max: max_width,
        };
        self
    }

    /// Show or hide the component-owned dropdown caret on a plain-label trigger.
    ///
    /// Args:
    ///     visible: Whether the caret is appended and reserved during fitting.
    ///
    /// Returns:
    ///     The updated dropdown.
    pub fn trigger_caret(mut self, visible: bool) -> Self {
        self.trigger_caret = visible;
        self
    }

    /// Set whether the trigger uses selected-state styling.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Set whether the trigger and its menu reject interaction.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set the initial open state used when the caller does not control the menu.
    pub fn default_open(mut self, default_open: bool) -> Self {
        self.default_open = default_open;
        self
    }

    /// Control the menu's open state from the caller.
    pub fn open(mut self, open: bool) -> Self {
        self.controlled_open = Some(open);
        self
    }

    /// Set a legacy rendered menu width.
    ///
    /// Args:
    ///     width: Outer menu width in rendered pixels.
    ///
    /// Returns:
    ///     The updated dropdown.
    pub fn menu_width(mut self, width: f32) -> Self {
        self.menu_width = MoonMenuWidth::Rendered(width);
        self
    }

    /// Set a fixed design-reference menu width scaled with the selected menu size.
    ///
    /// Args:
    ///     width: Outer width at the configured font reference size.
    ///
    /// Returns:
    ///     The updated dropdown.
    pub fn menu_width_scaled(mut self, width: f32) -> Self {
        self.menu_width = MoonMenuWidth::Scaled(width);
        self
    }

    /// Fit each menu level to its own items between font-scaled design-reference bounds.
    ///
    /// Args:
    ///     min_width: Minimum outer width at the configured font reference size.
    ///     max_width: Maximum outer width at the configured font reference size.
    ///
    /// Returns:
    ///     The updated dropdown.
    pub fn fit_menu_width(mut self, min_width: f32, max_width: f32) -> Self {
        self.menu_width = MoonMenuWidth::Fit {
            min: min_width,
            max: max_width,
        };
        self
    }

    /// Offset the popup from the trigger in design-reference units.
    pub fn menu_offset(mut self, x: f32, y: f32) -> Self {
        self.menu_offset_x = x;
        self.menu_offset_y = y;
        self
    }

    /// Set the density preset used by popup rows.
    pub fn menu_size(mut self, size: MoonMenuSize) -> Self {
        self.menu_size = size;
        self
    }

    /// Set a legacy maximum menu height in rendered pixels.
    ///
    /// Args:
    ///     max_height: Maximum rendered menu height.
    ///
    /// Returns:
    ///     The updated dropdown.
    pub fn menu_max_height(mut self, max_height: f32) -> Self {
        self.menu_max_height = Some(MoonMenuMaxHeight::Rendered(max_height));
        self
    }

    /// Set a UI-scaled design-reference maximum menu height.
    ///
    /// Args:
    ///     max_height: Maximum height at the configured UI reference scale.
    ///
    /// Returns:
    ///     The updated dropdown.
    pub fn menu_max_height_ui(mut self, max_height: f32) -> Self {
        self.menu_max_height = Some(MoonMenuMaxHeight::Ui(max_height));
        self
    }

    /// Pin an element above the menu's rows that does not scroll with them.
    ///
    /// The builder takes a closure rather than a built element because the popup's content is
    /// rebuilt on every render: a stored `AnyElement` could be consumed only once. The closure
    /// receives the window, so a header may own focusable content such as a search field.
    ///
    /// When a maximum height is configured, the header is laid out inside that limit rather than
    /// added on top of it: the row list gives up the space the header takes. The height is declared
    /// rather than measured because the row list is sized in pixels before its siblings are laid
    /// out; the virtualized width-fitting prefix reads the same budget, so an inaccurate
    /// declaration also mis-measures the menu's width.
    ///
    /// Calling this twice replaces the header, unlike [`MoonPopupMenu::header`], which appends.
    ///
    /// **The closure is retained for as long as the popup state is** — it is cloned into the
    /// popover's content closure, which outlives the frame. Capture weak handles and cheap values
    /// only: a strong view handle closes a `view -> popup state -> closure -> view` cycle and the
    /// view never drops, the same hazard the tree and virtual-list row builders carry.
    ///
    /// Args:
    ///     height_ui: Header height at the configured UI reference scale.
    ///     header: Builder invoked once per popup render.
    ///
    /// Returns:
    ///     The updated dropdown.
    pub fn header(
        mut self,
        height_ui: f32,
        header: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.menu_header = Some(std::rc::Rc::new(header));
        self.menu_header_height = height_ui;
        self
    }

    /// Set whether selecting a row closes the popup by default.
    pub fn close_on_select(mut self, close_on_select: bool) -> Self {
        self.close_on_select = close_on_select;
        self
    }

    /// Attach the callback invoked with the selected row key.
    pub fn on_select(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(std::rc::Rc::new(handler));
        self
    }

    /// Attach the callback invoked whenever the effective open state changes.
    pub fn on_open_change(
        mut self,
        handler: impl Fn(bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open_change = Some(std::rc::Rc::new(handler));
        self
    }

    /// Fit an external button label with the same caret, padding, and text metrics as a dropdown.
    ///
    /// This supports custom popover triggers that must align with a neighboring MoonDropdown
    /// without reproducing its private geometry. The external [`MoonButton`] must use
    /// `mono(true)` to match this measurement contract.
    ///
    /// Args:
    ///     cx: Application context used for active-theme text measurement.
    ///     label: Complete external trigger label without a caret.
    ///     size: MoonButton size used by the external trigger.
    ///     min_width: Minimum width at the configured font reference size.
    ///     max_width: Maximum width at the configured font reference size.
    ///
    /// Returns:
    ///     The fitted label with a component-owned caret and its rendered trigger width.
    pub fn fitted_trigger_label(
        cx: &App,
        label: &str,
        size: MoonButtonSize,
        min_width: f32,
        max_width: f32,
    ) -> (SharedString, f32) {
        let tokens = MoonTheme::active_tokens(cx);
        let (font_size, _, _) = button_text_metrics(size);
        let (label, width) = fit_dropdown_trigger_label(
            label,
            DROPDOWN_CARET,
            MoonDropdownTriggerWidth::Fit {
                min: min_width,
                max: max_width,
            },
            &tokens,
            font_size,
            0.0,
            |text| measure_text_width(cx, &tokens, text, font_size, 400.0, DROPDOWN_TRIGGER_MONO),
        );
        (
            label,
            width.expect("fitted trigger width always resolves to a rendered value"),
        )
    }

    /// Render the trigger after resolving its label and scaled width.
    ///
    /// Args:
    ///     cx: Application context used for active-theme text measurement.
    ///
    /// Returns:
    ///     The rendered MoonButton trigger.
    fn render_trigger(&self, cx: &App) -> impl IntoElement {
        let trigger_id = SharedString::from(format!("{}:trigger", self.id));
        let mut trigger = MoonButton::new(trigger_id)
            .variant(self.trigger_variant)
            .size(self.trigger_size)
            .selected(self.selected)
            .disabled(self.disabled);
        if self.segments.is_empty() {
            trigger = trigger.mono(DROPDOWN_TRIGGER_MONO);
        }

        let (font_size, _, _) = button_text_metrics(self.trigger_size);
        let tokens = MoonTheme::active_tokens(cx);
        let reserved_content_width = self.trigger_leading_icon.map_or(0.0, |_| {
            button_leading_icon_reservation(self.trigger_size, &tokens)
        });
        let suffix = if self.trigger_caret && self.segments.is_empty() {
            DROPDOWN_CARET
        } else {
            ""
        };
        let (label, resolved_width) = if self.segments.is_empty() {
            fit_dropdown_trigger_label(
                self.label.as_ref(),
                suffix,
                self.trigger_width,
                &tokens,
                font_size,
                reserved_content_width,
                |text| {
                    measure_text_width(cx, &tokens, text, font_size, 400.0, DROPDOWN_TRIGGER_MONO)
                },
            )
        } else {
            let width = match self.trigger_width {
                MoonDropdownTriggerWidth::Intrinsic => None,
                MoonDropdownTriggerWidth::Rendered(width) => Some(width),
                MoonDropdownTriggerWidth::Scaled(width) => {
                    Some(width * tokens.font(font_size) / font_size.max(1.0))
                }
                MoonDropdownTriggerWidth::Fit { min, .. } => {
                    Some(min * tokens.font(font_size) / font_size.max(1.0))
                }
            };
            (self.label.clone(), width)
        };

        if let Some(bounds) = self.bounds {
            trigger = trigger.bounds(MoonRect::new(0.0, 0.0, bounds.w, bounds.h));
        } else if let Some(width) = resolved_width {
            trigger = trigger.width(width);
        }

        if let Some(icon) = self.trigger_leading_icon {
            trigger = trigger.leading_icon(icon);
        }

        if self.segments.is_empty() {
            // Leave a truly empty icon trigger childless so MoonButton uses its square icon-only
            // layout instead of reserving text padding and a phantom content gap.
            if !label.is_empty() || self.trigger_leading_icon.is_none() {
                trigger = trigger.label(label);
            }
        } else {
            for segment in self.segments.clone() {
                trigger = trigger.segment(segment);
            }
        }

        trigger.render()
    }
}

impl RenderOnce for MoonDropdown {
    /// Renders the trigger and, while open, its deferred anchored menu.
    ///
    /// Args:
    ///     window: Window that owns keyed dropdown state and the deferred menu.
    ///     cx: Application context used for theme resolution and text measurement.
    ///
    /// Returns:
    ///     The rendered dropdown.
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state_id = ElementId::from(SharedString::from(format!("{}:moon-state", self.id)));
        let state = window.use_keyed_state(state_id, cx, |_, _| MoonDropdownState {
            open: self.default_open,
        });

        let controlled_open = self.controlled_open;
        let open = controlled_open.unwrap_or_else(|| state.read(cx).open);
        let on_open_change = self.on_open_change.clone();
        let parent_view = window.current_view();
        // How much the open menu must clear before its own gap. It depends on how the trigger is
        // laid out, so it is decided here rather than at the `.mt(..)` that consumes it.
        //
        // In flow (no caller-supplied bounds) the answer is ZERO: the anchor `CorePopover` hands
        // the menu already sits on the trigger's BOTTOM edge, because `ElementExt::on_prepaint`
        // measures an absolutely positioned canvas appended after the trigger, and in a block
        // container such a child takes its static position — below the preceding in-flow sibling.
        // Adding the height here too would push the menu down by a second trigger height.
        //
        // With caller-supplied bounds `MoonButton::bounds` renders the trigger ABSOLUTELY, which
        // leaves the popover host without a single in-flow child: its auto height collapses to
        // zero, the `size_full` canvas collapses with it, and the capture lands on the trigger's
        // TOP edge. Only there does the supplied height have to be added back.
        let trigger_height = self.bounds.map_or(0.0, |bounds| bounds.h);
        let trigger = self.render_trigger(cx).into_any_element();

        let mut root = div()
            .id(ElementId::from(SharedString::from(format!(
                "{}:root",
                self.id
            ))))
            .relative();

        if let Some(bounds) = self.bounds {
            root = root
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }

        if self.disabled {
            return root.child(trigger).into_any_element();
        }

        let menu_id = SharedString::from(format!("{}:menu", self.id));
        let items = self.items;
        let menu_layout = self.menu_layout;
        let menu_size = self.menu_size;
        let menu_width = self.menu_width;
        let menu_max_height = self.menu_max_height;
        let menu_offset_x = self.menu_offset_x;
        let menu_offset_y = self.menu_offset_y;
        let menu_header = self.menu_header.clone();
        let menu_header_height = self.menu_header_height;
        let popup_level = MoonMenuLevel::from_parts(std::rc::Rc::new(items), menu_layout);
        let dropdown_selection = std::rc::Rc::new(MoonDropdownSelectionContext::new(
            self.close_on_select,
            self.on_select,
            state.clone(),
            controlled_open,
            self.on_open_change.clone(),
            parent_view,
        ));

        let mut popover = CorePopover::new(ElementId::from(self.id.clone()))
            .appearance(false)
            .anchor(Anchor::TopLeft)
            .deferred_priority(30_000)
            .open(open)
            .trigger_any(trigger)
            .content(move |_, window, cx| {
                let tokens = MoonTheme::active_tokens(cx);
                let mut menu = MoonPopupMenu::new(menu_id.clone())
                    .shared_level(popup_level.clone())
                    .dropdown_selection(dropdown_selection.clone())
                    .size(menu_size)
                    .width_policy(menu_width)
                    .mono(true);
                if menu_width.is_measured() {
                    let viewport = window.viewport_size();
                    let viewport_max_width = (f32::from(viewport.width) - 16.0).max(1.0);
                    // The deferred anchor fits the content wrapper, and this menu sits below an
                    // in-wrapper top offset. Pay for that offset here or a nominal viewport-height
                    // menu still extends past the bottom by exactly the trigger compensation plus
                    // gap used below.
                    let popover_top_offset = trigger_height + tokens.ui(menu_offset_y);
                    let viewport_max_height =
                        (f32::from(viewport.height) - 16.0 - popover_top_offset).max(80.0);
                    let resolved_max_height = menu_max_height
                        .map(|height| height.resolve(&tokens))
                        .unwrap_or(viewport_max_height)
                        .min(viewport_max_height);
                    menu = menu
                        .rendered_max_width(viewport_max_width)
                        .max_height(resolved_max_height);
                } else if let Some(max_height) = menu_max_height {
                    menu = menu.max_height_policy(max_height);
                }
                // Built here rather than stored: this closure runs on every popup render, so a
                // retained element would be consumable only once.
                if let Some(header) = menu_header.as_ref() {
                    menu = menu.header(menu_header_height, header(window, cx));
                }

                div()
                    .mt(px(trigger_height + tokens.ui(menu_offset_y)))
                    .ml(px(tokens.ui(menu_offset_x)))
                    .child(menu)
            });

        {
            let state = state.clone();
            let on_open_change = on_open_change.clone();
            popover = popover.on_open_change(move |open, window, cx| {
                if let Some(on_open_change) = on_open_change.as_ref() {
                    on_open_change(*open, window, cx);
                }
                if controlled_open.is_none() {
                    state.update(cx, |state, _| {
                        state.open = *open;
                    });
                    cx.notify(parent_view);
                }
            });
        }

        root.child(popover).into_any_element()
    }
}
