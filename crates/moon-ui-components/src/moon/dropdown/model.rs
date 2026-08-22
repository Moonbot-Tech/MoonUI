//! Menu row models and immutable nested-menu storage.

use super::*;

/// Visual and interaction role of one popup-menu row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoonMenuItemKind {
    Item,
    Label,
    Separator,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Ordered menu-row signature accumulated during builder insertion.
pub(super) struct MenuLayoutFingerprint {
    pub(super) item_count: usize,
    pub(super) kind_hash: u64,
}

impl MenuLayoutFingerprint {
    /// Create the empty ordered menu-layout fingerprint.
    ///
    /// Returns:
    ///     A fingerprint ready to receive row kinds in display order.
    pub(super) fn new() -> Self {
        Self {
            item_count: 0,
            kind_hash: 0xcbf2_9ce4_8422_2325,
        }
    }

    /// Add one row kind without requiring a second pass over the completed item collection.
    ///
    /// Args:
    ///     kind: Visual role of the appended menu row.
    ///
    /// Returns:
    ///     Nothing; the fingerprint is updated in place.
    pub(super) fn push(&mut self, kind: MoonMenuItemKind) {
        let kind_tag = match kind {
            MoonMenuItemKind::Item => 1_u64,
            MoonMenuItemKind::Label => 2_u64,
            MoonMenuItemKind::Separator => 3_u64,
        };
        self.item_count += 1;
        self.kind_hash ^= kind_tag;
        self.kind_hash = self.kind_hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
}

impl Default for MenuLayoutFingerprint {
    /// Return the canonical empty menu-layout fingerprint.
    ///
    /// Returns:
    ///     The same value as [`Self::new`].
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
/// Shared immutable rows and their pre-accumulated variable-height layout signature.
pub(in crate::moon) struct MoonMenuLevel {
    pub(super) items: std::rc::Rc<Vec<MoonMenuItem>>,
    pub(super) layout: MenuLayoutFingerprint,
}

impl MoonMenuLevel {
    /// Create an empty menu level.
    ///
    /// Returns:
    ///     Shared empty row storage with the canonical empty layout.
    fn empty() -> Self {
        Self {
            items: std::rc::Rc::new(Vec::new()),
            layout: MenuLayoutFingerprint::new(),
        }
    }

    /// Build shared menu storage and its layout signature in one pass.
    ///
    /// Args:
    ///     items: Rows in display order.
    ///
    /// Returns:
    ///     A reusable immutable menu level.
    pub(in crate::moon) fn new(items: impl IntoIterator<Item = MoonMenuItem>) -> Self {
        let mut level = Self::empty();
        level.extend(items);
        level
    }

    /// Reuse already shared rows whose signature was accumulated by another builder.
    ///
    /// Args:
    ///     items: Shared rows in display order.
    ///     layout: Signature matching `items`.
    ///
    /// Returns:
    ///     A reusable immutable menu level.
    pub(super) fn from_parts(
        items: std::rc::Rc<Vec<MoonMenuItem>>,
        layout: MenuLayoutFingerprint,
    ) -> Self {
        Self { items, layout }
    }

    /// Append rows while extending the layout signature in the same pass.
    ///
    /// Args:
    ///     items: Rows to append in display order.
    ///
    /// Returns:
    ///     Nothing; this level is updated in place.
    pub(in crate::moon) fn extend(&mut self, items: impl IntoIterator<Item = MoonMenuItem>) {
        let target = std::rc::Rc::make_mut(&mut self.items);
        for item in items {
            self.layout.push(item.kind);
            target.push(item);
        }
    }

    /// Return the number of rows in this level.
    ///
    /// Returns:
    ///     Shared row count.
    pub(super) fn len(&self) -> usize {
        self.items.len()
    }

    /// Borrow the retained rows for sibling layout consumers.
    ///
    /// Returns:
    ///     The ordered immutable menu rows.
    pub(super) fn as_slice(&self) -> &[MoonMenuItem] {
        self.items.as_slice()
    }
}

/// Row geometry policy used by popup menus and dropdown-owned menus.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MoonMenuSize {
    Compact,
    Normal,
    Custom {
        row_height: f32,
        font_size: f32,
        line_height: f32,
        radius: f32,
        pad_x: f32,
        gap: f32,
    },
}

/// Immutable-render menu row with shared handlers and nested menu storage.
pub struct MoonMenuItem {
    pub(super) key: SharedString,
    pub(super) label: SharedString,
    pub(super) kind: MoonMenuItemKind,
    pub(super) right_label: Option<SharedString>,
    pub(super) tone: MoonTone,
    pub(super) selected: bool,
    pub(super) checked: bool,
    pub(super) disabled: bool,
    pub(super) actionable: bool,
    pub(super) submenu: MoonMenuLevel,
    pub(super) on_click: Option<MoonClickHandler>,
    /// Per-row override of the dropdown's `close_on_select`; `None` follows the dropdown.
    pub(super) closes_menu: Option<bool>,
}

impl Clone for MoonMenuItem {
    /// Clone one row model while letting regressions count repaint-time clone volume.
    ///
    /// Returns:
    ///     A row that shares immutable handlers and submenu storage with the source.
    fn clone(&self) -> Self {
        #[cfg(test)]
        if self.label.starts_with(MENU_CLONE_PROBE_PREFIX) {
            MENU_ITEM_CLONE_PROBE_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        Self {
            key: self.key.clone(),
            label: self.label.clone(),
            kind: self.kind,
            right_label: self.right_label.clone(),
            tone: self.tone,
            selected: self.selected,
            checked: self.checked,
            disabled: self.disabled,
            actionable: self.actionable,
            submenu: self.submenu.clone(),
            on_click: self.on_click.clone(),
            closes_menu: self.closes_menu,
        }
    }
}

/// Reset and return the menu-row clone probe used by virtual repaint regressions.
///
/// Returns:
///     Number of row clones recorded since the previous reset.
#[cfg(test)]
pub(in crate::moon) fn take_menu_item_clone_probe_count() -> usize {
    MENU_ITEM_CLONE_PROBE_COUNT.swap(0, Ordering::Relaxed)
}

impl MoonMenuItem {
    /// Create an enabled ordinary menu row whose key matches its label.
    ///
    /// Args:
    ///     label: Text and default selection key for the row.
    ///
    /// Returns:
    ///     A default actionable menu row.
    pub fn new(label: impl Into<SharedString>) -> Self {
        let label = label.into();
        Self {
            key: label.clone(),
            label,
            kind: MoonMenuItemKind::Item,
            right_label: None,
            tone: MoonTone::Default,
            selected: false,
            checked: false,
            disabled: false,
            actionable: true,
            submenu: MoonMenuLevel::empty(),
            on_click: None,
            closes_menu: None,
        }
    }

    /// Create an enabled ordinary menu row with a distinct selection key.
    ///
    /// Args:
    ///     key: Stable value reported by selection callbacks.
    ///     label: Text rendered for the row.
    ///
    /// Returns:
    ///     A default actionable menu row.
    pub fn with_key(key: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            kind: MoonMenuItemKind::Item,
            right_label: None,
            tone: MoonTone::Default,
            selected: false,
            checked: false,
            disabled: false,
            actionable: true,
            submenu: MoonMenuLevel::empty(),
            on_click: None,
            closes_menu: None,
        }
    }

    /// Create a disabled section-label row with no selection key.
    pub fn label(label: impl Into<SharedString>) -> Self {
        let mut item = Self::new(label);
        item.kind = MoonMenuItemKind::Label;
        item.disabled = true;
        item.actionable = false;
        item
    }

    /// Create an enabled section label that preserves label typography while accepting clicks.
    ///
    /// Args:
    ///     key: Stable selection key reported by dropdown callbacks.
    ///     label: Text rendered with section-label geometry and typography.
    ///
    /// Returns:
    ///     An enabled label row ready for an [`Self::on_click`] handler.
    pub fn action_label(key: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        let mut item = Self::with_key(key, label);
        item.kind = MoonMenuItemKind::Label;
        item
    }

    /// Create a compact inert separator row.
    ///
    /// Returns:
    ///     A disabled separator row with no interaction handler.
    pub fn separator() -> Self {
        Self {
            key: SharedString::from("separator"),
            label: SharedString::from(""),
            kind: MoonMenuItemKind::Separator,
            right_label: None,
            tone: MoonTone::Muted,
            selected: false,
            checked: false,
            disabled: true,
            actionable: false,
            submenu: MoonMenuLevel::empty(),
            on_click: None,
            closes_menu: None,
        }
    }

    /// Return the stable key reported by dropdown selection callbacks.
    pub fn key(&self) -> &SharedString {
        &self.key
    }

    /// Set the muted trailing text rendered at the row's right edge.
    ///
    /// Honoured by ordinary rows and by label rows alike, so a section heading or a click-only
    /// action row can carry a count without borrowing the checkbox column that would make it read
    /// as selectable state. On an ordinary row it also replaces the submenu chevron.
    ///
    /// Args:
    ///     right_label: Trailing text, typically a count or a shortcut.
    ///
    /// Returns:
    ///     The updated row.
    pub fn right_label(mut self, right_label: impl Into<SharedString>) -> Self {
        self.right_label = Some(right_label.into());
        self
    }

    /// Set the semantic tone used to render this row.
    pub fn tone(mut self, tone: MoonTone) -> Self {
        self.tone = tone;
        self
    }

    /// Set whether this row is the current single-select choice.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Set whether this row displays its checked indicator.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Set whether this row rejects pointer and keyboard activation.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Override, for this row alone, whether clicking it closes the menu.
    ///
    /// `MoonDropdown::close_on_select` is a whole-menu policy, which is wrong for a menu holding
    /// two kinds of row: checkbox rows that must leave a multi-select menu standing, and a row
    /// that opens a dialog. The second kind MUST take the menu down — a popup is deferred above
    /// the dialog layer, so a menu left open paints over the modal it just opened, and the first
    /// click into that modal both dismisses the menu and pulls focus back out of the dialog.
    ///
    /// Without this, the only way to mix the two in one menu is for the consumer to take over the
    /// dropdown's open state entirely, which costs it a mirrored flag and a retained callback in
    /// every hosting view.
    ///
    /// Args:
    ///     closes_menu: Whether a click on this row closes the menu, overriding the dropdown.
    ///
    /// Returns:
    ///     The row carrying its own close policy.
    pub fn closes_menu(mut self, closes_menu: bool) -> Self {
        self.closes_menu = Some(closes_menu);
        self
    }

    /// Attach an immutable nested menu and accumulate its layout signature in one pass.
    ///
    /// Args:
    ///     items: Nested rows in display order.
    ///
    /// Returns:
    ///     The updated parent row.
    pub fn submenu(mut self, items: impl IntoIterator<Item = MoonMenuItem>) -> Self {
        self.submenu = MoonMenuLevel::new(items);
        self
    }

    /// Return whether this row owns at least one nested menu row.
    ///
    /// Returns:
    ///     `true` when the shared nested level is non-empty.
    pub fn has_submenu(&self) -> bool {
        !self.submenu.items.is_empty()
    }

    /// Attach the handler invoked when this row is activated.
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(std::rc::Rc::new(handler));
        self
    }
}
