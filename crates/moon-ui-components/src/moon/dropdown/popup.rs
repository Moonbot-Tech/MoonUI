//! Popup-menu rendering, virtualization state, and nested-menu composition.

use super::*;

/// Shared immutable inputs used to render every row in one menu level.
struct MenuLevelRenderContext {
    menu_id: SharedString,
    mono: bool,
    metrics: MenuMetrics,
    width_policy: MoonMenuWidth,
    size: MoonMenuSize,
    width: f32,
    truncate_labels: bool,
    palette: MoonPalette,
    tokens: MoonThemeTokens,
    dropdown_selection: Option<std::rc::Rc<MoonDropdownSelectionContext>>,
}

/// Retained variable-height list state for one large popup-menu level.
struct MoonPopupMenuVirtualState {
    list: ListState,
    layout: MenuLayoutFingerprint,
    metrics: MenuMetrics,
}

impl MoonPopupMenuVirtualState {
    /// Create retained list state matching one menu-level shape.
    ///
    /// Args:
    ///     layout: Ordered row-layout fingerprint accumulated while building the menu.
    ///     metrics: Scaled row metrics for ordinary items and labels.
    ///
    /// Returns:
    ///     A top-aligned variable-height list with two rows of overdraw.
    fn new(layout: MenuLayoutFingerprint, metrics: MenuMetrics) -> Self {
        Self {
            list: ListState::new(
                layout.item_count,
                ListAlignment::Top,
                px(metrics.row_height * 2.0),
            ),
            layout,
            metrics,
        }
    }

    /// Synchronize cached heights while retaining scroll position across ordinary repaints.
    ///
    /// Args:
    ///     layout: Current ordered row-layout fingerprint.
    ///     metrics: Current scaled row metrics.
    ///
    /// Returns:
    ///     A clone of the retained list handle for the current render.
    fn sync(&mut self, layout: MenuLayoutFingerprint, metrics: MenuMetrics) -> ListState {
        if self.layout != layout {
            self.list.reset(layout.item_count);
            self.layout = layout;
        } else if self.metrics != metrics {
            self.list.remeasure();
        }
        self.metrics = metrics;
        self.list.clone()
    }
}

/// Render a row's trailing text.
///
/// The render half of the trailing-text policy; measurement and fitting use
/// the same font-size delta and font weight in the layout module.
///
/// Args:
///     right_label: Already-fitted trailing text.
///     p: Active palette.
///     metrics: Resolved row geometry.
///     mono: Whether the row uses the configured monospaced font.
///     alpha: Row alpha the trailing text is dimmed against.
///
/// Returns:
///     The rendered trailing text.
fn menu_trailing_label(
    right_label: SharedString,
    p: MoonPalette,
    metrics: MenuMetrics,
    mono: bool,
    alpha: f32,
) -> AnyElement {
    MoonText::new(right_label)
        .color(p.text_muted)
        .alpha(alpha)
        .font_size(metrics.font_size - MENU_TRAILING_FONT_DELTA)
        .line_height(metrics.line_height)
        .weight(MENU_TRAILING_WEIGHT)
        .mono(mono)
        .uppercase(false)
        .render()
        .into_any_element()
}

/// Reset and return the last palette shell observed by a probe submenu row.
///
/// Returns:
///     Palette shell color, or zero when no probe row rendered.
#[cfg(test)]
pub(super) fn take_palette_probe_shell() -> u32 {
    MENU_PALETTE_PROBE_SHELL.swap(0, Ordering::Relaxed) as u32
}

#[derive(IntoElement)]
/// Moon-styled popup menu with rendered, scaled, or per-level fitted width policies.
pub struct MoonPopupMenu {
    id: SharedString,
    headers: Vec<(f32, AnyElement)>,
    items: std::rc::Rc<Vec<MoonMenuItem>>,
    pub(super) layout: MenuLayoutFingerprint,
    size: MoonMenuSize,
    width: MoonMenuWidth,
    rendered_max_width: Option<f32>,
    max_height: Option<MoonMenuMaxHeight>,
    mono: bool,
    dropdown_selection: Option<std::rc::Rc<MoonDropdownSelectionContext>>,
}

#[derive(IntoElement)]
/// Deferred menu renderer that preserves resolved parent theme values and retained list state.
struct MoonPopupMenuResolvedTheme {
    menu: MoonPopupMenu,
    palette: MoonPalette,
    tokens: MoonThemeTokens,
}

impl MoonPopupMenu {
    /// Create a popup menu with normal rows and the legacy rendered default width.
    ///
    /// Args:
    ///     id: Stable element identity used by rows and nested submenus.
    ///
    /// Returns:
    ///     A default popup-menu builder.
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            headers: Vec::new(),
            items: std::rc::Rc::new(Vec::new()),
            layout: MenuLayoutFingerprint::new(),
            size: MoonMenuSize::Normal,
            width: MoonMenuWidth::Rendered(160.0),
            rendered_max_width: None,
            max_height: None,
            mono: true,
            dropdown_selection: None,
        }
    }

    /// Append one menu row and update the retained-layout signature in the same pass.
    ///
    /// Args:
    ///     item: Menu row to append.
    ///
    /// Returns:
    ///     The updated menu.
    pub fn item(mut self, item: MoonMenuItem) -> Self {
        self.layout.push(item.kind);
        std::rc::Rc::make_mut(&mut self.items).push(item);
        self
    }

    /// Pin an element above the rows, outside the scrolling region.
    ///
    /// Repeatable; each header carries its own height. A header does not scroll, so the space it
    /// takes comes out of the row list's budget, and the height travels with the element rather
    /// than as a separate builder call — the row list is sized in pixels before its siblings are
    /// laid out, so it cannot discover the header on its own, and a header whose height was never
    /// declared would be pinned to zero and vanish.
    ///
    /// The height is a design-reference value. It is UI-scaled at render and initially floored by
    /// the menu's row height so text survives font growth; a configured outer cap may then shrink
    /// the wrapper before reducing the list below its one-row floor.
    ///
    /// Args:
    ///     height_ui: Header height at the configured UI reference scale.
    ///     header: Element rendered above the rows.
    ///
    /// Returns:
    ///     The updated menu.
    pub fn header(mut self, height_ui: f32, header: impl IntoElement) -> Self {
        self.headers.push((height_ui, header.into_any_element()));
        self
    }

    /// Append menu rows while accumulating their retained-layout signature.
    ///
    /// Args:
    ///     new_items: Menu rows in display order.
    ///
    /// Returns:
    ///     The updated menu.
    pub fn items(mut self, new_items: impl IntoIterator<Item = MoonMenuItem>) -> Self {
        let items = std::rc::Rc::make_mut(&mut self.items);
        for item in new_items {
            self.layout.push(item.kind);
            items.push(item);
        }
        self
    }

    /// Install an already shared menu level without cloning or rescanning its rows.
    ///
    /// Args:
    ///     level: Shared rows and their ordered layout signature.
    ///
    /// Returns:
    ///     The updated menu.
    pub(in crate::moon) fn shared_level(mut self, level: MoonMenuLevel) -> Self {
        self.items = level.items;
        self.layout = level.layout;
        self
    }

    /// Set the density preset used for menu row geometry and typography.
    pub fn size(mut self, size: MoonMenuSize) -> Self {
        self.size = size;
        self
    }

    /// Set a legacy rendered outer width.
    ///
    /// Args:
    ///     width: Outer width in rendered pixels.
    ///
    /// Returns:
    ///     The updated menu.
    pub fn width(mut self, width: f32) -> Self {
        self.width = MoonMenuWidth::Rendered(width);
        self
    }

    /// Set a fixed design-reference width that scales with this menu's text metrics.
    ///
    /// Labels are truncated inside the resolved row budget, including right labels and submenu
    /// glyphs.
    ///
    /// Args:
    ///     width: Outer width at the configured font reference size.
    ///
    /// Returns:
    ///     The updated menu.
    pub fn width_scaled(mut self, width: f32) -> Self {
        self.width = MoonMenuWidth::Scaled(width);
        self
    }

    /// Fit this menu level to its items between font-scaled design-reference bounds.
    ///
    /// Each submenu resolves the same policy independently from its own items.
    ///
    /// Args:
    ///     min_width: Minimum outer width at the configured font reference size.
    ///     max_width: Maximum outer width at the configured font reference size.
    ///
    /// Returns:
    ///     The updated menu.
    pub fn fit_width(mut self, min_width: f32, max_width: f32) -> Self {
        self.width = MoonMenuWidth::Fit {
            min: min_width,
            max: max_width,
        };
        self
    }

    /// Apply an already-selected width policy to a nested menu.
    ///
    /// Args:
    ///     width: Policy inherited by the submenu and resolved against its own rows.
    ///
    /// Returns:
    ///     The updated nested menu.
    pub(in crate::moon) fn width_policy(mut self, width: MoonMenuWidth) -> Self {
        self.width = width;
        self
    }

    /// Cap measured widths to an already-rendered viewport budget.
    ///
    /// Args:
    ///     max_width: Maximum outer width in rendered pixels.
    ///
    /// Returns:
    ///     The updated menu.
    pub(in crate::moon) fn rendered_max_width(mut self, max_width: f32) -> Self {
        self.rendered_max_width = Some(max_width);
        self
    }

    /// Set a legacy maximum height in rendered pixels.
    ///
    /// Args:
    ///     max_height: Maximum rendered menu height.
    ///
    /// Returns:
    ///     The updated menu.
    pub fn max_height(mut self, max_height: f32) -> Self {
        self.max_height = Some(MoonMenuMaxHeight::Rendered(max_height));
        self
    }

    /// Set a UI-scaled design-reference maximum menu height.
    ///
    /// Args:
    ///     max_height: Maximum height at the configured UI reference scale.
    ///
    /// Returns:
    ///     The updated menu.
    pub fn max_height_ui(mut self, max_height: f32) -> Self {
        self.max_height = Some(MoonMenuMaxHeight::Ui(max_height));
        self
    }

    /// Apply an already-selected maximum-height policy to a nested menu host.
    ///
    /// Args:
    ///     max_height: Maximum-height policy inherited from a dropdown.
    ///
    /// Returns:
    ///     The updated menu.
    pub(super) fn max_height_policy(mut self, max_height: MoonMenuMaxHeight) -> Self {
        self.max_height = Some(max_height);
        self
    }

    /// Set whether menu labels use the configured monospaced font.
    pub fn mono(mut self, mono: bool) -> Self {
        self.mono = mono;
        self
    }

    /// Attach root-dropdown behavior that visible rows resolve lazily.
    ///
    /// Args:
    ///     selection: Shared dropdown selection and dismissal behavior.
    ///
    /// Returns:
    ///     The updated root popup.
    pub(super) fn dropdown_selection(
        mut self,
        selection: std::rc::Rc<MoonDropdownSelectionContext>,
    ) -> Self {
        self.dropdown_selection = Some(selection);
        self
    }

    /// Resolve retained virtual-list state for this menu level when it crosses the threshold.
    ///
    /// Args:
    ///     tokens: Theme tokens used to calculate row metrics.
    ///     window: Window that owns keyed retained state.
    ///     cx: Application context used to create or update that state.
    ///
    /// Returns:
    ///     The retained list handle for a large menu, or `None` for an eager small menu.
    fn retained_virtual_list_state(
        &self,
        tokens: &MoonThemeTokens,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<ListState> {
        menu_level_is_virtualized(self.items.len()).then(|| {
            let metrics = self.metrics().scaled(tokens);
            window
                .use_keyed_state(
                    ElementId::from(SharedString::from(format!("{}:virtual-list", self.id))),
                    cx,
                    |_, _| MoonPopupMenuVirtualState::new(self.layout, metrics),
                )
                .update(cx, |state, _| state.sync(self.layout, metrics))
        })
    }

    /// Return this configured popup as an element for normal themed rendering.
    pub fn render(self) -> impl IntoElement {
        self
    }

    /// Render a legacy fixed-width menu with an explicit palette and default theme tokens.
    ///
    /// Measured policies require the active `App` text system and must render through
    /// [`Self::render`] instead.
    ///
    /// Args:
    ///     p: Palette used to paint the menu.
    ///
    /// Returns:
    ///     The rendered fixed-width menu.
    ///
    /// Panics:
    ///     Panics when called after [`Self::width_scaled`] or [`Self::fit_width`].
    pub fn render_with_palette(self, p: MoonPalette) -> AnyElement {
        assert!(
            matches!(self.width, MoonMenuWidth::Rendered(_)),
            "scaled or fitted menu widths require RenderOnce with an App context"
        );
        if !menu_level_is_virtualized(self.items.len()) {
            return self.render_with_theme(p, MoonThemeTokens::default(), None, None);
        }
        MoonPopupMenuResolvedTheme {
            menu: self,
            palette: p,
            tokens: MoonThemeTokens::default(),
        }
        .into_any_element()
    }

    /// Render with explicit palette/tokens and an optional text-measurement context.
    ///
    /// Args:
    ///     p: Palette used to paint the menu.
    ///     tokens: Tokens used to resolve menu geometry.
    ///     cx: Optional application context; fitted policies require it for text measurement.
    ///     virtual_list_state: Retained variable-height state for a large menu level.
    ///
    /// Returns:
    ///     The rendered menu.
    fn render_with_theme(
        self,
        p: MoonPalette,
        tokens: MoonThemeTokens,
        cx: Option<&App>,
        virtual_list_state: Option<ListState>,
    ) -> AnyElement {
        let metrics = self.metrics().scaled(&tokens);
        self.render_with_metrics(p, metrics, tokens, cx, virtual_list_state)
    }

    /// Renders the menu with precomputed layout metrics and the supplied theme tokens.
    ///
    /// Args:
    ///     p: Palette used to paint the menu.
    ///     metrics: Resolved menu row metrics.
    ///     tokens: Active theme tokens.
    ///     cx: Optional application context used by fitted policies.
    ///     virtual_list_state: Retained variable-height state, or `None` for eager rendering.
    ///
    /// Returns:
    ///     The rendered menu.
    fn render_with_metrics(
        self,
        p: MoonPalette,
        metrics: MenuMetrics,
        tokens: MoonThemeTokens,
        cx: Option<&App>,
        virtual_list_state: Option<ListState>,
    ) -> AnyElement {
        let id = self.id.clone();
        // The capped eager row list needs an identity distinct from the outer menu to retain its
        // scroll offset. Clone it before `self` is consumed by the branch-specific row rendering.
        let id_for_rows = self.id.clone();
        let mono = self.mono;
        let virtualized = virtual_list_state.is_some();

        // Resolve this before measurement because pinned headers reduce both the row list's height
        // and the initial-row budget used to fit a virtualized menu's width. Each header also costs
        // one gap because the outer flex column separates every child.
        let menu_gap = tokens.ui(MENU_GAP);
        let resolved_outer_max = resolve_menu_outer_max(self.max_height, &tokens, virtualized);
        let requested_heights: Vec<f32> = self
            .headers
            .iter()
            .map(|(height_ui, _)| tokens.ui(*height_ui).max(metrics.row_height))
            .collect();
        let header_gaps = menu_gap * requested_heights.len() as f32;
        let requested_total = requested_heights.iter().sum::<f32>();
        let header_budget = clamp_header_budget(
            requested_total + header_gaps,
            resolved_outer_max,
            menu_outer_chrome(&tokens),
            metrics.row_height,
        );
        // The clamp only affects layout if the elements follow it. Spending the surviving budget
        // back onto the wrappers is what makes the header yield, as `clamp_header_budget` promises;
        // pinning them to the requested heights instead would let header plus list overrun the
        // menu's maximum and the outer `overflow_hidden` would clip the rows the clamp just saved.
        let header_heights: Vec<f32> = if requested_total > 0.0 {
            let scale = ((header_budget - header_gaps).max(0.0) / requested_total).min(1.0);
            requested_heights
                .iter()
                .map(|height| height * scale)
                .collect()
        } else {
            requested_heights
        };
        let content_max = menu_content_max(resolved_outer_max, &tokens, header_budget, metrics);

        let (mut width, mut truncate_labels) = if let Some(cx) = cx {
            if virtualized {
                resolve_virtual_menu_width(
                    self.width,
                    &self.items,
                    metrics,
                    &tokens,
                    cx,
                    mono,
                    content_max,
                )
            } else {
                resolve_menu_width(self.width, &self.items, metrics, &tokens, cx, mono)
            }
        } else {
            match self.width {
                MoonMenuWidth::Rendered(width) => (width, false),
                MoonMenuWidth::Scaled(_) | MoonMenuWidth::Fit { .. } => {
                    unreachable!("measured menu width reached a renderer without an App context")
                }
            }
        };
        if self.width.is_measured()
            && let Some(max_width) = self.rendered_max_width
            && width > max_width
        {
            width = max_width.max(1.0);
            truncate_labels = true;
        }
        let shadow = super::super::foundation::box_shadow(
            px(0.0),
            px(8.0),
            px(18.0),
            px(0.0),
            rgba_from(p.shadow, 0.46),
        );

        let mut menu = div()
            .id(ElementId::from(self.id.clone()))
            // Addressable from `VisualTestContext::debug_bounds` so a test can assert where the
            // menu lands after the deferred/anchored pass. Field, setter and paint-time record are
            // all `cfg`-gated to test builds, so this costs a release build nothing.
            .debug_selector(|| id.to_string())
            .relative()
            .w(px(width))
            .p(px(tokens.ui(MENU_PADDING)))
            .rounded(px(tokens.ui(5.0)))
            .border(px(1.0))
            .border_color(rgba_from(p.border, 1.0))
            .bg(rgba_from(p.shell_high, 0.98))
            .shadow(vec![shadow])
            .occlude()
            .flex()
            .flex_col()
            .gap(px(tokens.ui(MENU_GAP)));

        // The cap belongs to the whole menu, but the scroll belongs to the rows alone: a header
        // that scrolled with them would leave the menu the moment the list is longer than the cap,
        // which is exactly the case a header exists for.
        let capped = self.max_height.is_some();
        if virtual_list_state.is_none() && capped {
            menu = menu.max_h(px(resolved_outer_max)).overflow_hidden();
        }

        for ((_, header), height) in self.headers.into_iter().zip(header_heights) {
            // Pinned to exactly the height its budget reserved, so the declaration cannot drift
            // from the layout in either direction: an over-declared header would otherwise shrink
            // the row list without using the space, and an under-declared one would push the list
            // past the menu's maximum.
            menu = menu.child(
                div()
                    .flex_none()
                    .h(px(height))
                    .overflow_hidden()
                    .child(header),
            );
        }

        let row_context = std::rc::Rc::new(MenuLevelRenderContext {
            menu_id: id,
            mono,
            metrics,
            width_policy: self.width,
            size: self.size,
            width,
            truncate_labels,
            palette: p,
            tokens,
            dropdown_selection: self.dropdown_selection,
        });
        if let Some(list_state) = virtual_list_state {
            let list_height =
                virtual_menu_list_height(&self.items, metrics, &row_context.tokens, content_max);
            let item_count = self.items.len();
            let items = self.items;
            let row_context = row_context.clone();
            menu = menu.max_h(px(resolved_outer_max)).overflow_hidden().child(
                list(list_state, move |ix, _window, _cx| {
                    let item = items[ix].clone();
                    div()
                        .w_full()
                        .when(ix + 1 < item_count, |row| {
                            row.mb(px(row_context.tokens.ui(MENU_GAP)))
                        })
                        .child(Self::render_item(&row_context, ix, item, Some(_cx)))
                        .into_any_element()
                })
                .h(px(list_height))
                .w_full(),
            );
        } else {
            let items = std::rc::Rc::try_unwrap(self.items)
                .unwrap_or_else(|shared_items| shared_items.as_ref().clone());
            // Only a capped menu needs a scrolling container of its own; an uncapped one keeps its
            // natural height and its rows stay direct children, with no extra layout node and no
            // retained element state to key.
            if capped {
                let mut rows = div()
                    .id(ElementId::from(SharedString::from(format!(
                        "{}:rows",
                        id_for_rows
                    ))))
                    .flex()
                    .flex_col()
                    .gap(px(menu_gap))
                    .min_h_0()
                    .flex_1()
                    .overflow_y_scroll();
                for (ix, item) in items.into_iter().enumerate() {
                    rows = rows.child(Self::render_item(&row_context, ix, item, cx));
                }
                menu = menu.child(rows);
            } else {
                for (ix, item) in items.into_iter().enumerate() {
                    menu = menu.child(Self::render_item(&row_context, ix, item, cx));
                }
            }
        }

        menu.into_any_element()
    }

    /// Render one menu row and recursively render its selected submenu.
    ///
    /// Args:
    ///     context: Immutable level-wide identity, geometry, and theme inputs.
    ///     ix: Zero-based row index.
    ///     item: Row model.
    ///     cx: Optional application context used by measured width policies.
    ///
    /// Returns:
    ///     The rendered row element.
    fn render_item(
        context: &MenuLevelRenderContext,
        ix: usize,
        item: MoonMenuItem,
        cx: Option<&App>,
    ) -> AnyElement {
        let menu_id = &context.menu_id;
        let mono = context.mono;
        let metrics = context.metrics;
        let menu_width_policy = context.width_policy;
        let menu_size = context.size;
        let p = context.palette;
        let tokens = &context.tokens;
        let row_id = SharedString::from(format!("{}:item:{}", menu_id, ix));
        let mut item = item;
        #[cfg(test)]
        if item.label.starts_with(MENU_PALETTE_PROBE_PREFIX) {
            MENU_PALETTE_PROBE_SHELL.store(context.palette.shell as usize, Ordering::Relaxed);
        }
        if context.truncate_labels {
            fit_menu_item_label(
                &mut item,
                context.width,
                metrics,
                tokens,
                cx.expect("measured menu row requires an App context"),
                mono,
                menu_outer_chrome(tokens),
            );
        }

        match item.kind {
            MoonMenuItemKind::Separator => div()
                .id(ElementId::from(row_id.clone()))
                .debug_selector(move || row_id.to_string())
                .h(px(1.0))
                .mx(px(2.0))
                .my(px(3.0))
                .bg(rgba_from(p.border, 0.82))
                .into_any_element(),
            MoonMenuItemKind::Label => {
                let disabled = item.disabled;
                let actionable = moon_menu_item_accepts_click(
                    MoonMenuItemKind::Label,
                    disabled,
                    item.actionable,
                );
                let on_click = menu_item_click_handler(&item, context.dropdown_selection.as_ref());
                let mut row = div()
                    .id(ElementId::from(row_id.clone()))
                    .debug_selector({
                        let row_id = row_id.clone();
                        move || row_id.to_string()
                    })
                    .h(px(metrics.row_height))
                    .rounded(px(metrics.radius))
                    .px(px(metrics.pad_x))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(metrics.gap))
                    .when(actionable, |this| {
                        this.hover(move |this| this.bg(rgba_from(p.overlay, 0.055)))
                            .active(move |this| this.bg(rgba_from(p.overlay, 0.032)))
                    })
                    .child(
                        MoonText::new(item.label)
                            .color(p.text_muted)
                            .alpha(0.88)
                            .font_size(metrics.font_size)
                            .line_height(metrics.line_height)
                            .weight(500.0)
                            .mono(mono)
                            .uppercase(false)
                            .render(),
                    );

                // A label row carries the same trailing count an ordinary row can. The count sits
                // at the row's own muted alpha rather than taking the ordinary row's further
                // opacity reduction: the whole row is already secondary chrome, and dimming it
                // again would push it under the rows it is meant to be read against.
                //
                // `justify_between` plus the row's own gap, rather than a flexible spacer: that
                // renders exactly the one gap the width measurement charges for this row kind.
                if let Some(right_label) = item.right_label {
                    row = row.child(menu_trailing_label(right_label, p, metrics, mono, 0.88));
                }

                if actionable {
                    if let Some(on_click) = on_click {
                        row = row
                            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                cx.stop_propagation();
                            })
                            .on_click(move |event, window, cx| {
                                on_click(event, window, cx);
                            });
                    }
                }

                row.into_any_element()
            }
            MoonMenuItemKind::Item => {
                let disabled = item.disabled;
                let selected = item.selected;
                let checked = item.checked;
                let on_click = menu_item_click_handler(&item, context.dropdown_selection.as_ref());
                let submenu = item.submenu;
                let has_submenu = !submenu.items.is_empty();
                let fg = if disabled {
                    p.text_muted
                } else if selected {
                    p.selected_fg()
                } else {
                    item.tone.color(p)
                };
                let alpha = if disabled { 0.45 } else { 1.0 };

                let mut row = div()
                    .id(ElementId::from(row_id.clone()))
                    .debug_selector({
                        let row_id = row_id.clone();
                        move || row_id.to_string()
                    })
                    .relative()
                    .h(px(metrics.row_height))
                    .rounded(px(metrics.radius))
                    .px(px(metrics.pad_x))
                    .flex()
                    .items_center()
                    .gap(px(metrics.gap))
                    .cursor_default()
                    .when(selected, |this| this.bg(selected_background(p)))
                    .when(!disabled, |this| {
                        this.hover(move |this| this.bg(rgba_from(p.overlay, 0.055)))
                            .active(move |this| this.bg(rgba_from(p.overlay, 0.032)))
                    })
                    .child(
                        div()
                            .w(px(menu_check_width(&tokens)))
                            .h(px(tokens.line_height(metrics.line_height)))
                            .flex()
                            .items_center()
                            .justify_center()
                            .when(checked, |this| {
                                this.child(moon_icon(
                                    MOON_ICON_CHECK,
                                    tokens.ui(11.0),
                                    p.accent,
                                    alpha,
                                ))
                            }),
                    )
                    .child(
                        MoonText::new(item.label)
                            .color(fg)
                            .alpha(alpha)
                            .font_size(metrics.font_size)
                            .line_height(metrics.line_height)
                            .weight(if selected { 600.0 } else { 400.0 })
                            .mono(mono)
                            .uppercase(false)
                            .render(),
                    )
                    .child(div().flex_1());

                if let Some(right_label) = item.right_label {
                    row = row.child(menu_trailing_label(
                        right_label,
                        p,
                        metrics,
                        mono,
                        alpha * 0.88,
                    ));
                } else if has_submenu {
                    row = row.child(
                        MoonText::new("›")
                            .color(p.text_muted)
                            .alpha(alpha * 0.88)
                            .font_size(metrics.font_size)
                            .line_height(metrics.line_height)
                            .weight(600.0)
                            .mono(mono)
                            .uppercase(false)
                            .render(),
                    );
                }

                if moon_menu_item_accepts_click(MoonMenuItemKind::Item, disabled, false) {
                    if let Some(on_click) = on_click {
                        row = row
                            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                cx.stop_propagation();
                            })
                            .on_click(move |event, window, cx| {
                                on_click(event, window, cx);
                            });
                    }
                }

                if selected && has_submenu {
                    row = row.child(
                        deferred(
                            div()
                                .absolute()
                                .left_full()
                                .ml(px(tokens.ui(SUBMENU_OFFSET_X)))
                                .top(px(-tokens.ui(MENU_PADDING)))
                                .child(MoonPopupMenuResolvedTheme {
                                    menu: MoonPopupMenu::new(format!("{menu_id}:submenu:{ix}"))
                                        .shared_level(submenu)
                                        .width_policy(menu_width_policy)
                                        .size(menu_size),
                                    palette: p,
                                    tokens: tokens.clone(),
                                }),
                        )
                        .with_priority(1),
                    );
                }

                row.into_any_element()
            }
        }
    }

    /// Return unscaled row metrics for the configured menu size.
    ///
    /// Returns:
    ///     Design-reference row metrics.
    pub(super) fn metrics(&self) -> MenuMetrics {
        unscaled_menu_metrics(self.size)
    }
}

impl RenderOnce for MoonPopupMenu {
    /// Render the menu with active theme tokens and measured fitted widths.
    ///
    /// Args:
    ///     window: Window that owns retained state for large menu levels.
    ///     cx: Application context used for active-theme resolution and text measurement.
    ///
    /// Returns:
    ///     The rendered menu.
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = MoonTheme::active_tokens(cx);
        let virtual_list_state = self.retained_virtual_list_state(&tokens, window, cx);
        self.render_with_theme(
            MoonPalette::active(cx),
            tokens,
            Some(cx),
            virtual_list_state,
        )
    }
}

impl RenderOnce for MoonPopupMenuResolvedTheme {
    /// Render a resolved-theme menu while retaining its virtual-list scroll state by menu id.
    ///
    /// Args:
    ///     window: Window that owns the keyed list state.
    ///     cx: Application context used to create or update retained state.
    ///
    /// Returns:
    ///     The popup rendered with the inherited palette and theme tokens.
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let virtual_list_state = self
            .menu
            .retained_virtual_list_state(&self.tokens, window, cx);
        self.menu
            .render_with_theme(self.palette, self.tokens, Some(cx), virtual_list_state)
    }
}
