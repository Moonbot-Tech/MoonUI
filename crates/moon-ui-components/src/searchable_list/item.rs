use gpui::{
    AnyElement, App, ElementId, InteractiveElement as _, IntoElement, ParentElement, RenderOnce,
    StyleRefinement, Styled, Window, prelude::FluentBuilder,
};

use crate::{
    ActiveTheme, Disableable, Icon, IconName, Selectable, Sizable, Size, StyleSized, StyledExt,
    h_flex,
    moon::{
        MENU_CHECK_WIDTH, MoonMenuSize, MoonPalette, MoonTheme, foundation::selected_background,
        menu_row_metrics, rgba_from,
    },
};

/// How a searchable-list row is drawn.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum SearchableListRowLook {
    /// A drop-down's own row: input-sized geometry and a trailing selection tick.
    #[default]
    Input,
    /// A Moon menu row: menu geometry and a check mark in the fixed leading column.
    ///
    /// The check column is reserved whether or not the row is checked, so labels stay aligned and
    /// the cursor marker painted at the row's left edge falls inside it instead of on the first
    /// glyph. Row height, radius, padding and label size come from the compact menu metrics — that
    /// is what makes a popup opening beside dropdown menus indistinguishable from them.
    Menu,
}

/// A single row element used inside searchable-list dropdowns (Select, ComboBox, MultiComboBox).
///
/// - `selected` — controls the cursor-highlight background (the `List` overwrites this field via
///   `Selectable::selected` to match the keyboard cursor position).
/// - `checked` — controls the visibility of the check icon; set by the adapter based on the
///   current selection state and NOT overwritten by the `List`.
#[derive(IntoElement)]
pub struct SearchableListItemElement {
    id: ElementId,
    size: Size,
    style: StyleRefinement,
    /// Cursor/highlight background (overridden by `List` to the keyboard cursor row).
    selected: bool,
    /// Whether the check icon is shown.
    checked: bool,
    disabled: bool,
    children: Vec<AnyElement>,
    /// The icon drawn when `checked` is `true`.
    check_icon: Option<Icon>,
    /// Whether the row is drawn as a drop-down row or as a menu row.
    look: SearchableListRowLook,
    /// Whether the row heads a group instead of being one of its members.
    group: bool,
}

impl SearchableListItemElement {
    pub fn new(ix: usize) -> Self {
        Self {
            id: ("searchable-list-item", ix).into(),
            size: Size::default(),
            style: StyleRefinement::default(),
            selected: false,
            checked: false,
            disabled: false,
            children: Vec::new(),
            check_icon: Some(Icon::new(IconName::Check)),
            look: SearchableListRowLook::default(),
            group: false,
        }
    }

    /// Set whether the check icon is visible.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Override the default check icon.
    pub fn check_icon(mut self, icon: impl Into<Icon>) -> Self {
        self.check_icon = Some(icon.into());
        self
    }

    /// Choose whether the row is drawn as a drop-down row or as a menu row.
    pub fn look(mut self, look: SearchableListRowLook) -> Self {
        self.look = look;
        self
    }

    /// Mark the row as heading a group; only a menu-look row draws the distinction.
    pub fn group(mut self, group: bool) -> Self {
        self.group = group;
        self
    }
}

impl ParentElement for SearchableListItemElement {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Disableable for SearchableListItemElement {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Selectable for SearchableListItemElement {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl Sizable for SearchableListItemElement {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for SearchableListItemElement {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for SearchableListItemElement {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = MoonPalette::active(cx);
        let tokens = MoonTheme::active_tokens(cx);
        let menu_look = self.look == SearchableListRowLook::Menu;
        // A menu draws a group header without the check column, so it sits flush left while its
        // members are indented past theirs, and mutes its label.
        let group_row = menu_look && self.group;
        // A menu row is not an input row scaled down: its height, radius, padding and label size
        // come from the menu's own metrics, so both open from one filter row looking the same.
        let menu = menu_look.then(|| menu_row_metrics(MoonMenuSize::Compact, &tokens));
        h_flex()
            .id(self.id)
            .relative()
            .gap_x_1()
            .py_1()
            .px_2()
            .rounded(cx.theme().radius)
            .text_base()
            .text_color(cx.theme().foreground)
            .items_center()
            .justify_between()
            .input_text_size(self.size)
            .list_size(self.size)
            .when_some(menu, |this, metrics| {
                this.h(gpui::px(metrics.row_height))
                    .py_0()
                    .px(gpui::px(metrics.pad_x))
                    .rounded(gpui::px(metrics.radius))
                    .gap_x(gpui::px(metrics.gap))
                    .text_size(gpui::px(tokens.font(metrics.font_size)))
                    .line_height(gpui::px(tokens.line_height(metrics.line_height)))
            })
            .refine_style(&self.style)
            .when(!self.disabled, |this| {
                this.when(!self.selected, |this| {
                    this.hover(move |this| this.bg(rgba_from(p.overlay, 0.055)))
                })
            })
            .when(self.selected, |this| {
                // With a leading check column the cursor row is shown by its background alone,
                // exactly as a Moon menu shows it: the edge marker below would land inside that
                // column, on top of the check mark.
                this.bg(selected_background(p))
                    .text_color(gpui::rgb(p.selected_fg()))
                    .when(!menu_look, |this| {
                        this.child(
                            gpui::div()
                                .absolute()
                                .left(gpui::px(0.0))
                                .top(gpui::px(3.0))
                                .bottom(gpui::px(3.0))
                                .w(gpui::px(3.0))
                                .bg(gpui::rgb(p.accent)),
                        )
                        .child(
                            gpui::div()
                                .absolute()
                                .left(gpui::px(7.0))
                                .top(gpui::px(10.0))
                                .w(gpui::px(4.0))
                                .h(gpui::px(4.0))
                                .rounded_full()
                                .bg(gpui::rgb(p.accent)),
                        )
                    })
            })
            .when(group_row && !self.selected, |this| {
                this.text_color(rgba_from(p.text_muted, 0.88))
            })
            .when(self.disabled, |this| {
                this.cursor_not_allowed()
                    .text_color(cx.theme().muted_foreground)
            })
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .gap_x_1()
                    .when(menu_look && !group_row, |this| {
                        this.child(
                            gpui::div()
                                .w(gpui::px(tokens.ui(MENU_CHECK_WIDTH)))
                                .flex()
                                .flex_none()
                                .items_center()
                                .justify_center()
                                .when_some(self.check_icon.clone(), |this, icon| {
                                    this.child(
                                        icon.xsmall()
                                            .text_color(gpui::rgb(p.accent))
                                            .when(!self.checked, |this| this.invisible()),
                                    )
                                }),
                        )
                    })
                    .child(h_flex().w_full().items_center().children(self.children))
                    .when(!menu_look, |this| {
                        this.when_some(self.check_icon, |this, icon| {
                            this.child(
                                icon.xsmall()
                                    .text_color(cx.theme().foreground)
                                    .when(!self.checked, |this| this.invisible()),
                            )
                        })
                    }),
            )
    }
}
