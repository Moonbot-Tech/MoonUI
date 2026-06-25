use std::{cell::RefCell, collections::HashSet, ops::Range, rc::Rc};

use gpui::{
    AnyElement, App, Context, Div, ElementId, Entity, EventEmitter, FocusHandle,
    InteractiveElement as _, IntoElement, KeyBinding, ListSizingBehavior, Modifiers, MouseButton,
    MouseDownEvent, ParentElement, Pixels, Point, Render, RenderOnce, SharedString, Stateful,
    StatefulInteractiveElement as _, StyleRefinement, Styled, UniformListScrollHandle, Window, div,
    prelude::FluentBuilder as _, uniform_list,
};

use crate::{
    Selectable as _, StyledExt,
    actions::{Confirm, SelectDown, SelectLeft, SelectRight, SelectUp},
    list::ListItem,
    menu::{ContextMenuExt as _, PopupMenu},
    scroll::ScrollableElement,
};

const CONTEXT: &str = "Tree";
type RowRenderer =
    Rc<dyn Fn(&TreeEntry, TreeRowMeta, &mut Window, &mut App) -> AnyElement + 'static>;
type ContextMenuBuilder = Rc<
    dyn Fn(usize, &TreeEntry, PopupMenu, &mut Window, &mut Context<TreeState>) -> PopupMenu
        + 'static,
>;
type RowDecorator = Rc<
    dyn Fn(Stateful<Div>, &TreeEntry, TreeRowMeta, &mut Window, &mut App) -> Stateful<Div>
        + 'static,
>;
pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("up", SelectUp, Some(CONTEXT)),
        KeyBinding::new("down", SelectDown, Some(CONTEXT)),
        KeyBinding::new("left", SelectLeft, Some(CONTEXT)),
        KeyBinding::new("right", SelectRight, Some(CONTEXT)),
    ]);
}

/// Create a [`Tree`].
///
/// # Arguments
///
/// * `state` - The shared state managing the tree items.
/// * `render_item` - A closure to render each tree item.
///
/// ```ignore
/// let state = cx.new(|_| {
///     TreeState::new().items(vec![
///         TreeItem::new("src")
///             .child(TreeItem::new("lib.rs"),
///         TreeItem::new("Cargo.toml"),
///         TreeItem::new("README.md"),
///     ])
/// });
///
/// tree(&state, |ix, entry, selected, window, cx| {
///     let item = entry.item();
///     ListItem::new(ix).pl(px(16.) * entry.depth()).child(item.label.clone())
/// })
/// ```
pub fn tree<R>(state: &Entity<TreeState>, render_item: R) -> Tree
where
    R: Fn(usize, &TreeEntry, bool, &mut Window, &mut App) -> ListItem + 'static,
{
    Tree::new(state, render_item)
}

/// Selection behavior for a tree.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TreeSelectionMode {
    /// Only one visible entry can be selected.
    #[default]
    Single,
    /// Multiple entries can be selected with Shift and the platform secondary modifier.
    Multi,
}

/// Metadata passed to a custom tree row renderer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeRowMeta {
    pub index: usize,
    pub selected: bool,
    pub right_clicked: bool,
}

struct TreeItemState {
    expanded: bool,
    disabled: bool,
    folder: Option<bool>,
}

/// A tree item with a label, children, and an expanded state.
#[derive(Clone)]
pub struct TreeItem {
    pub id: SharedString,
    pub label: SharedString,
    pub children: Vec<TreeItem>,
    state: Rc<RefCell<TreeItemState>>,
}

/// A flat representation of a tree item with its depth.
#[derive(Clone)]
pub struct TreeEntry {
    item: TreeItem,
    depth: usize,
    expanded: bool,
}

impl TreeEntry {
    /// Get the source tree item.
    #[inline]
    pub fn item(&self) -> &TreeItem {
        &self.item
    }

    /// The depth of this item in the tree.
    #[inline]
    pub fn depth(&self) -> usize {
        self.depth
    }

    #[inline]
    pub fn is_root(&self) -> bool {
        self.depth == 0
    }

    /// Whether this item is a folder (has children).
    #[inline]
    pub fn is_folder(&self) -> bool {
        self.item.is_folder()
    }

    /// Return true if the item is expanded.
    #[inline]
    pub fn is_expanded(&self) -> bool {
        self.expanded
    }

    #[inline]
    pub fn is_disabled(&self) -> bool {
        self.item.is_disabled()
    }
}

/// Event emitted by [`TreeState`] when user-visible state changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TreeEvent {
    /// A tree node was expanded.
    Expanded(SharedString),
    /// A tree node was collapsed.
    Collapsed(SharedString),
    /// A row received a mouse down event in the built-in interaction mode.
    RowClicked {
        id: SharedString,
        modifiers: Modifiers,
        button: MouseButton,
    },
    /// A folder row requested expand/collapse.
    ChevronClicked { id: SharedString },
    /// A row was activated by keyboard or double-click style interaction.
    Activated(SharedString),
}

impl TreeItem {
    /// Create a new tree item with the given label.
    ///
    /// - The `id` for you to uniquely identify this item, then later you can use it for selection or other purposes.
    /// - The `label` is the text to display for this item.
    ///
    /// For example, the `id` is the full file path, and the `label` is the file name.
    ///
    /// ```ignore
    /// TreeItem::new("src/ui/button.rs", "button.rs")
    /// ```
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            children: Vec::new(),
            state: Rc::new(RefCell::new(TreeItemState {
                expanded: false,
                disabled: false,
                folder: None,
            })),
        }
    }

    /// Unique identifier supplied when the item was created.
    pub fn id(&self) -> &SharedString {
        &self.id
    }

    /// Visible label supplied when the item was created.
    pub fn label(&self) -> &SharedString {
        &self.label
    }

    /// Add a child item to this tree item.
    pub fn child(mut self, child: TreeItem) -> Self {
        self.children.push(child);
        self
    }

    /// Add multiple child items to this tree item.
    pub fn children(mut self, children: impl IntoIterator<Item = TreeItem>) -> Self {
        self.children.extend(children);
        self
    }

    /// Set expanded state for this tree item.
    pub fn expanded(self, expanded: bool) -> Self {
        self.state.borrow_mut().expanded = expanded;
        self
    }

    /// Set disabled state for this tree item.
    pub fn disabled(self, disabled: bool) -> Self {
        self.state.borrow_mut().disabled = disabled;
        self
    }

    /// Explicitly mark this item as a folder or a leaf.
    ///
    /// This is required for empty folders: without an explicit flag an item is
    /// considered a folder only when it has children.
    pub fn folder(self, folder: bool) -> Self {
        self.state.borrow_mut().folder = Some(folder);
        self
    }

    /// Whether this item is a folder (has children).
    #[inline]
    pub fn is_folder(&self) -> bool {
        self.state
            .borrow()
            .folder
            .unwrap_or_else(|| !self.children.is_empty())
    }

    /// Return true if the item is disabled.
    pub fn is_disabled(&self) -> bool {
        self.state.borrow().disabled
    }

    /// Return true if the item is expanded.
    #[inline]
    pub fn is_expanded(&self) -> bool {
        self.state.borrow().expanded
    }

    fn find_ancestors(&self, target_id: &SharedString) -> Option<Vec<TreeItem>> {
        if self.id == *target_id {
            return Some(vec![]);
        }

        for child in &self.children {
            if let Some(mut path) = child.find_ancestors(target_id) {
                path.push(self.clone());
                return Some(path);
            }
        }

        None
    }
}

/// State for managing tree items.
pub struct TreeState {
    focus_handle: FocusHandle,
    root_items: Vec<TreeItem>,
    entries: Vec<TreeEntry>,
    scroll_handle: UniformListScrollHandle,
    selected_ix: Option<usize>,
    selected_ids: HashSet<SharedString>,
    anchor_id: Option<SharedString>,
    expanded_ids: HashSet<SharedString>,
    force_expanded: bool,
    selection_mode: TreeSelectionMode,
    right_clicked_ix: Option<usize>,
    render_row: RowRenderer,
    row_decorators: Vec<RowDecorator>,
    custom_rows: bool,
    context_menu_builder: Option<ContextMenuBuilder>,
}

impl EventEmitter<TreeEvent> for TreeState {}

impl TreeState {
    /// Create a new empty tree state.
    pub fn new(cx: &mut App) -> Self {
        Self {
            selected_ix: None,
            selected_ids: HashSet::new(),
            anchor_id: None,
            expanded_ids: HashSet::new(),
            force_expanded: false,
            selection_mode: TreeSelectionMode::Single,
            right_clicked_ix: None,
            focus_handle: cx.focus_handle(),
            scroll_handle: UniformListScrollHandle::default(),
            root_items: Vec::new(),
            entries: Vec::new(),
            render_row: Rc::new(|entry, meta, _, _| {
                ListItem::new(meta.index)
                    .child(entry.item().label.clone())
                    .into_any_element()
            }),
            row_decorators: Vec::new(),
            custom_rows: false,
            context_menu_builder: None,
        }
    }

    /// Set the tree items.
    pub fn items(mut self, items: impl Into<Vec<TreeItem>>) -> Self {
        self.root_items = items.into();
        self.collect_item_expansion_flags();
        self.rebuild_entries();
        self.reconcile_selection();
        self
    }

    /// Set the tree items.
    pub fn set_items(&mut self, items: impl Into<Vec<TreeItem>>, cx: &mut Context<Self>) {
        self.root_items = items.into();
        self.collect_item_expansion_flags();
        self.rebuild_entries();
        self.reconcile_selection();
        self.right_clicked_ix = None;
        cx.notify();
    }

    /// Get the currently selected index, if any.
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_ix
    }

    /// Set the selected index, or `None` to clear selection.
    pub fn set_selected_index(&mut self, ix: Option<usize>, cx: &mut Context<Self>) {
        self.selected_ix = ix;
        self.selected_ids.clear();
        if let Some(ix) = ix
            && let Some(entry) = self.entries.get(ix)
        {
            self.selected_ids.insert(entry.item.id.clone());
            self.anchor_id = Some(entry.item.id.clone());
        } else {
            self.anchor_id = None;
        }
        cx.notify();
    }

    /// Set the selected index by tree item, or `None` to clear selection.
    pub fn set_selected_item(&mut self, item: Option<&TreeItem>, cx: &mut Context<Self>) {
        if let Some(item) = item {
            let ix = self
                .entries
                .iter()
                .position(|entry| entry.item.id == item.id);
            if ix.is_some() {
                self.selected_ix = ix;
            } else {
                self.expand_ancestors(item.id.clone(), cx);
                self.selected_ix = self
                    .entries
                    .iter()
                    .position(|entry| entry.item.id == item.id);
            }
            self.selected_ids.clear();
            self.selected_ids.insert(item.id.clone());
            self.anchor_id = Some(item.id.clone());
        } else {
            self.selected_ix = None;
            self.selected_ids.clear();
            self.anchor_id = None;
        }
        cx.notify();
    }

    /// Configure selection behavior.
    pub fn set_selection_mode(&mut self, mode: TreeSelectionMode, cx: &mut Context<Self>) {
        if self.selection_mode == mode {
            return;
        }
        self.selection_mode = mode;
        if mode == TreeSelectionMode::Single {
            self.keep_primary_selection_only();
        }
        self.reconcile_selection();
        cx.notify();
    }

    /// Return the configured selection behavior.
    pub fn selection_mode(&self) -> TreeSelectionMode {
        self.selection_mode
    }

    /// Return selected ids in visible tree order, followed by hidden selected ids.
    pub fn selected_ids(&self) -> Vec<SharedString> {
        let mut ids = Vec::with_capacity(self.selected_ids.len());
        for entry in &self.entries {
            if self.selected_ids.contains(&entry.item.id) {
                ids.push(entry.item.id.clone());
            }
        }
        for id in &self.selected_ids {
            if !ids.contains(id) {
                ids.push(id.clone());
            }
        }
        ids
    }

    /// Replace selection by ids. Hidden ids are retained and reconciled when entries rebuild.
    pub fn set_selected_ids(
        &mut self,
        ids: impl IntoIterator<Item = SharedString>,
        cx: &mut Context<Self>,
    ) {
        self.selected_ids = ids.into_iter().collect();
        if self.selection_mode == TreeSelectionMode::Single {
            self.keep_primary_selection_only();
        }
        self.anchor_id = self.selected_ids().first().cloned();
        self.reconcile_selection();
        cx.notify();
    }

    /// Select all currently visible entries. Useful for Ctrl+A handling in consumers.
    pub fn select_all_visible(&mut self, cx: &mut Context<Self>) {
        if self.selection_mode != TreeSelectionMode::Multi {
            return;
        }
        self.selected_ids = self
            .entries
            .iter()
            .map(|entry| entry.item.id.clone())
            .collect();
        self.anchor_id = self.entries.first().map(|entry| entry.item.id.clone());
        self.reconcile_selection();
        cx.notify();
    }

    /// Set expanded ids; the set is keyed by item id and survives item rebuilds.
    pub fn set_expanded(
        &mut self,
        ids: impl IntoIterator<Item = SharedString>,
        cx: &mut Context<Self>,
    ) {
        self.expanded_ids = ids.into_iter().collect();
        self.rebuild_entries();
        self.reconcile_selection();
        cx.notify();
    }

    /// Return expanded ids.
    pub fn expanded_ids(&self) -> Vec<SharedString> {
        self.expanded_ids.iter().cloned().collect()
    }

    /// Expand all current folders.
    pub fn expand_all(&mut self, cx: &mut Context<Self>) {
        let mut ids = HashSet::new();
        for item in &self.root_items {
            Self::collect_folder_ids(item, &mut ids);
        }
        self.expanded_ids.extend(ids);
        self.rebuild_entries();
        self.reconcile_selection();
        cx.notify();
    }

    /// Temporarily render all folders as expanded without mutating `expanded_ids`.
    pub fn set_force_expanded(&mut self, force: bool, cx: &mut Context<Self>) {
        if self.force_expanded == force {
            return;
        }
        self.force_expanded = force;
        self.rebuild_entries();
        self.reconcile_selection();
        cx.notify();
    }

    /// Whether all folders are temporarily rendered as expanded.
    pub fn force_expanded(&self) -> bool {
        self.force_expanded
    }

    /// Get the currently selected tree item, if any.
    pub fn selected_item(&self) -> Option<&TreeItem> {
        self.selected_ix
            .and_then(|ix| self.entries.get(ix).map(|entry| &entry.item))
    }

    pub fn scroll_to_item(&mut self, ix: usize, strategy: gpui::ScrollStrategy) {
        self.scroll_handle.scroll_to_item(ix, strategy);
    }

    /// Get the currently selected entry, if any.
    pub fn selected_entry(&self) -> Option<&TreeEntry> {
        self.selected_ix.and_then(|ix| self.entries.get(ix))
    }

    fn collect_item_expansion_flags(&mut self) {
        let mut ids = HashSet::new();
        for item in &self.root_items {
            Self::collect_expanded_item_ids(item, &mut ids);
        }
        self.expanded_ids.extend(ids);
    }

    fn collect_expanded_item_ids(item: &TreeItem, ids: &mut HashSet<SharedString>) {
        if item.is_expanded() {
            ids.insert(item.id.clone());
        }
        for child in &item.children {
            Self::collect_expanded_item_ids(child, ids);
        }
    }

    fn collect_folder_ids(item: &TreeItem, ids: &mut HashSet<SharedString>) {
        if item.is_folder() {
            ids.insert(item.id.clone());
        }
        for child in &item.children {
            Self::collect_folder_ids(child, ids);
        }
    }

    fn is_expanded_item(&self, item: &TreeItem) -> bool {
        self.force_expanded || self.expanded_ids.contains(&item.id)
    }

    fn index_for_id(&self, id: &SharedString) -> Option<usize> {
        self.entries.iter().position(|entry| entry.item.id == *id)
    }

    fn keep_primary_selection_only(&mut self) {
        let primary = self
            .selected_ix
            .and_then(|ix| self.entries.get(ix).map(|entry| entry.item.id.clone()))
            .or_else(|| self.selected_ids().first().cloned());
        self.selected_ids.clear();
        if let Some(id) = primary {
            self.selected_ids.insert(id.clone());
            self.anchor_id = Some(id);
        }
    }

    fn reconcile_selection(&mut self) {
        self.selected_ix = self
            .selected_ids()
            .first()
            .and_then(|id| self.index_for_id(id));
        self.right_clicked_ix = self.right_clicked_ix.filter(|ix| *ix < self.entries.len());
    }

    fn select_entry_with_modifiers(
        &mut self,
        ix: usize,
        modifiers: Modifiers,
        cx: &mut Context<Self>,
    ) {
        let Some(entry) = self.entries.get(ix) else {
            return;
        };
        if entry.item.is_disabled() {
            return;
        }
        let id = entry.item.id.clone();
        if self.selection_mode == TreeSelectionMode::Multi && modifiers.shift {
            let anchor_ix = self
                .anchor_id
                .as_ref()
                .and_then(|anchor| self.index_for_id(anchor))
                .unwrap_or(ix);
            let (start, end) = if anchor_ix <= ix {
                (anchor_ix, ix)
            } else {
                (ix, anchor_ix)
            };
            self.selected_ids.clear();
            for entry in &self.entries[start..=end] {
                if !entry.item.is_disabled() {
                    self.selected_ids.insert(entry.item.id.clone());
                }
            }
        } else if self.selection_mode == TreeSelectionMode::Multi && modifiers.secondary() {
            if !self.selected_ids.remove(&id) {
                self.selected_ids.insert(id.clone());
            }
            self.anchor_id = Some(id.clone());
        } else {
            self.selected_ids.clear();
            self.selected_ids.insert(id.clone());
            self.anchor_id = Some(id.clone());
        }
        self.selected_ix = Some(ix);
        cx.notify();
    }

    fn expand_ancestors(&mut self, target_id: SharedString, cx: &mut Context<Self>) {
        let mut ancestors = Vec::new();

        for item in &self.root_items {
            if let Some(found_ancestors) = item.find_ancestors(&target_id) {
                ancestors = found_ancestors;
                break;
            }
        }

        if ancestors.is_empty() {
            return;
        }

        for ancestor in ancestors.into_iter().rev() {
            if !self.expanded_ids.contains(&ancestor.id) {
                self.expanded_ids.insert(ancestor.id.clone());
                cx.emit(TreeEvent::Expanded(ancestor.id.clone()));
            }
        }

        self.rebuild_entries();
    }

    fn add_entry(&mut self, item: TreeItem, depth: usize) {
        self.entries.push(TreeEntry {
            item: item.clone(),
            depth,
            expanded: self.is_expanded_item(&item),
        });
        if self.is_expanded_item(&item) {
            for child in &item.children {
                self.add_entry(child.clone(), depth + 1);
            }
        }
    }

    fn toggle_expand(&mut self, ix: usize, cx: &mut Context<Self>) {
        let Some(entry) = self.entries.get(ix) else {
            return;
        };
        if !entry.is_folder() {
            return;
        }

        let expanded = !self.expanded_ids.contains(&entry.item.id);
        let id = entry.item.id.clone();

        if expanded {
            self.expanded_ids.insert(id.clone());
            cx.emit(TreeEvent::Expanded(id));
        } else {
            self.expanded_ids.remove(&id);
            cx.emit(TreeEvent::Collapsed(id));
        }

        self.right_clicked_ix = None;
        self.rebuild_entries();
    }

    fn rebuild_entries(&mut self) {
        self.entries.clear();
        for item in self.root_items.clone().into_iter() {
            self.add_entry(item, 0);
        }
    }

    pub fn focus(&mut self, window: &mut Window, cx: &mut App) {
        self.focus_handle.focus(window, cx);
    }

    fn on_action_confirm(&mut self, _: &Confirm, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(selected_ix) = self.selected_ix {
            if let Some(entry) = self.entries.get(selected_ix) {
                cx.emit(TreeEvent::Activated(entry.item.id.clone()));
                if entry.is_folder() {
                    cx.emit(TreeEvent::ChevronClicked {
                        id: entry.item.id.clone(),
                    });
                    self.toggle_expand(selected_ix, cx);
                    cx.notify();
                }
            }
        }
    }

    fn on_action_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(selected_ix) = self.selected_ix {
            if let Some(entry) = self.entries.get(selected_ix) {
                if entry.is_folder() && entry.is_expanded() {
                    self.toggle_expand(selected_ix, cx);
                    cx.notify();
                }
            }
        }
    }

    fn on_action_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(selected_ix) = self.selected_ix {
            if let Some(entry) = self.entries.get(selected_ix) {
                if entry.is_folder() && !entry.is_expanded() {
                    self.toggle_expand(selected_ix, cx);
                    cx.notify();
                }
            }
        }
    }

    fn on_action_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        let mut selected_ix = self.selected_ix.unwrap_or(0);

        if selected_ix > 0 {
            selected_ix = selected_ix - 1;
        } else {
            selected_ix = self.entries.len().saturating_sub(1);
        }

        let id = self
            .entries
            .get(selected_ix)
            .map(|entry| entry.item.id.clone());
        self.selected_ix = Some(selected_ix);
        self.selected_ids.clear();
        if let Some(id) = id {
            self.selected_ids.insert(id.clone());
            self.anchor_id = Some(id);
        }
        self.scroll_handle
            .scroll_to_item(selected_ix, gpui::ScrollStrategy::Top);
        cx.notify();
    }

    fn on_action_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        let mut selected_ix = self.selected_ix.unwrap_or(0);
        if selected_ix + 1 < self.entries.len() {
            selected_ix = selected_ix + 1;
        } else {
            selected_ix = 0;
        }

        let id = self
            .entries
            .get(selected_ix)
            .map(|entry| entry.item.id.clone());
        self.selected_ix = Some(selected_ix);
        self.selected_ids.clear();
        if let Some(id) = id {
            self.selected_ids.insert(id.clone());
            self.anchor_id = Some(id);
        }
        self.scroll_handle
            .scroll_to_item(selected_ix, gpui::ScrollStrategy::Bottom);
        cx.notify();
    }

    fn on_entry_click(
        &mut self,
        ix: usize,
        event: &MouseDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(entry) = self.entries.get(ix) else {
            return;
        };
        let id = entry.item.id.clone();
        let is_folder = entry.is_folder();
        cx.emit(TreeEvent::RowClicked {
            id: id.clone(),
            modifiers: event.modifiers,
            button: event.button,
        });
        self.select_entry_with_modifiers(ix, event.modifiers, cx);
        if event.button == MouseButton::Left && is_folder {
            cx.emit(TreeEvent::ChevronClicked { id });
            self.toggle_expand(ix, cx);
        }
        cx.notify();
    }
}

impl Render for TreeState {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let render_row = self.render_row.clone();
        let state = cx.entity().clone();
        let custom_rows = self.custom_rows;
        let row_decorators = self.row_decorators.clone();

        div()
            .id("tree-state")
            .size_full()
            .relative()
            .context_menu({
                let state = state.clone();
                move |menu, window, cx: &mut Context<PopupMenu>| {
                    if state.read(cx).context_menu_builder.is_none() {
                        return menu;
                    }

                    let (ix, entry) = {
                        let state = state.read(cx);
                        let entry = state
                            .right_clicked_ix
                            .and_then(|ix| state.entries.get(ix).cloned());
                        (state.right_clicked_ix, entry)
                    };

                    if let (Some(ix), Some(entry)) = (ix, entry) {
                        state.update(cx, |state, cx| {
                            if let Some(build) = state.context_menu_builder.clone() {
                                build(ix, &entry, menu, window, cx)
                            } else {
                                menu
                            }
                        })
                    } else {
                        menu
                    }
                }
            })
            .child(
                uniform_list("entries", self.entries.len(), {
                    cx.processor(move |state, visible_range: Range<usize>, window, cx| {
                        let mut items = Vec::with_capacity(visible_range.len());
                        for ix in visible_range {
                            let entry = &state.entries[ix];
                            let selected = state.selected_ids.contains(&entry.item.id);
                            let right_clicked = Some(ix) == state.right_clicked_ix;
                            let item = (render_row)(
                                entry,
                                TreeRowMeta {
                                    index: ix,
                                    selected,
                                    right_clicked,
                                },
                                window,
                                cx,
                            );

                            let mut el = div().id(ix).child(item).when(
                                !custom_rows && !entry.item().is_disabled(),
                                |this| {
                                    this.on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener({
                                            move |this, event: &MouseDownEvent, window, cx| {
                                                this.on_entry_click(ix, event, window, cx);
                                            }
                                        }),
                                    )
                                    .on_mouse_down(
                                        MouseButton::Right,
                                        cx.listener(
                                            move |this, event: &MouseDownEvent, window, cx| {
                                                this.right_clicked_ix = Some(ix);
                                                let Some(entry) = this.entries.get(ix) else {
                                                    return;
                                                };
                                                let id = entry.item.id.clone();
                                                cx.emit(TreeEvent::RowClicked {
                                                    id: id.clone(),
                                                    modifiers: event.modifiers,
                                                    button: event.button,
                                                });
                                                if !this.selected_ids.contains(&id) {
                                                    this.selected_ids.clear();
                                                    this.selected_ids.insert(id.clone());
                                                    this.selected_ix = Some(ix);
                                                    this.anchor_id = Some(id);
                                                }
                                                let _ = window;
                                                cx.notify();
                                            },
                                        ),
                                    )
                                },
                            );
                            for decorator in &row_decorators {
                                el = decorator(
                                    el,
                                    entry,
                                    TreeRowMeta {
                                        index: ix,
                                        selected,
                                        right_clicked,
                                    },
                                    window,
                                    cx,
                                );
                            }

                            items.push(el)
                        }

                        items
                    })
                })
                .flex_grow_1()
                .size_full()
                .track_scroll(&self.scroll_handle)
                .with_sizing_behavior(ListSizingBehavior::Auto)
                .into_any_element(),
            )
    }
}

/// A tree view element that displays hierarchical data.
#[derive(IntoElement)]
pub struct Tree {
    id: ElementId,
    state: Entity<TreeState>,
    style: StyleRefinement,
    render_row: RowRenderer,
    row_decorators: Vec<RowDecorator>,
    custom_rows: bool,
    selection_mode: Option<TreeSelectionMode>,
    context_menu_builder: Option<ContextMenuBuilder>,
}

impl Tree {
    pub fn new<R>(state: &Entity<TreeState>, render_item: R) -> Self
    where
        R: Fn(usize, &TreeEntry, bool, &mut Window, &mut App) -> ListItem + 'static,
    {
        Self {
            id: ElementId::Name(format!("tree-{}", state.entity_id()).into()),
            state: state.clone(),
            style: StyleRefinement::default(),
            render_row: Rc::new(move |entry, meta, window, app| {
                render_item(meta.index, entry, meta.selected, window, app)
                    .disabled(entry.item().is_disabled())
                    .selected(meta.selected)
                    .secondary_selected(meta.right_clicked)
                    .into_any_element()
            }),
            row_decorators: Vec::new(),
            custom_rows: false,
            selection_mode: None,
            context_menu_builder: None,
        }
    }

    /// Create a headless/controlled tree: the renderer returns the full row
    /// element and the tree does not install row click/expand handlers.
    ///
    /// Consumers keep their own row interaction logic while reusing TreeState's
    /// item flattening, virtualization, scroll handle, focus and keyboard action
    /// wiring.
    pub fn custom<R, E>(state: &Entity<TreeState>, render_row: R) -> Self
    where
        R: Fn(&TreeEntry, TreeRowMeta, &mut Window, &mut App) -> E + 'static,
        E: IntoElement,
    {
        Self {
            id: ElementId::Name(format!("tree-{}", state.entity_id()).into()),
            state: state.clone(),
            style: StyleRefinement::default(),
            render_row: Rc::new(move |entry, meta, window, app| {
                render_row(entry, meta, window, app).into_any_element()
            }),
            row_decorators: Vec::new(),
            custom_rows: true,
            selection_mode: None,
            context_menu_builder: None,
        }
    }

    /// Configure built-in selection behavior.
    pub fn selection_mode(mut self, mode: TreeSelectionMode) -> Self {
        self.selection_mode = Some(mode);
        self
    }

    /// Decorate the virtualized row container.
    ///
    /// This is the low-level hook for behaviors that must live on the real row
    /// hitbox, such as typed drag-and-drop.
    pub fn row_decorator<F>(mut self, decorator: F) -> Self
    where
        F: Fn(Stateful<Div>, &TreeEntry, TreeRowMeta, &mut Window, &mut App) -> Stateful<Div>
            + 'static,
    {
        self.row_decorators.push(Rc::new(decorator));
        self
    }

    /// Make rows draggable with a typed GPUI payload.
    pub fn draggable<T, W, V, P>(self, value: V, preview: P) -> Self
    where
        T: 'static,
        W: Render + 'static,
        V: Fn(&TreeEntry, &TreeRowMeta) -> Option<T> + 'static,
        P: Fn(&T, Point<Pixels>, &mut Window, &mut App) -> Entity<W> + 'static,
    {
        let value = Rc::new(value);
        let preview = Rc::new(preview);
        self.row_decorator(move |row, entry, meta, _window, _app| {
            let Some(payload) = value(entry, &meta) else {
                return row;
            };
            let preview = preview.clone();
            row.on_drag(payload, move |drag, pos, window, app| {
                preview(drag, pos, window, app)
            })
        })
    }

    /// Apply a style while a typed payload is dragged over a row.
    pub fn drag_over<T, F>(self, style: F) -> Self
    where
        T: 'static,
        F: Fn(
                StyleRefinement,
                &TreeEntry,
                &TreeRowMeta,
                &T,
                &mut Window,
                &mut App,
            ) -> StyleRefinement
            + 'static,
    {
        let style = Rc::new(style);
        self.row_decorator(move |row, entry, meta, _window, _app| {
            let entry = entry.clone();
            row.drag_over::<T>({
                let style = style.clone();
                move |base, drag, window, app| style(base, &entry, &meta, drag, window, app)
            })
        })
    }

    /// Install a typed drop target on every row.
    pub fn drop_target<T, C, D>(self, can_drop: C, on_drop: D) -> Self
    where
        T: 'static,
        C: Fn(&TreeEntry, &TreeRowMeta, &T, &mut Window, &mut App) -> bool + 'static,
        D: Fn(&TreeEntry, &TreeRowMeta, &T, &mut Window, &mut App) + 'static,
    {
        let can_drop = Rc::new(can_drop);
        let on_drop = Rc::new(on_drop);
        self.row_decorator(move |row, entry, meta, _window, _app| {
            let drop_entry = entry.clone();
            let drop_meta = meta.clone();
            let can_entry = entry.clone();
            let can_meta = meta.clone();
            row.can_drop({
                let can_drop = can_drop.clone();
                move |drag, window, app| {
                    drag.downcast_ref::<T>()
                        .map(|drag| can_drop(&can_entry, &can_meta, drag, window, app))
                        .unwrap_or(false)
                }
            })
            .on_drop({
                let on_drop = on_drop.clone();
                move |drag: &T, window, app| on_drop(&drop_entry, &drop_meta, drag, window, app)
            })
        })
    }

    /// Add a context menu to the tree.
    ///
    /// The closure receives:
    /// - `ix`: the index of the right-clicked entry
    /// - `entry`: the right-clicked tree entry
    /// - `menu`: the popup menu builder
    pub fn context_menu<F>(mut self, f: F) -> Self
    where
        F: Fn(usize, &TreeEntry, PopupMenu, &mut Window, &mut Context<TreeState>) -> PopupMenu
            + 'static,
    {
        self.context_menu_builder = Some(Rc::new(f));
        self
    }
}

impl Styled for Tree {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Tree {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focus_handle = self.state.read(cx).focus_handle.clone();
        let scroll_handle = self.state.read(cx).scroll_handle.clone();

        self.state.update(cx, |state, _| {
            state.render_row = self.render_row;
            state.row_decorators = self.row_decorators;
            state.custom_rows = self.custom_rows;
            if let Some(mode) = self.selection_mode {
                state.selection_mode = mode;
                if mode == TreeSelectionMode::Single {
                    state.keep_primary_selection_only();
                }
                state.reconcile_selection();
            }
            state.context_menu_builder = self.context_menu_builder;
        });

        div()
            .id(self.id)
            .key_context(CONTEXT)
            .track_focus(&focus_handle)
            .on_action(window.listener_for(&self.state, TreeState::on_action_confirm))
            .on_action(window.listener_for(&self.state, TreeState::on_action_left))
            .on_action(window.listener_for(&self.state, TreeState::on_action_right))
            .on_action(window.listener_for(&self.state, TreeState::on_action_up))
            .on_action(window.listener_for(&self.state, TreeState::on_action_down))
            .size_full()
            .child(self.state)
            .refine_style(&self.style)
            .vertical_scrollbar(&scroll_handle)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use indoc::indoc;

    use super::{TreeEvent, TreeState};
    use gpui::{AppContext as _, ParentElement as _, Render, SharedString, Subscription};

    struct TestCollector {
        _state: gpui::Entity<TreeState>,
        events: Rc<RefCell<Vec<TreeEvent>>>,
        _subscription: Subscription,
    }

    impl TestCollector {
        fn new(state: &gpui::Entity<TreeState>, cx: &mut gpui::Context<Self>) -> Self {
            let events = Rc::new(RefCell::new(Vec::new()));
            let events_clone = events.clone();
            let _subscription = cx.subscribe(state, move |_, _, ev: &TreeEvent, _| {
                events_clone.borrow_mut().push(ev.clone());
            });
            Self {
                _state: state.clone(),
                events,
                _subscription,
            }
        }
    }

    impl Render for TestCollector {
        fn render(
            &mut self,
            _: &mut gpui::Window,
            _: &mut gpui::Context<Self>,
        ) -> impl gpui::IntoElement {
            gpui::div()
        }
    }

    fn assert_entries(entries: &Vec<super::TreeEntry>, expected: &str) {
        let actual: Vec<String> = entries
            .iter()
            .map(|e| {
                let mut s = String::new();
                s.push_str(&"    ".repeat(e.depth));
                s.push_str(e.item().label.as_str());
                s
            })
            .collect();
        let actual = actual.join("\n");
        assert_eq!(actual.trim(), expected.trim());
    }

    #[gpui::test]
    fn test_tree_entry(cx: &mut gpui::TestAppContext) {
        use super::TreeItem;

        let items = vec![
            TreeItem::new("src", "src")
                .expanded(true)
                .child(
                    TreeItem::new("src/ui", "ui")
                        .expanded(true)
                        .child(TreeItem::new("src/ui/button.rs", "button.rs"))
                        .child(TreeItem::new("src/ui/icon.rs", "icon.rs"))
                        .child(TreeItem::new("src/ui/mod.rs", "mod.rs")),
                )
                .child(TreeItem::new("src/lib.rs", "lib.rs")),
            TreeItem::new("Cargo.toml", "Cargo.toml"),
            TreeItem::new("Cargo.lock", "Cargo.lock").disabled(true),
            TreeItem::new("README.md", "README.md"),
        ];

        let state = cx.new(|cx| TreeState::new(cx).items(items));
        state.update(cx, |state, cx| {
            assert_entries(
                &state.entries,
                indoc! {
                    r#"
                src
                    ui
                        button.rs
                        icon.rs
                        mod.rs
                    lib.rs
                Cargo.toml
                Cargo.lock
                README.md
                "#
                },
            );

            let entry = state.entries.get(0).unwrap();
            assert_eq!(entry.depth(), 0);
            assert_eq!(entry.is_root(), true);
            assert_eq!(entry.is_folder(), true);
            assert_eq!(entry.is_expanded(), true);

            let entry = state.entries.get(1).unwrap();
            assert_eq!(entry.depth(), 1);
            assert_eq!(entry.is_root(), false);
            assert_eq!(entry.is_folder(), true);
            assert_eq!(entry.is_expanded(), true);
            assert_eq!(entry.item().label.as_str(), "ui");

            state.toggle_expand(1, cx);
            let entry = state.entries.get(1).unwrap();
            assert_eq!(entry.is_expanded(), false);
            assert_entries(
                &state.entries,
                indoc! {
                    r#"
                src
                    ui
                    lib.rs
                Cargo.toml
                Cargo.lock
                README.md
                "#
                },
            );
        })
    }

    #[gpui::test]
    fn test_emits_expanded_event(cx: &mut gpui::TestAppContext) {
        let items = vec![
            super::TreeItem::new("src", "src").child(super::TreeItem::new("src/lib.rs", "lib.rs")),
        ];
        let state = cx.new(|cx| TreeState::new(cx).items(items));
        let collector = cx.new(|cx| TestCollector::new(&state, cx));

        state.update(cx, |state, cx| {
            state.toggle_expand(0, cx);
        });

        let events = collector.read_with(cx, |c, _| c.events.borrow().clone());
        assert_eq!(events, vec![TreeEvent::Expanded("src".into())]);
    }

    #[gpui::test]
    fn test_emits_collapsed_event(cx: &mut gpui::TestAppContext) {
        let items = vec![
            super::TreeItem::new("src", "src")
                .expanded(true)
                .child(super::TreeItem::new("src/lib.rs", "lib.rs")),
        ];
        let state = cx.new(|cx| TreeState::new(cx).items(items));
        let collector = cx.new(|cx| TestCollector::new(&state, cx));

        state.update(cx, |state, cx| {
            state.toggle_expand(0, cx);
        });

        let events = collector.read_with(cx, |c, _| c.events.borrow().clone());
        assert_eq!(events, vec![TreeEvent::Collapsed("src".into())]);
    }

    #[gpui::test]
    fn test_set_items_does_not_emit_expansion_events(cx: &mut gpui::TestAppContext) {
        let items = vec![
            super::TreeItem::new("src", "src")
                .expanded(true)
                .child(super::TreeItem::new("src/lib.rs", "lib.rs")),
        ];
        let state = cx.new(|cx| TreeState::new(cx).items(items));
        let collector = cx.new(|cx| TestCollector::new(&state, cx));

        let new_items = vec![
            super::TreeItem::new("docs", "docs")
                .expanded(true)
                .child(super::TreeItem::new("docs/readme.md", "readme.md")),
        ];
        state.update(cx, |state, cx| {
            state.set_items(new_items, cx);
        });

        let events = collector.read_with(cx, |c, _| c.events.borrow().clone());
        assert!(
            events.is_empty(),
            "set_items should not emit Expanded/Collapsed events"
        );
    }

    #[gpui::test]
    fn test_event_carries_item_id(cx: &mut gpui::TestAppContext) {
        let items = vec![
            super::TreeItem::new("src", "src").expanded(true).child(
                super::TreeItem::new("src/ui", "ui")
                    .child(super::TreeItem::new("src/ui/button.rs", "button.rs")),
            ),
        ];
        let state = cx.new(|cx| TreeState::new(cx).items(items));
        let collector = cx.new(|cx| TestCollector::new(&state, cx));

        // Toggle the child at index 1 ("src/ui"), event payload should be the id not the index.
        state.update(cx, |state, cx| {
            state.toggle_expand(1, cx);
        });

        let events = collector.read_with(cx, |c, _| c.events.borrow().clone());
        assert_eq!(events, vec![TreeEvent::Expanded("src/ui".into())]);
    }

    #[gpui::test]
    fn test_set_selected_item_emits_expanded_events_for_hidden_ancestors(
        cx: &mut gpui::TestAppContext,
    ) {
        let target = super::TreeItem::new("src/ui/button.rs", "button.rs");
        let items = vec![
            super::TreeItem::new("src", "src")
                .child(super::TreeItem::new("src/ui", "ui").child(target.clone())),
        ];
        let state = cx.new(|cx| TreeState::new(cx).items(items));
        let collector = cx.new(|cx| TestCollector::new(&state, cx));

        state.update(cx, |state, cx| {
            state.set_selected_item(Some(&target), cx);
        });

        let events = collector.read_with(cx, |c, _| c.events.borrow().clone());
        assert_eq!(
            events,
            vec![
                TreeEvent::Expanded("src".into()),
                TreeEvent::Expanded("src/ui".into())
            ]
        );
    }

    #[gpui::test]
    fn empty_folder_is_visible_and_expandable_by_explicit_flag(cx: &mut gpui::TestAppContext) {
        let items = vec![super::TreeItem::new("empty", "empty").folder(true)];
        let state = cx.new(|cx| TreeState::new(cx).items(items));

        state.update(cx, |state, cx| {
            assert_eq!(state.entries.len(), 1);
            assert!(state.entries[0].is_folder());
            state.toggle_expand(0, cx);
            assert!(state.expanded_ids().contains(&SharedString::from("empty")));
        });
    }

    #[gpui::test]
    fn selected_ids_survive_rebuild_by_id(cx: &mut gpui::TestAppContext) {
        let first = vec![
            super::TreeItem::new("root", "root")
                .expanded(true)
                .child(super::TreeItem::new("a", "a"))
                .child(super::TreeItem::new("b", "b")),
        ];
        let state = cx.new(|cx| TreeState::new(cx).items(first));

        state.update(cx, |state, cx| {
            state.set_selected_ids([SharedString::from("b")], cx);
            assert_eq!(state.selected_index(), Some(2));
            let rebuilt = vec![
                super::TreeItem::new("root", "root")
                    .expanded(true)
                    .child(super::TreeItem::new("x", "x"))
                    .child(super::TreeItem::new("b", "b")),
            ];
            state.set_items(rebuilt, cx);
            assert_eq!(state.selected_ids(), vec![SharedString::from("b")]);
            assert_eq!(state.selected_index(), Some(2));
        });
    }

    #[gpui::test]
    fn force_expanded_does_not_mutate_expanded_ids(cx: &mut gpui::TestAppContext) {
        let items = vec![
            super::TreeItem::new("root", "root")
                .child(super::TreeItem::new("root/a", "a"))
                .child(super::TreeItem::new("root/b", "b")),
        ];
        let state = cx.new(|cx| TreeState::new(cx).items(items));

        state.update(cx, |state, cx| {
            assert_eq!(state.entries.len(), 1);
            assert!(state.expanded_ids().is_empty());
            state.set_force_expanded(true, cx);
            assert_eq!(state.entries.len(), 3);
            assert!(state.expanded_ids().is_empty());
            state.set_force_expanded(false, cx);
            assert_eq!(state.entries.len(), 1);
        });
    }

    #[gpui::test]
    fn multi_selection_supports_shift_range_and_secondary_toggle(cx: &mut gpui::TestAppContext) {
        let items = vec![
            super::TreeItem::new("a", "a"),
            super::TreeItem::new("b", "b"),
            super::TreeItem::new("c", "c"),
            super::TreeItem::new("d", "d"),
        ];
        let state = cx.new(|cx| {
            let mut state = TreeState::new(cx).items(items);
            state.selection_mode = super::TreeSelectionMode::Multi;
            state
        });

        state.update(cx, |state, cx| {
            state.select_entry_with_modifiers(1, gpui::Modifiers::none(), cx);
            let mut shift = gpui::Modifiers::none();
            shift.shift = true;
            state.select_entry_with_modifiers(3, shift, cx);
            assert_eq!(
                state.selected_ids(),
                vec![
                    SharedString::from("b"),
                    SharedString::from("c"),
                    SharedString::from("d")
                ]
            );

            let mut secondary = gpui::Modifiers::none();
            secondary.control = true;
            state.select_entry_with_modifiers(2, secondary, cx);
            assert_eq!(
                state.selected_ids(),
                vec![SharedString::from("b"), SharedString::from("d")]
            );
        });
    }

    #[gpui::test]
    fn tree_typed_dnd_builders_are_composable(cx: &mut gpui::TestAppContext) {
        #[derive(Clone)]
        struct DragPayload {
            id: SharedString,
        }

        struct DragPreview;
        impl Render for DragPreview {
            fn render(
                &mut self,
                _: &mut gpui::Window,
                _: &mut gpui::Context<Self>,
            ) -> impl gpui::IntoElement {
                gpui::div()
            }
        }

        let state =
            cx.new(|cx| TreeState::new(cx).items([super::TreeItem::new("a", "a").folder(true)]));
        let _tree = super::Tree::custom(&state, |entry, _meta, _window, _cx| {
            gpui::div().child(entry.item().label().clone())
        })
        .draggable(
            |entry, _meta| {
                Some(DragPayload {
                    id: entry.item().id().clone(),
                })
            },
            |_drag, _pos, _window, cx| cx.new(|_| DragPreview),
        )
        .drag_over::<DragPayload, _>(|style, entry, _meta, drag, _window, _cx| {
            assert_eq!(entry.item().id(), &drag.id);
            style
        })
        .drop_target::<DragPayload, _, _>(
            |entry, _meta, drag, _window, _cx| entry.item().id() == &drag.id,
            |_entry, _meta, _drag, _window, _cx| {},
        );
    }
}
