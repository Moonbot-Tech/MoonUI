use gpui::*;
use crate::searchable_list::{SearchableListItem, SearchableVec};
use crate::select::{
    Select as CoreSelect, SelectEvent as CoreSelectEvent, SelectState as CoreSelectState,
};
use crate::{Sizable as _, Size};

use super::{
    button::{MoonButtonSize, MoonButtonVariant},
    dropdown::MoonMenuSize,
    index_path::IndexPath,
    tokens::MoonRect,
};

type MoonSelectDelegate<T> = SearchableVec<MoonSelectItem<T>>;
type MoonCoreSelectState<T> = CoreSelectState<MoonSelectDelegate<T>>;

pub(crate) fn bind_moon_select_keys(_cx: &mut App) {}

#[derive(Clone, Debug)]
pub struct MoonSelectItem<T> {
    value: T,
    label: SharedString,
    disabled: bool,
}

impl<T> MoonSelectItem<T> {
    pub fn new(value: T, label: impl Into<SharedString>) -> Self {
        Self {
            value,
            label: label.into(),
            disabled: false,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl<T> SearchableListItem for MoonSelectItem<T>
where
    T: Clone + PartialEq + 'static,
{
    type Value = T;

    fn title(&self) -> SharedString {
        self.label.clone()
    }

    fn value(&self) -> &Self::Value {
        &self.value
    }

    fn matches(&self, query: &str) -> bool {
        self.label
            .as_ref()
            .to_lowercase()
            .contains(&query.to_lowercase())
    }

    fn disabled(&self) -> bool {
        self.disabled
    }
}

pub enum MoonSelectEvent<T> {
    Confirm(Option<T>),
}

pub struct MoonSelectState<T>
where
    T: Clone + PartialEq + 'static,
{
    core: Entity<MoonCoreSelectState<T>>,
    items: Vec<MoonSelectItem<T>>,
    selected_index: Option<IndexPath>,
    active_index: Option<IndexPath>,
    open: bool,
    dirty_items: bool,
    dirty_selection: bool,
    dirty_open: bool,
}

impl<T> EventEmitter<MoonSelectEvent<T>> for MoonSelectState<T> where T: Clone + PartialEq + 'static {}

impl<T> Focusable for MoonSelectState<T>
where
    T: Clone + PartialEq + 'static,
{
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.core.focus_handle(cx)
    }
}

impl<T> MoonSelectState<T>
where
    T: Clone + PartialEq + 'static,
{
    pub fn new(
        items: impl IntoIterator<Item = MoonSelectItem<T>>,
        selected_index: Option<IndexPath>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let items: Vec<_> = items.into_iter().collect();
        let core_selected = selected_index.map(to_core_index);
        let delegate = SearchableVec::new(items.clone());
        let core = cx.new(|cx| MoonCoreSelectState::new(delegate, core_selected, window, cx));

        cx.subscribe(
            &core,
            |this, _, event: &CoreSelectEvent<MoonSelectDelegate<T>>, cx| {
                let CoreSelectEvent::Confirm(value) = event;
                this.set_selected_value_mirror(value.clone());
                cx.emit(MoonSelectEvent::Confirm(value.clone()));
                cx.notify();
            },
        )
        .detach();

        Self {
            core,
            items,
            selected_index,
            active_index: selected_index,
            open: false,
            dirty_items: false,
            dirty_selection: false,
            dirty_open: false,
        }
    }

    pub fn items(&self) -> &[MoonSelectItem<T>] {
        &self.items
    }

    pub fn set_items(
        &mut self,
        items: impl IntoIterator<Item = MoonSelectItem<T>>,
        cx: &mut Context<Self>,
    ) {
        let selected_value = self.selected_value().cloned();
        self.items = items.into_iter().collect();
        self.selected_index = selected_value.and_then(|value| self.index_for_value(&value));
        self.active_index = self.selected_index;
        self.dirty_items = true;
        self.dirty_selection = true;
        cx.notify();
    }

    pub fn selected_index(&self) -> Option<IndexPath> {
        self.selected_index
    }

    pub fn active_index(&self) -> Option<IndexPath> {
        self.active_index
    }

    pub fn selected_value(&self) -> Option<&T> {
        self.selected_index
            .and_then(|index| self.items.get(index.row).map(|item| &item.value))
    }

    pub fn selected_label(&self) -> Option<SharedString> {
        self.selected_index
            .and_then(|index| self.items.get(index.row).map(|item| item.label.clone()))
    }

    pub fn set_selected_index(&mut self, index: Option<IndexPath>) {
        self.selected_index = index;
        self.active_index = index;
        self.dirty_selection = true;
    }

    pub fn set_selected_value(&mut self, value: &T) -> bool {
        let Some(index) = self.index_for_value(value) else {
            return false;
        };
        self.selected_index = Some(index);
        self.active_index = Some(index);
        self.dirty_selection = true;
        true
    }

    pub fn clear_selection(&mut self) {
        self.selected_index = None;
        self.active_index = None;
        self.dirty_selection = true;
    }

    pub fn set_open(&mut self, open: bool) {
        self.open = open;
        self.dirty_open = true;
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.core.focus_handle(cx).focus(window, cx);
    }

    fn index_for_value(&self, value: &T) -> Option<IndexPath> {
        self.items
            .iter()
            .position(|item| &item.value == value)
            .map(IndexPath::new)
    }

    fn set_selected_value_mirror(&mut self, value: Option<T>) {
        self.selected_index = value.and_then(|value| self.index_for_value(&value));
        self.active_index = self.selected_index;
        self.dirty_selection = false;
    }

    fn sync_core(&mut self, searchable: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.dirty_items {
            let items = SearchableVec::new(self.items.clone());
            self.core.update(cx, |core, cx| {
                core.replace_items(items, cx);
            });
            self.dirty_items = false;
        }

        if self.dirty_selection {
            let selected = self.selected_index.map(to_core_index);
            self.core.update(cx, |core, cx| {
                core.set_selected_index(selected, window, cx);
            });
            self.dirty_selection = false;
        }

        if self.dirty_open {
            let open = self.open;
            self.core.update(cx, |core, cx| {
                core.set_open_for_moon(open, cx);
            });
            self.dirty_open = false;
        }

        self.core.update(cx, |core, cx| {
            core.set_searchable(searchable, cx);
        });
    }
}

#[derive(IntoElement)]
pub struct MoonSelect<T>
where
    T: Clone + PartialEq + 'static,
{
    id: SharedString,
    bounds: Option<MoonRect>,
    state: Entity<MoonSelectState<T>>,
    placeholder: SharedString,
    title_prefix: Option<SharedString>,
    disabled: bool,
    cleanable: bool,
    searchable: bool,
    search_placeholder: SharedString,
    appearance: bool,
    trigger_variant: MoonButtonVariant,
    trigger_size: MoonButtonSize,
    menu_width: f32,
    menu_max_height: Option<f32>,
    menu_size: MoonMenuSize,
}

impl<T> MoonSelect<T>
where
    T: Clone + PartialEq + 'static,
{
    pub fn new(state: &Entity<MoonSelectState<T>>) -> Self {
        Self {
            id: SharedString::from(format!("moon-select:{}", state.entity_id())),
            bounds: None,
            state: state.clone(),
            placeholder: SharedString::from("Select"),
            title_prefix: None,
            disabled: false,
            cleanable: false,
            searchable: false,
            search_placeholder: SharedString::from("Search..."),
            appearance: true,
            trigger_variant: MoonButtonVariant::Neutral,
            trigger_size: MoonButtonSize::Toolbar,
            menu_width: 180.0,
            menu_max_height: None,
            menu_size: MoonMenuSize::Normal,
        }
    }

    pub fn id(mut self, id: impl Into<SharedString>) -> Self {
        self.id = id.into();
        self
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn title_prefix(mut self, prefix: impl Into<SharedString>) -> Self {
        self.title_prefix = Some(prefix.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn cleanable(mut self, cleanable: bool) -> Self {
        self.cleanable = cleanable;
        self
    }

    pub fn searchable(mut self, searchable: bool) -> Self {
        self.searchable = searchable;
        self
    }

    pub fn search_placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.search_placeholder = placeholder.into();
        self
    }

    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }

    pub fn trigger_variant(mut self, variant: MoonButtonVariant) -> Self {
        self.trigger_variant = variant;
        self
    }

    pub fn trigger_size(mut self, size: MoonButtonSize) -> Self {
        self.trigger_size = size;
        self
    }

    pub fn menu_width(mut self, width: f32) -> Self {
        self.menu_width = width;
        self
    }

    pub fn menu_max_height(mut self, height: f32) -> Self {
        self.menu_max_height = Some(height);
        self
    }

    pub fn menu_size(mut self, size: MoonMenuSize) -> Self {
        self.menu_size = size;
        self
    }
}

impl<T> RenderOnce for MoonSelect<T>
where
    T: Clone + PartialEq + 'static,
{
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        self.state.update(cx, |state, cx| {
            state.sync_core(self.searchable, window, cx);
        });

        let core = self.state.read(cx).core.clone();
        let mut select = CoreSelect::new(&core)
            .placeholder(self.placeholder)
            .cleanable(self.cleanable)
            .search_placeholder(self.search_placeholder)
            .disabled(self.disabled)
            .appearance(self.appearance)
            .menu_width(px(self.menu_width))
            .with_size(size_for(self.trigger_size, self.menu_size));

        if let Some(prefix) = self.title_prefix {
            select = select.title_prefix(prefix);
        }
        if let Some(max_height) = self.menu_max_height {
            select = select.menu_max_h(px(max_height));
        }

        let mut root = div()
            .id(ElementId::from(self.id.clone()))
            .relative()
            .child(select);

        if let Some(bounds) = self.bounds {
            root = root
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        }

        let _variant = self.trigger_variant;
        root
    }
}

fn to_core_index(index: IndexPath) -> crate::IndexPath {
    crate::IndexPath::new(index.row)
        .section(index.section)
        .column(index.column)
}

fn size_for(trigger: MoonButtonSize, _menu: MoonMenuSize) -> Size {
    match trigger {
        MoonButtonSize::Micro => Size::XSmall,
        MoonButtonSize::Action => Size::Small,
        MoonButtonSize::Toolbar => Size::Medium,
        MoonButtonSize::Pill => Size::Large,
        MoonButtonSize::Custom { height, .. } => Size::Size(px(height)),
    }
}
