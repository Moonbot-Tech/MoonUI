use std::{borrow::BorrowMut, collections::HashMap, rc::Rc};

use gpui::prelude::FluentBuilder;
use gpui::*;
use serde::{Deserialize, Serialize};

use super::{
    background::MoonBackgroundPolicy,
    button::{MoonButton, MoonButtonSize, MoonButtonVariant},
    text::MoonText,
    theme::MoonTheme,
    tokens::{MoonPalette, rgba_from},
};
use crate::event::InteractiveElementExt as _;

const DOCK_RESIZE_HIT_SIZE: f32 = 6.0;
const DOCK_MIN_SIDE_SIZE: f32 = 112.0;
const DOCK_MIN_CENTER_SIZE: f32 = 220.0;
const DOCK_MIN_BOTTOM_SIZE: f32 = 104.0;
const DOCK_TILE_MIN_W: f32 = 160.0;
const DOCK_TILE_MIN_H: f32 = 96.0;
const DOCK_TILE_SNAP: f32 = 4.0;

pub enum DockEvent {
    LayoutChanged,
    DetachRequested {
        panel_name: SharedString,
    },
    /// The close (×) button of a panel was clicked. The dock does NOT remove the panel
    /// itself — the host app decides what to do (e.g. move it back to its home tab strip
    /// instead of destroying it). Emitted instead of an internal `remove_panel_by_name`.
    PanelCloseRequested {
        panel_name: SharedString,
    },
}

pub enum PanelEvent {
    LayoutChanged,
    ZoomIn,
    ZoomOut,
    Close,
    Detach,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DockPlacement {
    #[default]
    Center,
    Left,
    Right,
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockSplitPlacement {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum DockRoot {
    Center,
    Left,
    Right,
    Bottom,
}

#[derive(Clone, Debug)]
enum DockResizeTarget {
    OuterLeft,
    OuterRight,
    OuterBottom,
    Split {
        root: DockRoot,
        path: Vec<usize>,
        after_ix: usize,
    },
}

#[derive(Clone, Debug)]
struct DockResizeDrag {
    dock_id: EntityId,
    target: DockResizeTarget,
}

impl Render for DockResizeDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

#[derive(Clone, Debug)]
struct DockTabDrag {
    dock_id: EntityId,
    root: DockRoot,
    path: Vec<usize>,
    panel_name: SharedString,
    /// Whether this panel may participate in split drops (true only for panels with
    /// `show_dock_header`). A split drop is accepted only when BOTH the dragged panel and
    /// the target slot are splittable — so e.g. a bottom dock panel can split the bottom
    /// strip but cannot be dropped into the chart slot.
    splittable: bool,
}

impl Render for DockTabDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DockTileDragKind {
    Move,
    ResizeRight,
    ResizeBottom,
    ResizeBottomRight,
}

#[derive(Clone, Debug)]
struct DockTileDrag {
    dock_id: EntityId,
    root: DockRoot,
    path: Vec<usize>,
    ix: usize,
    kind: DockTileDragKind,
}

impl Render for DockTileDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

#[derive(Clone, Debug)]
struct DockTileDragStart {
    root: DockRoot,
    path: Vec<usize>,
    ix: usize,
    cursor: Point<Pixels>,
    meta: TileMeta,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DockAreaState {
    #[serde(default)]
    pub version: Option<usize>,
    pub center: PanelState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_dock: Option<DockState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_dock: Option<DockState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bottom_dock: Option<DockState>,
}

impl Default for DockAreaState {
    fn default() -> Self {
        Self {
            version: None,
            center: PanelState::empty(),
            left_dock: None,
            right_dock: None,
            bottom_dock: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DockState {
    pub panel: PanelState,
    pub placement: DockPlacement,
    pub size: f32,
    pub open: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PanelState {
    pub panel_name: String,
    #[serde(default)]
    pub children: Vec<PanelState>,
    pub info: PanelInfo,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PanelInfo {
    #[serde(rename = "stack")]
    Stack { sizes: Vec<f32>, axis: usize },
    #[serde(rename = "tabs")]
    Tabs { active_index: usize },
    #[serde(rename = "panel")]
    Panel(serde_json::Value),
    #[serde(rename = "tiles")]
    Tiles { metas: Vec<TileMeta> },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TileMeta {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub z_index: usize,
}

impl Default for PanelInfo {
    fn default() -> Self {
        Self::Panel(serde_json::Value::Null)
    }
}

impl PanelInfo {
    pub fn panel(info: serde_json::Value) -> Self {
        Self::Panel(info)
    }

    pub fn tabs(active_index: usize) -> Self {
        Self::Tabs { active_index }
    }

    pub fn stack(sizes: Vec<f32>, horizontal: bool) -> Self {
        Self::Stack {
            sizes,
            axis: usize::from(!horizontal),
        }
    }

    pub fn active_index(&self) -> Option<usize> {
        match self {
            Self::Tabs { active_index } => Some(*active_index),
            _ => None,
        }
    }
}

impl PanelState {
    pub fn empty() -> Self {
        Self {
            panel_name: String::new(),
            children: Vec::new(),
            info: PanelInfo::default(),
        }
    }

    pub fn new(panel_name: impl Into<String>) -> Self {
        Self {
            panel_name: panel_name.into(),
            children: Vec::new(),
            info: PanelInfo::default(),
        }
    }

    pub fn child(mut self, child: PanelState) -> Self {
        self.children.push(child);
        self
    }

    pub fn info(mut self, info: PanelInfo) -> Self {
        self.info = info;
        self
    }

    pub fn to_item<C>(&self, window: &mut Window, cx: &mut C) -> DockItem
    where
        C: BorrowMut<App>,
    {
        match &self.info {
            PanelInfo::Tabs { active_index } => DockItem::Tabs {
                items: self
                    .children
                    .iter()
                    .flat_map(|child| child.to_item(window, cx).into_panels())
                    .collect(),
                active_ix: *active_index,
            },
            PanelInfo::Stack { axis, .. } => DockItem::Split {
                horizontal: *axis == 0,
                sizes: match &self.info {
                    // 0.0 sentinel (см. dump) → None (flex), иначе фиксированный размер.
                    PanelInfo::Stack { sizes, .. } => {
                        sizes.iter().map(|s| (*s > 0.0).then_some(*s)).collect()
                    }
                    _ => Vec::new(),
                },
                items: self
                    .children
                    .iter()
                    .map(|child| child.to_item(window, cx))
                    .collect(),
            },
            PanelInfo::Tiles { metas } => DockItem::Tiles {
                items: self
                    .children
                    .iter()
                    .flat_map(|child| child.to_item(window, cx).into_panels())
                    .collect(),
                metas: metas.clone(),
            },
            PanelInfo::Panel(_) => DockItem::Panel(MoonPanelRegistry::build_panel(
                &self.panel_name,
                self,
                &self.info,
                window,
                cx,
            )),
        }
    }
}

pub trait Panel: EventEmitter<PanelEvent> + Render {
    fn panel_name(&self) -> &'static str;

    fn tab_name(&self, _cx: &App) -> Option<SharedString> {
        None
    }

    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.panel_name()
    }

    fn title_suffix(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        None
    }

    fn closable(&self, _cx: &App) -> bool {
        true
    }

    fn zoomable(&self, _cx: &App) -> bool {
        true
    }

    fn detachable(&self, _cx: &App) -> bool {
        false
    }

    /// Whether to render the dock header (tab bar + panel controls) when this panel is the
    /// sole occupant of a slot (`DockItem::Panel`). Default false: a lone panel shows no
    /// header (e.g. a chart with its own tab strip). Dock panels that can be split out
    /// should return true so they keep a drag handle + close button outside the tab strip.
    fn show_dock_header(&self, _cx: &App) -> bool {
        false
    }

    fn visible(&self, _cx: &App) -> bool {
        true
    }

    fn set_active(&mut self, _active: bool, _window: &mut Window, _cx: &mut Context<Self>) {}

    fn set_zoomed(&mut self, _zoomed: bool, _window: &mut Window, _cx: &mut Context<Self>) {}

    fn dump(&self, _cx: &App) -> PanelState {
        PanelState::new(self.panel_name())
    }

    fn background_policy(&self, _cx: &App) -> MoonBackgroundPolicy {
        MoonBackgroundPolicy::Opaque
    }

    fn on_added_to(
        &mut self,
        _dock_area: WeakEntity<DockArea>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
    }

    fn toolbar_buttons(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Vec<AnyElement>> {
        None
    }

    fn on_removed(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {}
}

pub trait PanelView: 'static {
    fn panel_name(&self, cx: &App) -> SharedString;
    fn tab_name(&self, cx: &App) -> Option<SharedString>;
    fn title(&self, window: &mut Window, cx: &mut App) -> AnyElement;
    fn title_suffix(&self, window: &mut Window, cx: &mut App) -> Option<AnyElement>;
    fn render_panel(&self, window: &mut Window, cx: &mut App) -> AnyElement;
    fn dump(&self, cx: &App) -> PanelState;
    fn background_policy(&self, cx: &App) -> MoonBackgroundPolicy;
    fn closable(&self, cx: &App) -> bool;
    fn zoomable(&self, cx: &App) -> bool;
    fn detachable(&self, cx: &App) -> bool;
    fn show_dock_header(&self, cx: &App) -> bool;
    fn visible(&self, cx: &App) -> bool;
    fn set_active(&self, active: bool, window: &mut Window, cx: &mut App);
    fn set_zoomed(&self, zoomed: bool, window: &mut Window, cx: &mut App);
    fn on_added_to(&self, dock_area: WeakEntity<DockArea>, window: &mut Window, cx: &mut App);
    fn on_removed(&self, window: &mut Window, cx: &mut App);
    fn toolbar_buttons(&self, window: &mut Window, cx: &mut App) -> Option<Vec<AnyElement>>;
}

impl<T> PanelView for Entity<T>
where
    T: Panel,
{
    fn panel_name(&self, cx: &App) -> SharedString {
        SharedString::from(self.read(cx).panel_name())
    }

    fn tab_name(&self, cx: &App) -> Option<SharedString> {
        self.read(cx).tab_name(cx)
    }

    fn title(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        self.update(cx, |panel, cx| panel.title(window, cx).into_any_element())
    }

    fn title_suffix(&self, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        self.update(cx, |panel, cx| panel.title_suffix(window, cx))
    }

    fn render_panel(&self, _window: &mut Window, _cx: &mut App) -> AnyElement {
        AnyView::from(self.clone())
            .cached(StyleRefinement::default().size_full())
            .into_any_element()
    }

    fn dump(&self, cx: &App) -> PanelState {
        self.read(cx).dump(cx)
    }

    fn background_policy(&self, cx: &App) -> MoonBackgroundPolicy {
        self.read(cx).background_policy(cx)
    }

    fn closable(&self, cx: &App) -> bool {
        self.read(cx).closable(cx)
    }

    fn zoomable(&self, cx: &App) -> bool {
        self.read(cx).zoomable(cx)
    }

    fn detachable(&self, cx: &App) -> bool {
        self.read(cx).detachable(cx)
    }

    fn show_dock_header(&self, cx: &App) -> bool {
        self.read(cx).show_dock_header(cx)
    }

    fn visible(&self, cx: &App) -> bool {
        self.read(cx).visible(cx)
    }

    fn set_active(&self, active: bool, window: &mut Window, cx: &mut App) {
        self.update(cx, |panel, cx| panel.set_active(active, window, cx));
    }

    fn set_zoomed(&self, zoomed: bool, window: &mut Window, cx: &mut App) {
        self.update(cx, |panel, cx| panel.set_zoomed(zoomed, window, cx));
    }

    fn on_added_to(&self, dock_area: WeakEntity<DockArea>, window: &mut Window, cx: &mut App) {
        self.update(cx, |panel, cx| panel.on_added_to(dock_area, window, cx));
    }

    fn on_removed(&self, window: &mut Window, cx: &mut App) {
        self.update(cx, |panel, cx| panel.on_removed(window, cx));
    }

    fn toolbar_buttons(&self, window: &mut Window, cx: &mut App) -> Option<Vec<AnyElement>> {
        self.update(cx, |panel, cx| panel.toolbar_buttons(window, cx))
    }
}

pub struct MoonDockPanel {
    panel_name: SharedString,
    title: SharedString,
    render: MoonPanelRender,
    background_policy: MoonBackgroundPolicy,
    closable: bool,
    zoomable: bool,
    detachable: bool,
    show_dock_header: bool,
    visible: bool,
}

type MoonPanelRender = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;

impl MoonDockPanel {
    pub fn new(
        panel_name: impl Into<SharedString>,
        title: impl Into<SharedString>,
        render: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            panel_name: panel_name.into(),
            title: title.into(),
            render: Rc::new(render),
            background_policy: MoonBackgroundPolicy::Opaque,
            closable: true,
            zoomable: true,
            detachable: false,
            show_dock_header: false,
            visible: true,
        }
    }

    pub fn background_policy(mut self, policy: MoonBackgroundPolicy) -> Self {
        self.background_policy = policy;
        self
    }

    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    pub fn zoomable(mut self, zoomable: bool) -> Self {
        self.zoomable = zoomable;
        self
    }

    pub fn detachable(mut self, detachable: bool) -> Self {
        self.detachable = detachable;
        self
    }

    pub fn show_dock_header(mut self, show_dock_header: bool) -> Self {
        self.show_dock_header = show_dock_header;
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }
}

impl PanelView for MoonDockPanel {
    fn panel_name(&self, _cx: &App) -> SharedString {
        self.panel_name.clone()
    }

    fn tab_name(&self, _cx: &App) -> Option<SharedString> {
        None
    }

    fn title(&self, _window: &mut Window, cx: &mut App) -> AnyElement {
        let p = MoonPalette::active(cx);
        MoonText::new(self.title.clone())
            .color(p.text_soft)
            .font_size(10.5)
            .line_height(13.0)
            .weight(600.0)
            .mono(true)
            .uppercase(false)
            .render()
            .into_any_element()
    }

    fn title_suffix(&self, _window: &mut Window, _cx: &mut App) -> Option<AnyElement> {
        None
    }

    fn render_panel(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        (self.render)(window, cx)
    }

    fn dump(&self, _cx: &App) -> PanelState {
        PanelState::new(self.panel_name.to_string())
    }

    fn background_policy(&self, _cx: &App) -> MoonBackgroundPolicy {
        self.background_policy
    }

    fn closable(&self, _cx: &App) -> bool {
        self.closable
    }

    fn zoomable(&self, _cx: &App) -> bool {
        self.zoomable
    }

    fn detachable(&self, _cx: &App) -> bool {
        self.detachable
    }

    fn show_dock_header(&self, _cx: &App) -> bool {
        self.show_dock_header
    }

    fn visible(&self, _cx: &App) -> bool {
        self.visible
    }

    fn set_active(&self, _active: bool, _window: &mut Window, _cx: &mut App) {}

    fn set_zoomed(&self, _zoomed: bool, _window: &mut Window, _cx: &mut App) {}

    fn on_added_to(&self, _dock_area: WeakEntity<DockArea>, _window: &mut Window, _cx: &mut App) {}

    fn on_removed(&self, _window: &mut Window, _cx: &mut App) {}

    fn toolbar_buttons(&self, _window: &mut Window, _cx: &mut App) -> Option<Vec<AnyElement>> {
        None
    }
}

#[derive(Clone)]
pub enum DockItem {
    Empty,
    Panel(Rc<dyn PanelView>),
    Tabs {
        items: Vec<Rc<dyn PanelView>>,
        active_ix: usize,
    },
    Tiles {
        items: Vec<Rc<dyn PanelView>>,
        metas: Vec<TileMeta>,
    },
    Split {
        horizontal: bool,
        items: Vec<DockItem>,
        sizes: Vec<Option<f32>>,
    },
}

impl DockItem {
    pub fn panel(panel: Rc<dyn PanelView>) -> Self {
        Self::Panel(panel)
    }

    pub fn tab<T>(
        panel: Entity<T>,
        dock_area: &WeakEntity<DockArea>,
        window: &mut Window,
        cx: &mut App,
    ) -> Self
    where
        T: Panel,
    {
        panel.update(cx, |panel, cx| {
            panel.on_added_to(dock_area.clone(), window, cx);
        });
        Self::Panel(Rc::new(panel))
    }

    pub fn tabs(
        items: Vec<Rc<dyn PanelView>>,
        dock_area: &WeakEntity<DockArea>,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        for item in &items {
            item.on_added_to(dock_area.clone(), window, cx);
        }
        Self::Tabs {
            items,
            active_ix: 0,
        }
    }

    pub fn split(horizontal: bool, items: Vec<DockItem>) -> Self {
        Self::Split {
            horizontal,
            items,
            sizes: Vec::new(),
        }
    }

    pub fn tiles(items: Vec<Rc<dyn PanelView>>, metas: Vec<TileMeta>) -> Self {
        Self::Tiles { items, metas }
    }

    pub fn split_with_sizes(
        axis: Axis,
        items: Vec<DockItem>,
        sizes: Vec<Option<Pixels>>,
        _dock_area: &WeakEntity<DockArea>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self {
        Self::Split {
            horizontal: axis == Axis::Horizontal,
            items,
            sizes: sizes.into_iter().map(|size| size.map(f32::from)).collect(),
        }
    }

    pub fn h_split(
        items: Vec<DockItem>,
        dock_area: &WeakEntity<DockArea>,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        Self::split_with_sizes(Axis::Horizontal, items, Vec::new(), dock_area, window, cx)
    }

    pub fn v_split(
        items: Vec<DockItem>,
        dock_area: &WeakEntity<DockArea>,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        Self::split_with_sizes(Axis::Vertical, items, Vec::new(), dock_area, window, cx)
    }

    fn with_panel_added(self, panel: Rc<dyn PanelView>) -> Self {
        match self {
            DockItem::Empty => DockItem::Panel(panel),
            DockItem::Panel(existing) => DockItem::Tabs {
                items: vec![existing, panel],
                active_ix: 1,
            },
            DockItem::Tabs {
                mut items,
                active_ix: _,
            } => {
                items.push(panel);
                DockItem::Tabs {
                    active_ix: items.len() - 1,
                    items,
                }
            }
            DockItem::Split { .. } | DockItem::Tiles { .. } => DockItem::Tabs {
                items: vec![panel],
                active_ix: 0,
            },
        }
    }

    /// Insert `panel` into the first `Tabs` node that already contains a panel whose
    /// name is in `sibling_names`, at `ix` (clamped to the tab count), making it active.
    /// Used to restore a detached/closed panel back to its "home" tab strip WITHOUT
    /// collapsing the surrounding split (unlike `with_panel_added`, which replaces a
    /// whole `Split` with a single `Tabs`). Returns true if it was inserted.
    fn insert_into_named_tabs(
        &mut self,
        panel: Rc<dyn PanelView>,
        ix: usize,
        sibling_names: &[&str],
        dock_area: &WeakEntity<DockArea>,
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        match self {
            DockItem::Tabs { items, active_ix } => {
                if items.iter().any(|p| {
                    sibling_names
                        .iter()
                        .any(|n| p.panel_name(cx).as_ref() == *n)
                }) {
                    let ix = ix.min(items.len());
                    panel.on_added_to(dock_area.clone(), window, cx);
                    items.insert(ix, panel);
                    *active_ix = ix;
                    return true;
                }
                false
            }
            DockItem::Panel(existing) => {
                // Slot holds a lone sibling (panels were split apart): merge the returning
                // panel with it into a Tabs so it rejoins as a tab — instead of failing and
                // letting the caller fall back to a destructive full-window add.
                if sibling_names
                    .iter()
                    .any(|n| existing.panel_name(cx).as_ref() == *n)
                {
                    panel.on_added_to(dock_area.clone(), window, cx);
                    let existing = existing.clone();
                    let (items, active_ix) = if ix == 0 {
                        (vec![panel, existing], 0)
                    } else {
                        (vec![existing, panel], 1)
                    };
                    *self = DockItem::Tabs { items, active_ix };
                    return true;
                }
                false
            }
            DockItem::Split { items, .. } => {
                for it in items.iter_mut() {
                    if it.insert_into_named_tabs(
                        panel.clone(),
                        ix,
                        sibling_names,
                        dock_area,
                        window,
                        cx,
                    ) {
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }

    fn notify_added(&self, dock_area: &WeakEntity<DockArea>, window: &mut Window, cx: &mut App) {
        match self {
            DockItem::Empty => {}
            DockItem::Panel(panel) => panel.on_added_to(dock_area.clone(), window, cx),
            DockItem::Tabs { items, .. } => {
                for panel in items {
                    panel.on_added_to(dock_area.clone(), window, cx);
                }
            }
            DockItem::Tiles { items, .. } => {
                for panel in items {
                    panel.on_added_to(dock_area.clone(), window, cx);
                }
            }
            DockItem::Split { items, .. } => {
                for item in items {
                    item.notify_added(dock_area, window, cx);
                }
            }
        }
    }

    fn is_empty(&self) -> bool {
        matches!(self, DockItem::Empty)
    }

    fn background_policy(&self, cx: &App) -> MoonBackgroundPolicy {
        match self {
            DockItem::Empty => MoonBackgroundPolicy::Opaque,
            DockItem::Panel(panel) => panel.background_policy(cx),
            DockItem::Tabs { items, active_ix } => items
                .get((*active_ix).min(items.len().saturating_sub(1)))
                .map(|panel| panel.background_policy(cx))
                .unwrap_or(MoonBackgroundPolicy::Opaque),
            // Composite containers can mix transparent chart hosts and opaque UI
            // panels. Their own surface must stay unpainted; each child slot
            // applies its own policy.
            DockItem::Split { .. } | DockItem::Tiles { .. } => MoonBackgroundPolicy::NoFill,
        }
    }

    fn remove_panel_named(
        self,
        panel_name: &str,
        window: &mut Window,
        cx: &mut App,
    ) -> (Self, bool) {
        match self {
            DockItem::Empty => (DockItem::Empty, false),
            DockItem::Panel(panel) => {
                if panel.panel_name(cx).to_string() == panel_name {
                    panel.on_removed(window, cx);
                    (DockItem::Empty, true)
                } else {
                    (DockItem::Panel(panel), false)
                }
            }
            DockItem::Tabs { items, active_ix } => {
                let before = items.len();
                let mut items: Vec<_> = items
                    .into_iter()
                    .filter(|panel| {
                        let remove = panel.panel_name(cx).to_string() == panel_name;
                        if remove {
                            panel.on_removed(window, cx);
                        }
                        !remove
                    })
                    .collect();
                let removed = before != items.len();
                match items.len() {
                    0 => (DockItem::Empty, removed),
                    1 => (DockItem::Panel(items.remove(0)), removed),
                    len => (
                        DockItem::Tabs {
                            items,
                            active_ix: active_ix.min(len.saturating_sub(1)),
                        },
                        removed,
                    ),
                }
            }
            DockItem::Split {
                horizontal,
                items,
                sizes,
            } => {
                let mut removed = false;
                let mut kept_items = Vec::new();
                let mut kept_sizes = Vec::new();
                for (ix, item) in items.into_iter().enumerate() {
                    let (item, did_remove) = item.remove_panel_named(panel_name, window, cx);
                    removed |= did_remove;
                    if !item.is_empty() {
                        kept_items.push(item);
                        kept_sizes.push(sizes.get(ix).copied().unwrap_or(None));
                    }
                }
                match kept_items.len() {
                    0 => (DockItem::Empty, removed),
                    1 => (kept_items.remove(0), removed),
                    _ => (
                        DockItem::Split {
                            horizontal,
                            items: kept_items,
                            sizes: kept_sizes,
                        },
                        removed,
                    ),
                }
            }
            DockItem::Tiles { items, metas } => {
                let before = items.len();
                let mut kept_items = Vec::new();
                let mut kept_metas = Vec::new();
                for (ix, panel) in items.into_iter().enumerate() {
                    if panel.panel_name(cx).to_string() == panel_name {
                        panel.on_removed(window, cx);
                    } else {
                        kept_items.push(panel);
                        kept_metas.push(metas.get(ix).copied().unwrap_or(TileMeta {
                            x: 0.0,
                            y: 0.0,
                            w: 240.0,
                            h: 160.0,
                            z_index: ix,
                        }));
                    }
                }
                let removed = before != kept_items.len();
                match kept_items.len() {
                    0 => (DockItem::Empty, before != 0),
                    1 => (DockItem::Panel(kept_items.remove(0)), removed),
                    _ => (
                        DockItem::Tiles {
                            items: kept_items,
                            metas: kept_metas,
                        },
                        removed,
                    ),
                }
            }
        }
    }

    fn into_panels(self) -> Vec<Rc<dyn PanelView>> {
        match self {
            DockItem::Empty => Vec::new(),
            DockItem::Panel(panel) => vec![panel],
            DockItem::Tabs { items, .. } => items,
            DockItem::Tiles { items, .. } => items,
            DockItem::Split { items, .. } => items
                .into_iter()
                .flat_map(|item| item.into_panels())
                .collect(),
        }
    }

    fn find_panel_named(&self, panel_name: &str, cx: &App) -> Option<Rc<dyn PanelView>> {
        match self {
            DockItem::Empty => None,
            DockItem::Panel(panel) => {
                (panel.panel_name(cx).to_string() == panel_name).then_some(panel.clone())
            }
            DockItem::Tabs { items, .. } | DockItem::Tiles { items, .. } => items
                .iter()
                .find(|panel| panel.panel_name(cx).to_string() == panel_name)
                .cloned(),
            DockItem::Split { items, .. } => items
                .iter()
                .find_map(|item| item.find_panel_named(panel_name, cx)),
        }
    }

    /// Путь к НАИМЕНЬШЕМУ поддереву, содержащему ВСЕ присутствующие из `names` (относительно
    /// self). Для восстановления сплита: соседний слот мог быть вложенным сплитом (столбец из
    /// панелей) — его надо обернуть целиком, а не один лист внутри. `None`, если ни одна из
    /// `names` не найдена. Отсутствующие имена игнорируются (могли открепить/закрыть).
    fn smallest_subtree_with_all(&self, names: &[&str], cx: &App) -> Option<Vec<usize>> {
        let present: Vec<&str> = names
            .iter()
            .copied()
            .filter(|n| self.find_panel_named(n, cx).is_some())
            .collect();
        if present.is_empty() {
            return None;
        }
        self.smallest_node_with(&present, cx)
    }

    /// Путь к наименьшему узлу, содержащему ВСЕ `names` (все обязаны присутствовать). Спускаемся
    /// в ребёнка, который всё ещё держит все имена; если ни один не держит — этот узел и есть
    /// наименьший (`[]`).
    fn smallest_node_with(&self, names: &[&str], cx: &App) -> Option<Vec<usize>> {
        if !names.iter().all(|n| self.find_panel_named(n, cx).is_some()) {
            return None;
        }
        if let DockItem::Split { items, .. } = self {
            for (i, child) in items.iter().enumerate() {
                if let Some(mut sub) = child.smallest_node_with(names, cx) {
                    let mut path = vec![i];
                    path.append(&mut sub);
                    return Some(path);
                }
            }
        }
        Some(Vec::new())
    }

    /// Имя любой панели в этом узле, КРОМЕ `exclude` — «якорь» целевого слота для split:
    /// переживает схлопывание узла при take (по нему находим слот заново).
    fn first_panel_name_excluding(&self, exclude: &str, cx: &App) -> Option<String> {
        match self {
            DockItem::Empty => None,
            DockItem::Panel(p) => {
                let n = p.panel_name(cx).to_string();
                (n != exclude).then_some(n)
            }
            DockItem::Tabs { items, .. } | DockItem::Tiles { items, .. } => items
                .iter()
                .map(|p| p.panel_name(cx).to_string())
                .find(|n| n != exclude),
            DockItem::Split { items, .. } => items
                .iter()
                .find_map(|it| it.first_panel_name_excluding(exclude, cx)),
        }
    }

    /// Путь к УЗЛУ (Panel/Tabs/Tiles), содержащему панель `name`. Относительно self.
    fn find_panel_path(&self, name: &str, cx: &App) -> Option<Vec<usize>> {
        match self {
            DockItem::Empty => None,
            DockItem::Panel(p) => (p.panel_name(cx).as_ref() == name).then(Vec::new),
            DockItem::Tabs { items, .. } | DockItem::Tiles { items, .. } => items
                .iter()
                .any(|p| p.panel_name(cx).as_ref() == name)
                .then(Vec::new),
            DockItem::Split { items, .. } => items.iter().enumerate().find_map(|(i, it)| {
                it.find_panel_path(name, cx).map(|mut sub| {
                    let mut path = vec![i];
                    path.append(&mut sub);
                    path
                })
            }),
        }
    }

    /// Затолкать панель вкладкой в первый встреченный Tabs (неразрушающий фолбэк, чтобы не
    /// схлопывать центр через with_panel_added). True — если нашёлся Tabs.
    fn try_push_into_first_tabs(&mut self, panel: Rc<dyn PanelView>) -> bool {
        match self {
            DockItem::Tabs { items, active_ix } => {
                items.push(panel);
                *active_ix = items.len() - 1;
                true
            }
            DockItem::Split { items, .. } => {
                for it in items.iter_mut() {
                    if it.try_push_into_first_tabs(panel.clone()) {
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }

    fn take_panel_named(self, panel_name: &str, cx: &App) -> (Self, Option<Rc<dyn PanelView>>) {
        match self {
            DockItem::Empty => (DockItem::Empty, None),
            DockItem::Panel(panel) => {
                if panel.panel_name(cx).to_string() == panel_name {
                    (DockItem::Empty, Some(panel))
                } else {
                    (DockItem::Panel(panel), None)
                }
            }
            DockItem::Tabs { items, active_ix } => {
                let mut taken = None;
                let mut kept = Vec::with_capacity(items.len());
                for panel in items {
                    if panel.panel_name(cx).to_string() == panel_name {
                        taken = Some(panel);
                    } else {
                        kept.push(panel);
                    }
                }
                let item = match kept.len() {
                    0 => DockItem::Empty,
                    1 => DockItem::Panel(kept.remove(0)),
                    len => DockItem::Tabs {
                        items: kept,
                        active_ix: active_ix.min(len.saturating_sub(1)),
                    },
                };
                (item, taken)
            }
            DockItem::Tiles { items, metas } => {
                let mut taken = None;
                let mut kept_items = Vec::with_capacity(items.len());
                let mut kept_metas = Vec::with_capacity(items.len());
                for (ix, panel) in items.into_iter().enumerate() {
                    if panel.panel_name(cx).to_string() == panel_name {
                        taken = Some(panel);
                    } else {
                        kept_items.push(panel);
                        kept_metas.push(metas.get(ix).copied().unwrap_or(TileMeta {
                            x: 12.0 + ix as f32 * 18.0,
                            y: 12.0 + ix as f32 * 18.0,
                            w: 320.0,
                            h: 200.0,
                            z_index: ix,
                        }));
                    }
                }
                let item = match kept_items.len() {
                    0 => DockItem::Empty,
                    1 => DockItem::Panel(kept_items.remove(0)),
                    _ => DockItem::Tiles {
                        items: kept_items,
                        metas: kept_metas,
                    },
                };
                (item, taken)
            }
            DockItem::Split {
                horizontal,
                items,
                sizes,
            } => {
                let mut taken = None;
                let mut kept_items = Vec::new();
                let mut kept_sizes = Vec::new();
                for (ix, item) in items.into_iter().enumerate() {
                    let (item, child_taken) = item.take_panel_named(panel_name, cx);
                    if taken.is_none() {
                        taken = child_taken;
                    }
                    if !item.is_empty() {
                        kept_items.push(item);
                        kept_sizes.push(sizes.get(ix).copied().unwrap_or(None));
                    }
                }
                let item = match kept_items.len() {
                    0 => DockItem::Empty,
                    1 => kept_items.remove(0),
                    _ => DockItem::Split {
                        horizontal,
                        items: kept_items,
                        sizes: kept_sizes,
                    },
                };
                (item, taken)
            }
        }
    }

    fn dump(&self, cx: &App) -> PanelState {
        match self {
            DockItem::Empty => PanelState::empty(),
            DockItem::Panel(panel) => panel.dump(cx),
            DockItem::Tabs { items, active_ix } => {
                let mut state = PanelState::new("tabs").info(PanelInfo::tabs(*active_ix));
                state.children = items.iter().map(|panel| panel.dump(cx)).collect();
                state
            }
            DockItem::Tiles { items, metas } => {
                let mut state = PanelState::new("tiles").info(PanelInfo::Tiles {
                    metas: metas.clone(),
                });
                state.children = items.iter().map(|panel| panel.dump(cx)).collect();
                state
            }
            DockItem::Split {
                horizontal,
                items,
                sizes,
            } => {
                // По размеру на КАЖДУЮ панель (выравнивание sizes↔items). None (flex,
                // без фикс. размера) → 0.0 как sentinel; to_item обратно превращает 0.0 в
                // None. Старый `flatten` выкидывал None и сдвигал размеры на чужие панели
                // при load → «все области неверные» после перезапуска.
                let sizes: Vec<f32> = (0..items.len())
                    .map(|i| sizes.get(i).copied().flatten().unwrap_or(0.0))
                    .collect();
                let mut state = PanelState::new("stack").info(PanelInfo::stack(sizes, *horizontal));
                state.children = items.iter().map(|item| item.dump(cx)).collect();
                state
            }
        }
    }
}

type PanelFactory =
    Rc<dyn Fn(&PanelState, &PanelInfo, &mut Window, &mut App) -> Rc<dyn PanelView> + 'static>;

#[derive(Default)]
struct MoonPanelRegistry {
    factories: HashMap<String, PanelFactory>,
}

impl Global for MoonPanelRegistry {}

impl MoonPanelRegistry {
    fn build_panel<C>(
        panel_name: &str,
        state: &PanelState,
        info: &PanelInfo,
        window: &mut Window,
        cx: &mut C,
    ) -> Rc<dyn PanelView>
    where
        C: BorrowMut<App>,
    {
        let factory = cx
            .borrow_mut()
            .default_global::<MoonPanelRegistry>()
            .factories
            .get(panel_name)
            .cloned();

        if let Some(factory) = factory {
            return factory(state, info, window, cx.borrow_mut());
        }

        let title = SharedString::from(if panel_name.is_empty() {
            "Invalid Panel"
        } else {
            panel_name
        });
        Rc::new(MoonDockPanel::new(panel_name.to_string(), title, |_, _| {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child("Missing panel factory")
                .into_any_element()
        }))
    }
}

pub fn register_panel<F>(cx: &mut App, panel_name: &str, factory: F)
where
    F: Fn(&PanelState, &PanelInfo, &mut Window, &mut App) -> Rc<dyn PanelView> + 'static,
{
    cx.default_global::<MoonPanelRegistry>()
        .factories
        .insert(panel_name.to_string(), Rc::new(factory));
}

#[derive(Default)]
struct MoonTabPanelRuntimeState {
    active_ix: usize,
}

#[derive(IntoElement)]
pub struct TabPanel {
    id: SharedString,
    items: Vec<Rc<dyn PanelView>>,
    active_ix: usize,
    dock_area: Option<WeakEntity<DockArea>>,
    dock_root: Option<DockRoot>,
    dock_path: Vec<usize>,
    background_policy: MoonBackgroundPolicy,
    content_background_policy: Option<MoonBackgroundPolicy>,
    header_background_policy: MoonBackgroundPolicy,
    show_header: bool,
    show_panel_controls: bool,
}

impl TabPanel {
    pub fn new(id: impl Into<SharedString>, items: Vec<Rc<dyn PanelView>>) -> Self {
        Self {
            id: id.into(),
            items,
            active_ix: 0,
            dock_area: None,
            dock_root: None,
            dock_path: Vec::new(),
            background_policy: MoonBackgroundPolicy::Opaque,
            content_background_policy: None,
            header_background_policy: MoonBackgroundPolicy::Opaque,
            show_header: true,
            show_panel_controls: true,
        }
    }

    pub fn active_index(mut self, active_ix: usize) -> Self {
        self.active_ix = active_ix;
        self
    }

    pub fn background_policy(mut self, policy: MoonBackgroundPolicy) -> Self {
        self.background_policy = policy;
        self
    }

    pub fn content_background_policy(mut self, policy: MoonBackgroundPolicy) -> Self {
        self.content_background_policy = Some(policy);
        self
    }

    pub fn header_background_policy(mut self, policy: MoonBackgroundPolicy) -> Self {
        self.header_background_policy = policy;
        self
    }

    pub fn show_header(mut self, show_header: bool) -> Self {
        self.show_header = show_header;
        self
    }

    pub fn show_panel_controls(mut self, show_panel_controls: bool) -> Self {
        self.show_panel_controls = show_panel_controls;
        self
    }

    fn dock_context(
        mut self,
        dock_area: WeakEntity<DockArea>,
        root: DockRoot,
        path: Vec<usize>,
    ) -> Self {
        self.dock_area = Some(dock_area);
        self.dock_root = Some(root);
        self.dock_path = path;
        self
    }
}

impl RenderOnce for TabPanel {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = MoonPalette::active(cx);
        let tokens = MoonTheme::active_tokens(cx);
        let state = window.use_keyed_state(
            ElementId::from(SharedString::from(format!("{}:state", self.id))),
            cx,
            |_, _| MoonTabPanelRuntimeState {
                active_ix: self.active_ix,
            },
        );
        let active_ix = state
            .read(cx)
            .active_ix
            .min(self.items.len().saturating_sub(1));
        let active_panel = self.items.get(active_ix).cloned();
        let content_policy = self
            .content_background_policy
            .or_else(|| {
                active_panel
                    .as_ref()
                    .map(|panel| panel.background_policy(cx))
            })
            .unwrap_or(MoonBackgroundPolicy::Opaque);
        let parent_view = window.current_view();
        if let Some(panel) = active_panel.as_ref() {
            panel.set_active(true, window, cx);
        }

        let mut root = div()
            .id(ElementId::from(self.id.clone()))
            .relative()
            .size_full()
            .min_w(px(0.0))
            .min_h(px(0.0))
            .flex()
            .flex_col()
            .overflow_hidden()
            .border(px(1.0))
            .border_color(rgba_from(p.border, 1.0));
        root = self.background_policy.apply(root, p.shell_high, 0.98);

        if self.show_header {
            let mut header = div()
                .id(ElementId::from(SharedString::from(format!(
                    "{}:header",
                    self.id
                ))))
                .h(px(tokens.fit_height(29.0, 13.0, 8.0)))
                .flex()
                .flex_none()
                .items_center()
                .gap(px(tokens.ui(4.0)))
                .px(px(tokens.ui(6.0)))
                .border_b(px(1.0))
                .border_color(rgba_from(p.border, 1.0));
            header = self.header_background_policy.apply(header, p.panel, 1.0);

            for (ix, panel) in self.items.iter().enumerate() {
                let selected = ix == active_ix;
                let state = state.clone();
                let dock_area = self.dock_area.clone();
                let dock_root = self.dock_root;
                let dock_path = self.dock_path.clone();
                let panel_name = panel.panel_name(cx);
                let tab_label = panel.tab_name(cx).unwrap_or_else(|| panel.panel_name(cx));
                // Вид вкладки = как у верхних (MoonTabStrip): высота 28, mono-текст,
                // активная подсвечивается янтарным underline снизу, а не фоном Panel.
                // drag/drop/double-click ниже навешиваются на этот же tab_host — поэтому
                // механика докинга не зависит от вида.
                let mut tab_host = div()
                    .id(ElementId::from(SharedString::from(format!(
                        "{}:tab-host:{ix}",
                        self.id
                    ))))
                    .relative()
                    .h(px(tokens.fit_height(28.0, 13.0, 7.5)))
                    .flex()
                    .flex_none()
                    .items_center()
                    .px(px(tokens.ui(8.0)))
                    .cursor_pointer()
                    .when(!selected, |this| {
                        this.hover(move |h| h.bg(rgba_from(p.overlay, 0.018)))
                            .active(move |a| a.bg(rgba_from(p.overlay, 0.012)))
                    })
                    .child(
                        div().mt(px(tokens.ui(2.0))).child(
                            MoonText::new(tab_label)
                                .color(if selected { p.text } else { p.text_muted })
                                .font_size(10.0)
                                .line_height(13.0)
                                .weight(if selected { 600.0 } else { 400.0 })
                                .mono(true)
                                .render(),
                        ),
                    )
                    .on_click(move |_, _, cx| {
                        state.update(cx, |state, _| state.active_ix = ix);
                        if let (Some(dock_area), Some(root)) = (dock_area.as_ref(), dock_root) {
                            _ = dock_area.update(cx, |dock, cx| {
                                if dock.set_tabs_active_index(root, &dock_path, ix) {
                                    cx.emit(DockEvent::LayoutChanged);
                                    cx.notify();
                                }
                            });
                        }
                        cx.notify(parent_view);
                    });
                if selected {
                    // Точный underline активной вкладки из палитры (тот же, что у верхних).
                    tab_host = tab_host.child(super::tab::moon_active_tab_underline_scaled(
                        p,
                        tokens.clone(),
                    ));
                }
                if let (Some(dock_area), Some(root)) = (self.dock_area.clone(), self.dock_root) {
                    if let Some(dock_entity) = dock_area.upgrade() {
                        let drag = DockTabDrag {
                            dock_id: dock_entity.entity_id(),
                            root,
                            path: self.dock_path.clone(),
                            panel_name: panel_name.clone(),
                            splittable: panel.show_dock_header(cx),
                        };
                        let drop_dock_area = dock_area.clone();
                        let drop_path = self.dock_path.clone();
                        tab_host = tab_host
                            .on_drag(drag, |drag, _, _, cx| {
                                cx.stop_propagation();
                                cx.new(|_| drag.clone())
                            })
                            .drag_over::<DockTabDrag>(|style, _, _, cx| {
                                let p = MoonPalette::active(cx);
                                style
                                    .border_l(px(2.0))
                                    .border_color(rgba_from(p.accent, 0.9))
                            })
                            .on_drop(move |drag: &DockTabDrag, _window, cx| {
                                if drag.dock_id != dock_entity.entity_id() {
                                    return;
                                }
                                _ = drop_dock_area.update(cx, |dock, cx| {
                                    let changed = if drag.root == root && drag.path == drop_path {
                                        dock.move_tab_before(
                                            root,
                                            &drop_path,
                                            drag.panel_name.as_ref(),
                                            ix,
                                            cx,
                                        )
                                    } else {
                                        dock.move_panel_to_tabs(
                                            drag.panel_name.as_ref(),
                                            root,
                                            &drop_path,
                                            ix,
                                            cx,
                                        )
                                    };
                                    if changed {
                                        cx.emit(DockEvent::LayoutChanged);
                                        cx.notify();
                                    }
                                });
                            })
                            .on_double_click({
                                let dbl_area = dock_area.clone();
                                let dbl_name = panel_name.clone();
                                move |_, _, cx| {
                                    // Дабл-клик по вкладке = вынос в окно (как кнопка ⧉).
                                    // Хост (терминал) решает по panel_name (DetachRequested).
                                    if let Some(area) = dbl_area.upgrade() {
                                        area.update(cx, |_d, cx| {
                                            cx.emit(DockEvent::DetachRequested {
                                                panel_name: dbl_name.clone(),
                                            });
                                        });
                                    }
                                }
                            });
                    }
                }
                header = header.child(tab_host);
            }

            header = header.child(div().flex_1());
            if let Some(panel) = active_panel.as_ref() {
                if let Some(buttons) = panel.toolbar_buttons(window, cx) {
                    for button in buttons {
                        header = header.child(button);
                    }
                }
                if self.show_panel_controls {
                    let panel_name = panel.panel_name(cx);
                    if panel.detachable(cx) {
                        let dock_area = self.dock_area.clone();
                        header = header.child(
                            MoonButton::new(format!("{}:detach", self.id))
                                .label("⧉")
                                .size(MoonButtonSize::Micro)
                                .variant(MoonButtonVariant::Ghost)
                                .on_click({
                                    let panel_name = panel_name.clone();
                                    move |_, _, cx| {
                                        if let Some(dock_area) =
                                            dock_area.as_ref().and_then(|area| area.upgrade())
                                        {
                                            dock_area.update(cx, |_dock, cx| {
                                                cx.emit(DockEvent::DetachRequested {
                                                    panel_name: panel_name.clone(),
                                                });
                                            });
                                        }
                                    }
                                })
                                .render(),
                        );
                    }
                    if panel.zoomable(cx) {
                        let dock_area = self.dock_area.clone();
                        let zoom_label = if self
                            .dock_area
                            .as_ref()
                            .and_then(|dock_area| dock_area.upgrade())
                            .and_then(|dock_area| dock_area.read(cx).zoomed_panel.as_ref().cloned())
                            .as_ref()
                            == Some(&panel_name)
                        {
                            "□"
                        } else {
                            "▣"
                        };
                        header = header.child(
                            MoonButton::new(format!("{}:zoom", self.id))
                                .label(zoom_label)
                                .size(MoonButtonSize::Micro)
                                .variant(MoonButtonVariant::Ghost)
                                .on_click({
                                    let panel_name = panel_name.clone();
                                    move |_, window, cx| {
                                        if let Some(dock_area) =
                                            dock_area.as_ref().and_then(|area| area.upgrade())
                                        {
                                            dock_area.update(cx, |dock, cx| {
                                                dock.toggle_zoom_panel(
                                                    panel_name.clone(),
                                                    window,
                                                    cx,
                                                );
                                            });
                                        }
                                    }
                                })
                                .render(),
                        );
                    }
                    if panel.closable(cx) {
                        let dock_area = self.dock_area.clone();
                        header = header.child(
                            MoonButton::new(format!("{}:close", self.id))
                                .label("×")
                                .size(MoonButtonSize::Micro)
                                .variant(MoonButtonVariant::Ghost)
                                .on_click({
                                    let panel_name = panel_name.clone();
                                    move |_, _window, cx| {
                                        if let Some(dock_area) =
                                            dock_area.as_ref().and_then(|area| area.upgrade())
                                        {
                                            // Не удаляем сами — отдаём решение хосту (вернуть в
                                            // домашнюю строку, а не уничтожить). См. DockEvent.
                                            dock_area.update(cx, |_dock, cx| {
                                                cx.emit(DockEvent::PanelCloseRequested {
                                                    panel_name: panel_name.clone(),
                                                });
                                            });
                                        }
                                    }
                                })
                                .render(),
                        );
                    }
                }
            }

            root = root.child(header);
        }

        let mut content = div()
            .id(ElementId::from(SharedString::from(format!(
                "{}:content",
                self.id
            ))))
            .relative()
            .flex_1()
            .w_full()
            .min_h(px(0.0))
            .overflow_hidden();
        content = content_policy.apply(content, p.shell, 1.0);

        if let Some(panel) = active_panel {
            let mut panel_host = div().absolute().top_0().right_0().bottom_0().left_0();
            panel_host = content_policy.apply(panel_host, p.shell, 1.0);
            content = content.child(panel_host.child(panel.render_panel(window, cx)));
        }

        root.child(content)
    }
}

pub struct DockArea {
    id: SharedString,
    version: Option<usize>,
    center: DockItem,
    left: Option<(DockItem, f32, bool)>,
    right: Option<(DockItem, f32, bool)>,
    bottom: Option<(DockItem, f32, bool)>,
    zoomed_panel: Option<SharedString>,
    background_policy: MoonBackgroundPolicy,
    tab_background_policy: MoonBackgroundPolicy,
    content_background_policy: Option<MoonBackgroundPolicy>,
    root_bounds: Bounds<Pixels>,
    row_bounds: Bounds<Pixels>,
    split_bounds: HashMap<String, Bounds<Pixels>>,
    tile_bounds: HashMap<String, Bounds<Pixels>>,
    tile_drag_start: Option<DockTileDragStart>,
    /// When false, slots do not expose split drop-zones — dragging a tab can only reorder
    /// it within a tab strip or move it into another existing tab strip, not create a new
    /// split anywhere (which lets panels land in e.g. a chart slot and wedge the layout).
    enable_split_drop: bool,
}

impl EventEmitter<DockEvent> for DockArea {}

impl DockArea {
    pub fn new(
        id: impl Into<SharedString>,
        version: Option<usize>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Self {
        Self {
            id: id.into(),
            version,
            center: DockItem::Empty,
            left: None,
            right: None,
            bottom: None,
            zoomed_panel: None,
            background_policy: MoonBackgroundPolicy::Opaque,
            tab_background_policy: MoonBackgroundPolicy::Opaque,
            content_background_policy: None,
            root_bounds: Bounds::default(),
            row_bounds: Bounds::default(),
            split_bounds: HashMap::new(),
            tile_bounds: HashMap::new(),
            tile_drag_start: None,
            enable_split_drop: true,
        }
    }

    #[cfg(test)]
    fn test_with_center(center: DockItem) -> Self {
        Self {
            id: "test-dock".into(),
            version: None,
            center,
            left: None,
            right: None,
            bottom: None,
            zoomed_panel: None,
            background_policy: MoonBackgroundPolicy::Opaque,
            tab_background_policy: MoonBackgroundPolicy::Opaque,
            content_background_policy: None,
            root_bounds: Bounds::default(),
            row_bounds: Bounds::default(),
            split_bounds: HashMap::new(),
            tile_bounds: HashMap::new(),
            tile_drag_start: None,
            enable_split_drop: true,
        }
    }

    pub fn from_state(
        id: impl Into<SharedString>,
        state: DockAreaState,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let center = state.center.to_item(window, cx);
        let left = state
            .left_dock
            .map(|dock| (dock.panel.to_item(window, cx), dock.size, dock.open));
        let right = state
            .right_dock
            .map(|dock| (dock.panel.to_item(window, cx), dock.size, dock.open));
        let bottom = state
            .bottom_dock
            .map(|dock| (dock.panel.to_item(window, cx), dock.size, dock.open));

        Self {
            id: id.into(),
            version: state.version,
            center,
            left,
            right,
            bottom,
            zoomed_panel: None,
            background_policy: MoonBackgroundPolicy::Opaque,
            tab_background_policy: MoonBackgroundPolicy::Opaque,
            content_background_policy: None,
            root_bounds: Bounds::default(),
            row_bounds: Bounds::default(),
            split_bounds: HashMap::new(),
            tile_bounds: HashMap::new(),
            tile_drag_start: None,
            enable_split_drop: true,
        }
    }

    pub fn load(
        &mut self,
        state: DockAreaState,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        self.version = state.version;
        self.center = state.center.to_item(window, cx);
        self.left = state
            .left_dock
            .map(|dock| (dock.panel.to_item(window, cx), dock.size, dock.open));
        self.right = state
            .right_dock
            .map(|dock| (dock.panel.to_item(window, cx), dock.size, dock.open));
        self.bottom = state
            .bottom_dock
            .map(|dock| (dock.panel.to_item(window, cx), dock.size, dock.open));
        self.zoomed_panel = None;

        let dock_area = cx.entity().downgrade();
        self.center
            .notify_added(&dock_area, window, cx.borrow_mut());
        if let Some((item, _, _)) = &self.left {
            item.notify_added(&dock_area, window, cx.borrow_mut());
        }
        if let Some((item, _, _)) = &self.right {
            item.notify_added(&dock_area, window, cx.borrow_mut());
        }
        if let Some((item, _, _)) = &self.bottom {
            item.notify_added(&dock_area, window, cx.borrow_mut());
        }
        cx.notify();
        Ok(())
    }

    pub fn background_policy(mut self, policy: MoonBackgroundPolicy) -> Self {
        self.background_policy = policy;
        self
    }

    /// Disable split drop-zones (default enabled). With this off, dragging a tab can only
    /// reorder within / move between existing tab strips — it cannot create a new split
    /// (so panels can't be dropped into a chart slot and wedge the layout).
    pub fn enable_split_drop(mut self, enable: bool) -> Self {
        self.enable_split_drop = enable;
        self
    }

    pub fn tab_background_policy(mut self, policy: MoonBackgroundPolicy) -> Self {
        self.tab_background_policy = policy;
        self
    }

    pub fn content_background_policy(mut self, policy: MoonBackgroundPolicy) -> Self {
        self.content_background_policy = Some(policy);
        self
    }

    pub fn set_center(&mut self, item: DockItem, window: &mut Window, cx: &mut Context<Self>) {
        item.notify_added(&cx.entity().downgrade(), window, cx.borrow_mut());
        self.center = item;
        cx.emit(DockEvent::LayoutChanged);
        cx.notify();
    }

    pub fn add_panel(
        &mut self,
        panel: Rc<dyn PanelView>,
        placement: DockPlacement,
        size: Option<f32>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        panel.on_added_to(cx.entity().downgrade(), window, cx.borrow_mut());
        let item = DockItem::panel(panel.clone());
        match placement {
            DockPlacement::Center => {
                self.center =
                    std::mem::replace(&mut self.center, DockItem::Empty).with_panel_added(panel);
            }
            DockPlacement::Left => {
                let item = if let Some((existing, _, _)) = self.left.take() {
                    existing.with_panel_added(panel)
                } else {
                    item
                };
                self.left = Some((item, size.unwrap_or(220.0), true));
            }
            DockPlacement::Right => {
                let item = if let Some((existing, _, _)) = self.right.take() {
                    existing.with_panel_added(panel)
                } else {
                    item
                };
                self.right = Some((item, size.unwrap_or(280.0), true));
            }
            DockPlacement::Bottom => {
                let item = if let Some((existing, _, _)) = self.bottom.take() {
                    existing.with_panel_added(panel)
                } else {
                    item
                };
                self.bottom = Some((item, size.unwrap_or(160.0), true));
            }
        }
        cx.emit(DockEvent::LayoutChanged);
        cx.notify();
    }

    /// Restore `panel` into its "home" tab strip — the `Tabs` node inside `center` that
    /// already holds one of `sibling_names` — at `ix` (clamped). Unlike `add_panel(Center)`,
    /// this does NOT collapse the surrounding split. Returns false if no such strip exists
    /// (e.g. every sibling is detached); the caller may then fall back to `add_panel`.
    pub fn insert_panel_into_home_tabs(
        &mut self,
        panel: Rc<dyn PanelView>,
        ix: usize,
        sibling_names: &[&str],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let weak = cx.entity().downgrade();
        let ok = self.center.insert_into_named_tabs(
            panel,
            ix,
            sibling_names,
            &weak,
            window,
            cx.borrow_mut(),
        );
        if ok {
            cx.emit(DockEvent::LayoutChanged);
            cx.notify();
        }
        ok
    }

    /// Restore `panel` back BESIDE its former split neighbours (side-by-side / stacked), instead
    /// of merging it into a tab strip. Handles arbitrary NESTED splits (rows of columns etc.).
    ///
    /// `sibling_names` — every panel that shared the returning panel's immediate parent split (any
    /// present one anchors that split). `slot_panels` — the panels of the ADJACENT slot the panel
    /// sat next to (that slot may itself be a nested split). `index`/`placement` — the panel's
    /// former position & side; `panel_size`/`sibling_size` — the pre-detach pixel slot sizes
    /// (`None` = flex) so it returns at its old proportion.
    ///
    /// Two outcomes:
    /// - the parent split of the SAME orientation still exists (panel was one of 3+ members) →
    ///   insert `panel` as a new member at `index`, in place;
    /// - that split collapsed (panel was one of two) → wrap the WHOLE adjacent slot subtree (the
    ///   smallest node holding all present `slot_panels`, so a nested column-stack is wrapped as a
    ///   unit, not one leaf inside it) into a fresh split with `panel` on `placement`.
    ///
    /// Returns false if neither anchor nor slot survives — the caller falls back to tab restore.
    pub fn insert_panel_beside_sibling(
        &mut self,
        panel: Rc<dyn PanelView>,
        sibling_names: &[&str],
        slot_panels: &[&str],
        index: usize,
        placement: DockSplitPlacement,
        panel_size: Option<f32>,
        sibling_size: Option<f32>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        // Orientation implied by the target side: Left/Right → horizontal split, Top/Bottom → vertical.
        let want_horizontal = matches!(
            placement,
            DockSplitPlacement::Left | DockSplitPlacement::Right
        );

        // Case 1: an anchor sibling still sits inside a Split of the SAME orientation — that split
        // survived (3+ members). Insert `panel` as a new member at `index`, in place.
        let mut anchor: Option<(DockRoot, Vec<usize>)> = None;
        'roots: for root in [
            DockRoot::Bottom,
            DockRoot::Center,
            DockRoot::Left,
            DockRoot::Right,
        ] {
            if let Some(item) = self.root_item_mut(root) {
                for name in sibling_names {
                    if let Some(path) = item.find_panel_path(name, cx) {
                        anchor = Some((root, path));
                        break 'roots;
                    }
                }
            }
        }
        if let Some((root, path)) = &anchor {
            if let Some((_, parent_path)) = path.split_last() {
                let parent_matches = self
                    .root_item_mut(*root)
                    .and_then(|it| Self::item_at_path_mut(it, parent_path))
                    .map(|p| matches!(p, DockItem::Split { horizontal, .. } if *horizontal == want_horizontal))
                    .unwrap_or(false);
                if parent_matches {
                    let weak = cx.entity().downgrade();
                    panel.on_added_to(weak, window, cx);
                    if let Some(DockItem::Split { items, sizes, .. }) = self
                        .root_item_mut(*root)
                        .and_then(|it| Self::item_at_path_mut(it, parent_path))
                    {
                        let ix = index.min(items.len());
                        items.insert(ix, DockItem::Panel(panel));
                        // Keep `sizes` aligned with `items`; give the returning slot its former size.
                        while sizes.len() + 1 < items.len() {
                            sizes.push(None);
                        }
                        sizes.insert(ix.min(sizes.len()), panel_size);
                    }
                    cx.emit(DockEvent::LayoutChanged);
                    cx.notify();
                    return true;
                }
            }
        }

        // Case 2: wrap the whole adjacent slot subtree (smallest node holding all present
        // `slot_panels`) into a fresh split with `panel` on `placement`.
        let mut wrap: Option<(DockRoot, Vec<usize>)> = None;
        for root in [
            DockRoot::Bottom,
            DockRoot::Center,
            DockRoot::Left,
            DockRoot::Right,
        ] {
            if let Some(item) = self.root_item_mut(root) {
                if let Some(path) = item.smallest_subtree_with_all(slot_panels, cx) {
                    wrap = Some((root, path));
                    break;
                }
            }
        }
        let Some((root, path)) = wrap else {
            return false;
        };
        let weak = cx.entity().downgrade();
        panel.on_added_to(weak, window, cx);
        let ok = self.split_item_with_panel(root, &path, placement, panel);
        if ok {
            // `split_item_with_panel` orders the new panel first for Left/Top, second for
            // Right/Bottom, with even (empty) sizes. Overwrite with the remembered proportions
            // in that same order so the returning panel reclaims its former slot size.
            if panel_size.is_some() || sibling_size.is_some() {
                if let Some(DockItem::Split { sizes, .. }) = self
                    .root_item_mut(root)
                    .and_then(|it| Self::item_at_path_mut(it, &path))
                {
                    let panel_first = matches!(
                        placement,
                        DockSplitPlacement::Left | DockSplitPlacement::Top
                    );
                    *sizes = if panel_first {
                        vec![panel_size, sibling_size]
                    } else {
                        vec![sibling_size, panel_size]
                    };
                }
            }
            cx.emit(DockEvent::LayoutChanged);
            cx.notify();
        }
        ok
    }

    pub fn remove_panel_by_name(
        &mut self,
        panel_name: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let (center, removed_center) = std::mem::replace(&mut self.center, DockItem::Empty)
            .remove_panel_named(panel_name, window, cx.borrow_mut());
        self.center = center;

        let mut removed = removed_center;
        self.left = self.left.take().and_then(|(item, size, open)| {
            let (item, did_remove) = item.remove_panel_named(panel_name, window, cx.borrow_mut());
            removed |= did_remove;
            (!item.is_empty()).then_some((item, size, open))
        });
        self.right = self.right.take().and_then(|(item, size, open)| {
            let (item, did_remove) = item.remove_panel_named(panel_name, window, cx.borrow_mut());
            removed |= did_remove;
            (!item.is_empty()).then_some((item, size, open))
        });
        self.bottom = self.bottom.take().and_then(|(item, size, open)| {
            let (item, did_remove) = item.remove_panel_named(panel_name, window, cx.borrow_mut());
            removed |= did_remove;
            (!item.is_empty()).then_some((item, size, open))
        });

        if removed {
            if self.zoomed_panel.as_ref().map(|name| name.as_ref()) == Some(panel_name) {
                self.zoomed_panel = None;
            }
            cx.emit(DockEvent::LayoutChanged);
            cx.notify();
        }
        removed
    }

    pub fn set_dock_open(
        &mut self,
        placement: DockPlacement,
        open: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        let slot = match placement {
            DockPlacement::Center => return false,
            DockPlacement::Left => self.left.as_mut(),
            DockPlacement::Right => self.right.as_mut(),
            DockPlacement::Bottom => self.bottom.as_mut(),
        };
        let Some((_, _, current_open)) = slot else {
            return false;
        };
        if *current_open == open {
            return false;
        }
        *current_open = open;
        cx.emit(DockEvent::LayoutChanged);
        cx.notify();
        true
    }

    pub fn toggle_dock(&mut self, placement: DockPlacement, cx: &mut Context<Self>) -> bool {
        let open = match placement {
            DockPlacement::Center => return false,
            DockPlacement::Left => self.left.as_ref().map(|(_, _, open)| !*open),
            DockPlacement::Right => self.right.as_ref().map(|(_, _, open)| !*open),
            DockPlacement::Bottom => self.bottom.as_ref().map(|(_, _, open)| !*open),
        };
        open.map(|open| self.set_dock_open(placement, open, cx))
            .unwrap_or(false)
    }

    pub fn toggle_zoom_panel(
        &mut self,
        panel_name: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let panel_name = panel_name.into();
        let next = if self.zoomed_panel.as_ref() == Some(&panel_name) {
            None
        } else {
            Some(panel_name.clone())
        };
        if let Some(current) = self.zoomed_panel.as_ref() {
            if let Some(panel) = self.find_panel_named(current.as_ref(), cx) {
                panel.set_zoomed(false, window, cx.borrow_mut());
            }
        }
        if let Some(next_name) = next.as_ref() {
            if let Some(panel) = self.find_panel_named(next_name.as_ref(), cx) {
                panel.set_zoomed(true, window, cx.borrow_mut());
            }
        }
        self.zoomed_panel = next;
        cx.emit(DockEvent::LayoutChanged);
        cx.notify();
    }

    pub fn clear_zoom(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(current) = self.zoomed_panel.take() {
            if let Some(panel) = self.find_panel_named(current.as_ref(), cx) {
                panel.set_zoomed(false, window, cx.borrow_mut());
            }
            cx.emit(DockEvent::LayoutChanged);
            cx.notify();
        }
    }

    pub fn dump(&self, cx: &App) -> DockAreaState {
        DockAreaState {
            version: self.version,
            center: self.center.dump(cx),
            left_dock: self.left.as_ref().map(|(panel, size, open)| DockState {
                panel: panel.dump(cx),
                placement: DockPlacement::Left,
                size: *size,
                open: *open,
            }),
            right_dock: self.right.as_ref().map(|(panel, size, open)| DockState {
                panel: panel.dump(cx),
                placement: DockPlacement::Right,
                size: *size,
                open: *open,
            }),
            bottom_dock: self.bottom.as_ref().map(|(panel, size, open)| DockState {
                panel: panel.dump(cx),
                placement: DockPlacement::Bottom,
                size: *size,
                open: *open,
            }),
        }
    }

    fn split_key(root: DockRoot, path: &[usize]) -> String {
        let root = match root {
            DockRoot::Center => "center",
            DockRoot::Left => "left",
            DockRoot::Right => "right",
            DockRoot::Bottom => "bottom",
        };
        if path.is_empty() {
            root.to_string()
        } else {
            format!(
                "{}:{}",
                root,
                path.iter()
                    .map(|ix| ix.to_string())
                    .collect::<Vec<_>>()
                    .join(".")
            )
        }
    }

    fn resize_handle(
        &self,
        id: SharedString,
        horizontal_split: bool,
        target: DockResizeTarget,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = MoonPalette::active(cx);
        let drag = DockResizeDrag {
            dock_id: cx.entity_id(),
            target,
        };

        div()
            .id(ElementId::from(id))
            .flex_none()
            .relative()
            .bg(rgba_from(p.shell, 1.0))
            .occlude()
            .cursor(if horizontal_split {
                CursorStyle::ResizeColumn
            } else {
                CursorStyle::ResizeRow
            })
            .when(horizontal_split, |this| {
                this.w(px(DOCK_RESIZE_HIT_SIZE))
                    .h_full()
                    .items_center()
                    .justify_center()
                    .child(div().w(px(1.0)).h_full().bg(rgba_from(p.border, 1.0)))
            })
            .when(!horizontal_split, |this| {
                this.h(px(DOCK_RESIZE_HIT_SIZE))
                    .w_full()
                    .items_center()
                    .justify_center()
                    .child(div().h(px(1.0)).w_full().bg(rgba_from(p.border, 1.0)))
            })
            .hover(|style| style.bg(rgba_from(p.shell_high, 1.0)))
            .on_drag(drag, |drag, _, _, cx| {
                cx.stop_propagation();
                cx.new(|_| drag.clone())
            })
            .on_drag_move(cx.listener(Self::on_resize_drag_move))
            .into_any_element()
    }

    fn root_item_mut(&mut self, root: DockRoot) -> Option<&mut DockItem> {
        match root {
            DockRoot::Center => Some(&mut self.center),
            DockRoot::Left => self.left.as_mut().map(|(item, _, _)| item),
            DockRoot::Right => self.right.as_mut().map(|(item, _, _)| item),
            DockRoot::Bottom => self.bottom.as_mut().map(|(item, _, _)| item),
        }
    }

    fn item_at_path_mut<'a>(item: &'a mut DockItem, path: &[usize]) -> Option<&'a mut DockItem> {
        let mut current = item;
        for ix in path {
            let DockItem::Split { items, .. } = current else {
                return None;
            };
            current = items.get_mut(*ix)?;
        }
        Some(current)
    }

    fn find_panel_named(&self, panel_name: &str, cx: &App) -> Option<Rc<dyn PanelView>> {
        self.center
            .find_panel_named(panel_name, cx)
            .or_else(|| {
                self.left
                    .as_ref()
                    .and_then(|(item, _, _)| item.find_panel_named(panel_name, cx))
            })
            .or_else(|| {
                self.right
                    .as_ref()
                    .and_then(|(item, _, _)| item.find_panel_named(panel_name, cx))
            })
            .or_else(|| {
                self.bottom
                    .as_ref()
                    .and_then(|(item, _, _)| item.find_panel_named(panel_name, cx))
            })
    }

    fn set_tabs_active_index(&mut self, root: DockRoot, path: &[usize], active_ix: usize) -> bool {
        let Some(DockItem::Tabs {
            items,
            active_ix: current,
        }) = self
            .root_item_mut(root)
            .and_then(|item| Self::item_at_path_mut(item, path))
        else {
            return false;
        };
        let active_ix = active_ix.min(items.len().saturating_sub(1));
        if *current == active_ix {
            return false;
        }
        *current = active_ix;
        true
    }

    fn move_tab_before(
        &mut self,
        root: DockRoot,
        path: &[usize],
        panel_name: &str,
        target_ix: usize,
        cx: &App,
    ) -> bool {
        let Some(DockItem::Tabs { items, active_ix }) = self
            .root_item_mut(root)
            .and_then(|item| Self::item_at_path_mut(item, path))
        else {
            return false;
        };
        let Some(from_ix) = items
            .iter()
            .position(|panel| panel.panel_name(cx).to_string() == panel_name)
        else {
            return false;
        };
        let panel = items.remove(from_ix);
        let target_ix = if from_ix < target_ix {
            target_ix.saturating_sub(1)
        } else {
            target_ix
        }
        .min(items.len());
        items.insert(target_ix, panel);
        *active_ix = target_ix;
        true
    }

    fn take_panel_named_for_move(
        &mut self,
        panel_name: &str,
        cx: &App,
    ) -> Option<Rc<dyn PanelView>> {
        let (center, taken) =
            std::mem::replace(&mut self.center, DockItem::Empty).take_panel_named(panel_name, cx);
        self.center = center;
        if taken.is_some() {
            return taken;
        }

        if let Some((item, size, open)) = self.left.take() {
            let (item, taken) = item.take_panel_named(panel_name, cx);
            if !item.is_empty() {
                self.left = Some((item, size, open));
            }
            if taken.is_some() {
                return taken;
            }
        }
        if let Some((item, size, open)) = self.right.take() {
            let (item, taken) = item.take_panel_named(panel_name, cx);
            if !item.is_empty() {
                self.right = Some((item, size, open));
            }
            if taken.is_some() {
                return taken;
            }
        }
        if let Some((item, size, open)) = self.bottom.take() {
            let (item, taken) = item.take_panel_named(panel_name, cx);
            if !item.is_empty() {
                self.bottom = Some((item, size, open));
            }
            if taken.is_some() {
                return taken;
            }
        }

        None
    }

    fn insert_panel_into_tabs(
        &mut self,
        root: DockRoot,
        path: &[usize],
        target_ix: usize,
        panel: Rc<dyn PanelView>,
    ) -> bool {
        let Some(item) = self
            .root_item_mut(root)
            .and_then(|item| Self::item_at_path_mut(item, path))
        else {
            return false;
        };

        match item {
            DockItem::Empty => {
                *item = DockItem::Panel(panel);
                true
            }
            DockItem::Panel(existing) => {
                let existing = existing.clone();
                *item = DockItem::Tabs {
                    items: vec![existing, panel],
                    active_ix: 1,
                };
                true
            }
            DockItem::Tabs { items, active_ix } => {
                let target_ix = target_ix.min(items.len());
                items.insert(target_ix, panel);
                *active_ix = target_ix;
                true
            }
            DockItem::Split { .. } | DockItem::Tiles { .. } => false,
        }
    }

    fn move_panel_to_tabs(
        &mut self,
        panel_name: &str,
        root: DockRoot,
        path: &[usize],
        target_ix: usize,
        cx: &App,
    ) -> bool {
        let anchor = self
            .root_item_mut(root)
            .and_then(|it| Self::item_at_path_mut(it, path))
            .and_then(|it| it.first_panel_name_excluding(panel_name, cx));
        if anchor.is_none() {
            return false;
        }
        let Some(panel) = self.take_panel_named_for_move(panel_name, cx) else {
            return false;
        };
        let target_path = anchor
            .as_deref()
            .and_then(|a| {
                self.root_item_mut(root)
                    .and_then(|it| it.find_panel_path(a, cx))
            })
            .unwrap_or_else(|| path.to_vec());
        if self.insert_panel_into_tabs(root, &target_path, target_ix, panel.clone()) {
            true
        } else {
            let pushed = self
                .root_item_mut(root)
                .map(|it| it.try_push_into_first_tabs(panel.clone()))
                .unwrap_or(false);
            if !pushed {
                let existing = std::mem::replace(&mut self.center, DockItem::Empty);
                self.center = DockItem::Split {
                    horizontal: false,
                    items: vec![existing, DockItem::Panel(panel)],
                    sizes: Vec::new(),
                };
            }
            false
        }
    }

    fn split_item_with_panel(
        &mut self,
        root: DockRoot,
        path: &[usize],
        placement: DockSplitPlacement,
        panel: Rc<dyn PanelView>,
    ) -> bool {
        let Some(item) = self
            .root_item_mut(root)
            .and_then(|item| Self::item_at_path_mut(item, path))
        else {
            return false;
        };
        let existing = std::mem::replace(item, DockItem::Empty);
        if existing.is_empty() {
            *item = DockItem::Panel(panel);
            return true;
        }
        let new_panel = DockItem::Panel(panel);
        let (horizontal, items) = match placement {
            DockSplitPlacement::Left => (true, vec![new_panel, existing]),
            DockSplitPlacement::Right => (true, vec![existing, new_panel]),
            DockSplitPlacement::Top => (false, vec![new_panel, existing]),
            DockSplitPlacement::Bottom => (false, vec![existing, new_panel]),
        };
        *item = DockItem::Split {
            horizontal,
            items,
            sizes: Vec::new(),
        };
        true
    }

    fn move_panel_to_split(
        &mut self,
        panel_name: &str,
        root: DockRoot,
        path: &[usize],
        placement: DockSplitPlacement,
        cx: &App,
    ) -> bool {
        // «Якорь» целевого слота — имя соседней панели в нём (до take). take может схлопнуть
        // узел и сдвинуть path; по якорю находим целевой слот заново уже после take.
        let anchor = self
            .root_item_mut(root)
            .and_then(|it| Self::item_at_path_mut(it, path))
            .and_then(|it| it.first_panel_name_excluding(panel_name, cx));
        // anchor=None → целевой слот содержит ТОЛЬКО саму перетаскиваемую панель (дроп на
        // себя): split бессмыслен. Отменяем ДО take, чтобы не остаться без якоря и не
        // разрушить раскладку. Панель остаётся на месте.
        if anchor.is_none() {
            return false;
        }
        let Some(panel) = self.take_panel_named_for_move(panel_name, cx) else {
            return false;
        };
        // Пересчитываем путь к целевому слоту по якорю (path мог устареть после take).
        let target_path = anchor
            .as_deref()
            .and_then(|a| {
                self.root_item_mut(root)
                    .and_then(|it| it.find_panel_path(a, cx))
            })
            .unwrap_or_else(|| path.to_vec());
        if self.split_item_with_panel(root, &target_path, placement, panel.clone()) {
            true
        } else {
            // НЕ разрушаем центр (старый with_panel_added схлопывал всё в Tabs[panel]):
            // возвращаем панель вкладкой в первый Tabs.
            let pushed = self
                .root_item_mut(root)
                .map(|it| it.try_push_into_first_tabs(panel.clone()))
                .unwrap_or(false);
            if !pushed {
                // Нет ни одного Tabs — добавляем панель отдельным слотом снизу, НЕ заменяя
                // центр (with_panel_added схлопывал всё в Tabs[panel] → на весь экран).
                let existing = std::mem::replace(&mut self.center, DockItem::Empty);
                self.center = DockItem::Split {
                    horizontal: false,
                    items: vec![existing, DockItem::Panel(panel)],
                    sizes: Vec::new(),
                };
            }
            false
        }
    }

    fn set_changed_size(slot: &mut f32, size: f32) -> bool {
        if (*slot - size).abs() <= 0.5 {
            return false;
        }
        *slot = size;
        true
    }

    fn snap_tile_value(value: f32) -> f32 {
        (value / DOCK_TILE_SNAP).round() * DOCK_TILE_SNAP
    }

    fn clamp_tile_meta(mut meta: TileMeta, bounds: Bounds<Pixels>) -> TileMeta {
        let max_w = f32::from(bounds.size.width).max(DOCK_TILE_MIN_W);
        let max_h = f32::from(bounds.size.height).max(DOCK_TILE_MIN_H);
        meta.w = Self::snap_tile_value(meta.w).clamp(DOCK_TILE_MIN_W, max_w);
        meta.h = Self::snap_tile_value(meta.h).clamp(DOCK_TILE_MIN_H, max_h);
        meta.x = Self::snap_tile_value(meta.x).clamp(0.0, (max_w - meta.w).max(0.0));
        meta.y = Self::snap_tile_value(meta.y).clamp(0.0, (max_h - meta.h).max(0.0));
        meta
    }

    fn tile_key(root: DockRoot, path: &[usize]) -> String {
        format!("tiles:{}", Self::split_key(root, path))
    }

    fn update_tile_meta(
        &mut self,
        root: DockRoot,
        path: &[usize],
        ix: usize,
        meta: TileMeta,
    ) -> bool {
        let Some(DockItem::Tiles { metas, .. }) = self
            .root_item_mut(root)
            .and_then(|item| Self::item_at_path_mut(item, path))
        else {
            return false;
        };

        if metas.len() <= ix {
            metas.resize(
                ix + 1,
                TileMeta {
                    x: 12.0,
                    y: 12.0,
                    w: 320.0,
                    h: 200.0,
                    z_index: 0,
                },
            );
        }

        if metas[ix] == meta {
            return false;
        }
        metas[ix] = meta;
        true
    }

    fn start_tile_drag(
        &mut self,
        root: DockRoot,
        path: Vec<usize>,
        ix: usize,
        cursor: Point<Pixels>,
        meta: TileMeta,
    ) {
        self.tile_drag_start = Some(DockTileDragStart {
            root,
            path,
            ix,
            cursor,
            meta,
        });
    }

    fn clear_tile_drag(
        &mut self,
        _event: &MouseUpEvent,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        self.tile_drag_start = None;
    }

    fn on_tile_drag_move(
        &mut self,
        event: &DragMoveEvent<DockTileDrag>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let drag = event.drag(cx);
        if drag.dock_id != cx.entity_id() {
            return;
        }
        let Some(start) = self.tile_drag_start.as_ref() else {
            return;
        };
        if start.root != drag.root || start.path != drag.path || start.ix != drag.ix {
            return;
        }

        let dx = f32::from(event.event.position.x - start.cursor.x);
        let dy = f32::from(event.event.position.y - start.cursor.y);
        let mut meta = start.meta;
        match drag.kind {
            DockTileDragKind::Move => {
                meta.x += dx;
                meta.y += dy;
            }
            DockTileDragKind::ResizeRight => {
                meta.w += dx;
            }
            DockTileDragKind::ResizeBottom => {
                meta.h += dy;
            }
            DockTileDragKind::ResizeBottomRight => {
                meta.w += dx;
                meta.h += dy;
            }
        }

        let bounds = self
            .tile_bounds
            .get(&Self::tile_key(drag.root, &drag.path))
            .copied()
            .unwrap_or(self.root_bounds);
        meta = Self::clamp_tile_meta(meta, bounds);
        if self.update_tile_meta(drag.root, &drag.path, drag.ix, meta) {
            cx.emit(DockEvent::LayoutChanged);
            cx.notify();
        }
    }

    fn split_drop_zone(
        &self,
        id: SharedString,
        root: DockRoot,
        path: Vec<usize>,
        placement: DockSplitPlacement,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let dock = cx.entity();
        let dock_id = cx.entity_id();
        let tokens = MoonTheme::active_tokens(cx);
        let mut zone = div()
            .id(ElementId::from(id))
            .absolute()
            .when(matches!(placement, DockSplitPlacement::Left), |this| {
                this.left(px(0.0))
                    .top(px(0.0))
                    .bottom(px(0.0))
                    .w(px(tokens.ui(42.0)))
                    .border_l(px(tokens.ui(2.0)))
            })
            .when(matches!(placement, DockSplitPlacement::Right), |this| {
                this.right(px(0.0))
                    .top(px(0.0))
                    .bottom(px(0.0))
                    .w(px(tokens.ui(42.0)))
                    .border_r(px(tokens.ui(2.0)))
            })
            .when(matches!(placement, DockSplitPlacement::Top), |this| {
                this.top(px(0.0))
                    .left(px(tokens.ui(42.0)))
                    .right(px(tokens.ui(42.0)))
                    .h(px(tokens.ui(34.0)))
                    .border_t(px(tokens.ui(2.0)))
            })
            .when(matches!(placement, DockSplitPlacement::Bottom), |this| {
                this.bottom(px(0.0))
                    .left(px(tokens.ui(42.0)))
                    .right(px(tokens.ui(42.0)))
                    .h(px(tokens.ui(34.0)))
                    .border_b(px(tokens.ui(2.0)))
            })
            .drag_over::<DockTabDrag>(move |style, drag, _, cx| {
                let p = MoonPalette::active(cx);
                if drag.dock_id == dock_id && drag.splittable {
                    style
                        .bg(rgba_from(p.accent, p.accent_tint_a))
                        .border_color(rgba_from(p.accent, 0.88))
                } else {
                    style
                }
            });

        zone = zone.on_drop(move |drag: &DockTabDrag, _window, cx| {
            if drag.dock_id != dock_id || !drag.splittable {
                return;
            }
            _ = dock.update(cx, |dock, cx| {
                if dock.move_panel_to_split(drag.panel_name.as_ref(), root, &path, placement, cx) {
                    cx.emit(DockEvent::LayoutChanged);
                    cx.notify();
                }
            });
        });

        zone.into_any_element()
    }

    fn add_split_drop_zones(
        &self,
        mut host: Div,
        id_text: &str,
        root: DockRoot,
        path: Vec<usize>,
        target_splittable: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        // Slot accepts split drops only if it is itself splittable (a bottom dock panel),
        // so a chart/detect slot never gets split zones.
        if !self.enable_split_drop || !target_splittable {
            return host;
        }
        host = host
            .child(self.split_drop_zone(
                SharedString::from(format!("{id_text}:drop-left")),
                root,
                path.clone(),
                DockSplitPlacement::Left,
                cx,
            ))
            .child(self.split_drop_zone(
                SharedString::from(format!("{id_text}:drop-right")),
                root,
                path.clone(),
                DockSplitPlacement::Right,
                cx,
            ))
            .child(self.split_drop_zone(
                SharedString::from(format!("{id_text}:drop-top")),
                root,
                path.clone(),
                DockSplitPlacement::Top,
                cx,
            ))
            .child(self.split_drop_zone(
                SharedString::from(format!("{id_text}:drop-bottom")),
                root,
                path,
                DockSplitPlacement::Bottom,
                cx,
            ));
        host
    }

    fn resize_outer_left(&mut self, position: Point<Pixels>) -> bool {
        let row_w = f32::from(self.row_bounds.size.width);
        if row_w <= 1.0 {
            return false;
        }
        let right_w = self
            .right
            .as_ref()
            .filter(|(_, _, open)| *open)
            .map(|(_, size, _)| *size)
            .unwrap_or(0.0);
        let max = (row_w - right_w - DOCK_MIN_CENTER_SIZE).max(DOCK_MIN_SIDE_SIZE);
        let size = (f32::from(position.x) - f32::from(self.row_bounds.origin.x))
            .clamp(DOCK_MIN_SIDE_SIZE, max);
        self.left
            .as_mut()
            .map(|(_, current, _)| Self::set_changed_size(current, size))
            .unwrap_or(false)
    }

    fn resize_outer_right(&mut self, position: Point<Pixels>) -> bool {
        let row_w = f32::from(self.row_bounds.size.width);
        if row_w <= 1.0 {
            return false;
        }
        let left_w = self
            .left
            .as_ref()
            .filter(|(_, _, open)| *open)
            .map(|(_, size, _)| *size)
            .unwrap_or(0.0);
        let max = (row_w - left_w - DOCK_MIN_CENTER_SIZE).max(DOCK_MIN_SIDE_SIZE);
        let row_right = f32::from(self.row_bounds.origin.x) + row_w;
        let size = (row_right - f32::from(position.x)).clamp(DOCK_MIN_SIDE_SIZE, max);
        self.right
            .as_mut()
            .map(|(_, current, _)| Self::set_changed_size(current, size))
            .unwrap_or(false)
    }

    fn resize_outer_bottom(&mut self, position: Point<Pixels>) -> bool {
        let root_h = f32::from(self.root_bounds.size.height);
        if root_h <= 1.0 {
            return false;
        }
        let root_bottom = f32::from(self.root_bounds.origin.y) + root_h;
        let max = (root_h - DOCK_MIN_CENTER_SIZE).max(DOCK_MIN_BOTTOM_SIZE);
        let size = (root_bottom - f32::from(position.y)).clamp(DOCK_MIN_BOTTOM_SIZE, max);
        self.bottom
            .as_mut()
            .map(|(_, current, _)| Self::set_changed_size(current, size))
            .unwrap_or(false)
    }

    fn resize_split(
        &mut self,
        root: DockRoot,
        path: &[usize],
        after_ix: usize,
        position: Point<Pixels>,
    ) -> bool {
        let key = Self::split_key(root, path);
        let Some(bounds) = self.split_bounds.get(&key).cloned() else {
            return false;
        };
        let Some(DockItem::Split {
            horizontal,
            items,
            sizes,
        }) = self
            .root_item_mut(root)
            .and_then(|item| Self::item_at_path_mut(item, path))
        else {
            return false;
        };
        if after_ix == 0 || after_ix >= items.len() {
            return false;
        }

        if sizes.len() < items.len() {
            sizes.resize(items.len(), None);
        }

        let total = if *horizontal {
            f32::from(bounds.size.width)
        } else {
            f32::from(bounds.size.height)
        };
        if total <= 1.0 {
            return false;
        }
        if total <= DOCK_MIN_SIDE_SIZE * 2.0 {
            return false;
        }

        let local = if *horizontal {
            f32::from(position.x) - f32::from(bounds.origin.x)
        } else {
            f32::from(position.y) - f32::from(bounds.origin.y)
        }
        .clamp(DOCK_MIN_SIDE_SIZE, total - DOCK_MIN_SIDE_SIZE);

        let target_ix = if after_ix + 1 == items.len() {
            after_ix
        } else {
            after_ix - 1
        };
        let size = if target_ix == after_ix {
            (total - local).clamp(DOCK_MIN_SIDE_SIZE, total - DOCK_MIN_SIDE_SIZE)
        } else {
            local.clamp(DOCK_MIN_SIDE_SIZE, total - DOCK_MIN_SIDE_SIZE)
        };

        let previous = sizes[target_ix].unwrap_or(-1.0);
        if (previous - size).abs() <= 0.5 {
            return false;
        }
        sizes[target_ix] = Some(size);
        true
    }

    fn on_resize_drag_move(
        &mut self,
        event: &DragMoveEvent<DockResizeDrag>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let drag = event.drag(cx);
        if drag.dock_id != cx.entity_id() {
            return;
        }

        let changed = match &drag.target {
            DockResizeTarget::OuterLeft => self.resize_outer_left(event.event.position),
            DockResizeTarget::OuterRight => self.resize_outer_right(event.event.position),
            DockResizeTarget::OuterBottom => self.resize_outer_bottom(event.event.position),
            DockResizeTarget::Split {
                root,
                path,
                after_ix,
            } => self.resize_split(*root, path, *after_ix, event.event.position),
        };

        if changed {
            cx.emit(DockEvent::LayoutChanged);
            cx.notify();
        }
    }

    fn render_item(
        &self,
        id: SharedString,
        root: DockRoot,
        path: Vec<usize>,
        item: &DockItem,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match item {
            DockItem::Empty => {
                let p = MoonPalette::active(cx);
                div()
                    .size_full()
                    .bg(rgba_from(p.shell, 1.0))
                    .into_any_element()
            }
            DockItem::Panel(panel) => {
                let id_text = id.to_string();
                let want_header = panel.show_dock_header(cx);
                let host = div().relative().size_full().child(
                    TabPanel::new(id, vec![panel.clone()])
                        .dock_context(cx.entity().downgrade(), root, path.clone())
                        .show_header(want_header)
                        .show_panel_controls(want_header)
                        .background_policy(self.tab_background_policy)
                        .content_background_policy(
                            self.content_background_policy
                                .unwrap_or_else(|| panel.background_policy(cx)),
                        ),
                );
                self.add_split_drop_zones(host, &id_text, root, path, want_header, cx)
                    .into_any_element()
            }
            DockItem::Tabs { items, active_ix } => {
                let target_splittable = items.iter().any(|p| p.show_dock_header(cx));
                let id_text = id.to_string();
                let host = div().relative().size_full().child(
                    TabPanel::new(id, items.clone())
                        .dock_context(cx.entity().downgrade(), root, path.clone())
                        .active_index(*active_ix)
                        .background_policy(self.tab_background_policy)
                        .when_some(self.content_background_policy, |this, policy| {
                            this.content_background_policy(policy)
                        }),
                );
                self.add_split_drop_zones(host, &id_text, root, path, target_splittable, cx)
                    .into_any_element()
            }
            DockItem::Tiles { items, metas } => {
                let p = MoonPalette::active(cx);
                let id_text = id.to_string();
                let tile_key = Self::tile_key(root, &path);
                let dock = cx.entity();
                let mut tiles = div()
                    .id(ElementId::from(id.clone()))
                    .relative()
                    .size_full()
                    .overflow_hidden();
                tiles = item.background_policy(cx).apply(tiles, p.shell, 1.0);
                tiles = tiles.child(
                    canvas(
                        {
                            let dock = dock.clone();
                            let tile_key = tile_key.clone();
                            move |bounds, _, cx| {
                                dock.update(cx, |area, _| {
                                    area.tile_bounds.insert(tile_key.clone(), bounds);
                                });
                            }
                        },
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .size_full(),
                );
                let mut ordered = items.iter().enumerate().collect::<Vec<_>>();
                ordered
                    .sort_by_key(|(ix, _)| metas.get(*ix).map(|meta| meta.z_index).unwrap_or(*ix));
                let tokens = MoonTheme::active_tokens(cx);
                for (ix, panel) in ordered {
                    let meta = metas.get(ix).copied().unwrap_or(TileMeta {
                        x: 12.0 + ix as f32 * 18.0,
                        y: 12.0 + ix as f32 * 18.0,
                        w: 320.0,
                        h: 200.0,
                        z_index: ix,
                    });
                    let dock_id = cx.entity_id();
                    let tile_path = path.clone();
                    let move_drag = DockTileDrag {
                        dock_id,
                        root,
                        path: tile_path.clone(),
                        ix,
                        kind: DockTileDragKind::Move,
                    };
                    let right_drag = DockTileDrag {
                        kind: DockTileDragKind::ResizeRight,
                        ..move_drag.clone()
                    };
                    let bottom_drag = DockTileDrag {
                        kind: DockTileDragKind::ResizeBottom,
                        ..move_drag.clone()
                    };
                    let corner_drag = DockTileDrag {
                        kind: DockTileDragKind::ResizeBottomRight,
                        ..move_drag.clone()
                    };

                    let start_tile_drag =
                        |dock: Entity<DockArea>,
                         root: DockRoot,
                         path: Vec<usize>,
                         ix: usize,
                         meta: TileMeta| {
                            move |event: &MouseDownEvent, _window: &mut Window, cx: &mut App| {
                                cx.stop_propagation();
                                dock.update(cx, |area, _| {
                                    area.start_tile_drag(
                                        root,
                                        path.clone(),
                                        ix,
                                        event.position,
                                        meta,
                                    );
                                });
                            }
                        };

                    let mut tile = div()
                        .id(ElementId::from(SharedString::from(format!(
                            "{id_text}:tile:{ix}"
                        ))))
                        .absolute()
                        .left(px(meta.x))
                        .top(px(meta.y))
                        .w(px(meta.w))
                        .h(px(meta.h))
                        .overflow_hidden()
                        .rounded(px(tokens.ui(5.0)))
                        .border(px(1.0))
                        .border_color(rgba_from(p.border, 1.0));
                    tile = panel.background_policy(cx).apply(tile, p.shell_high, 1.0);
                    tiles = tiles.child(
                        tile.child(
                            TabPanel::new(
                                SharedString::from(format!("{id_text}:tile-panel:{ix}")),
                                vec![panel.clone()],
                            )
                            .dock_context(cx.entity().downgrade(), root, {
                                let mut tile_path = path.clone();
                                tile_path.push(ix);
                                tile_path
                            })
                            .background_policy(MoonBackgroundPolicy::NoFill)
                            .content_background_policy(panel.background_policy(cx)),
                        )
                        .child(
                            div()
                                .id(ElementId::from(SharedString::from(format!(
                                    "{id_text}:tile-move:{ix}"
                                ))))
                                .absolute()
                                .left(px(0.0))
                                .top(px(0.0))
                                .right(px(tokens.ui(42.0)))
                                .h(px(tokens.ui(23.0)))
                                .cursor(CursorStyle::OpenHand)
                                .hover(move |style| style.bg(rgba_from(p.overlay, 0.035)))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    start_tile_drag(
                                        dock.clone(),
                                        root,
                                        tile_path.clone(),
                                        ix,
                                        meta,
                                    ),
                                )
                                .on_mouse_up(MouseButton::Left, cx.listener(Self::clear_tile_drag))
                                .on_mouse_up_out(
                                    MouseButton::Left,
                                    cx.listener(Self::clear_tile_drag),
                                )
                                .on_drag(move_drag, |drag, _, _, cx| {
                                    cx.stop_propagation();
                                    cx.new(|_| drag.clone())
                                })
                                .on_drag_move(cx.listener(Self::on_tile_drag_move)),
                        )
                        .child(
                            div()
                                .id(ElementId::from(SharedString::from(format!(
                                    "{id_text}:tile-resize-r:{ix}"
                                ))))
                                .absolute()
                                .right(px(0.0))
                                .top(px(22.0))
                                .bottom(px(10.0))
                                .w(px(tokens.ui(7.0)))
                                .cursor(CursorStyle::ResizeLeftRight)
                                .hover(|style| style.bg(rgba_from(p.accent, 0.16)))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    start_tile_drag(
                                        dock.clone(),
                                        root,
                                        tile_path.clone(),
                                        ix,
                                        meta,
                                    ),
                                )
                                .on_mouse_up(MouseButton::Left, cx.listener(Self::clear_tile_drag))
                                .on_mouse_up_out(
                                    MouseButton::Left,
                                    cx.listener(Self::clear_tile_drag),
                                )
                                .on_drag(right_drag, |drag, _, _, cx| {
                                    cx.stop_propagation();
                                    cx.new(|_| drag.clone())
                                })
                                .on_drag_move(cx.listener(Self::on_tile_drag_move)),
                        )
                        .child(
                            div()
                                .id(ElementId::from(SharedString::from(format!(
                                    "{id_text}:tile-resize-b:{ix}"
                                ))))
                                .absolute()
                                .left(px(0.0))
                                .right(px(10.0))
                                .bottom(px(0.0))
                                .h(px(tokens.ui(7.0)))
                                .cursor(CursorStyle::ResizeUpDown)
                                .hover(|style| style.bg(rgba_from(p.accent, 0.16)))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    start_tile_drag(
                                        dock.clone(),
                                        root,
                                        tile_path.clone(),
                                        ix,
                                        meta,
                                    ),
                                )
                                .on_mouse_up(MouseButton::Left, cx.listener(Self::clear_tile_drag))
                                .on_mouse_up_out(
                                    MouseButton::Left,
                                    cx.listener(Self::clear_tile_drag),
                                )
                                .on_drag(bottom_drag, |drag, _, _, cx| {
                                    cx.stop_propagation();
                                    cx.new(|_| drag.clone())
                                })
                                .on_drag_move(cx.listener(Self::on_tile_drag_move)),
                        )
                        .child(
                            div()
                                .id(ElementId::from(SharedString::from(format!(
                                    "{id_text}:tile-resize-corner:{ix}"
                                ))))
                                .absolute()
                                .right(px(0.0))
                                .bottom(px(0.0))
                                .size(px(12.0))
                                .cursor(CursorStyle::ResizeUpLeftDownRight)
                                .hover(|style| style.bg(rgba_from(p.accent, 0.22)))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    start_tile_drag(
                                        dock.clone(),
                                        root,
                                        tile_path.clone(),
                                        ix,
                                        meta,
                                    ),
                                )
                                .on_mouse_up(MouseButton::Left, cx.listener(Self::clear_tile_drag))
                                .on_mouse_up_out(
                                    MouseButton::Left,
                                    cx.listener(Self::clear_tile_drag),
                                )
                                .on_drag(corner_drag, |drag, _, _, cx| {
                                    cx.stop_propagation();
                                    cx.new(|_| drag.clone())
                                })
                                .on_drag_move(cx.listener(Self::on_tile_drag_move)),
                        ),
                    );
                }
                tiles.into_any_element()
            }
            DockItem::Split {
                horizontal,
                items,
                sizes,
            } => {
                let p = MoonPalette::active(cx);
                let id_text = id.to_string();
                let split_key = Self::split_key(root, &path);
                let dock = cx.entity();
                let first_child_flexes = items.len() > 1
                    && sizes.len() >= items.len()
                    && sizes.iter().take(items.len()).all(Option::is_some);
                let mut split = div()
                    .id(ElementId::from(id.clone()))
                    .size_full()
                    .relative()
                    .flex()
                    .when(*horizontal, |this| this.flex_row())
                    .when(!*horizontal, |this| this.flex_col());
                split = split.child(
                    canvas(
                        {
                            let dock = dock.clone();
                            let split_key = split_key.clone();
                            move |bounds, _, cx| {
                                dock.update(cx, |area, _| {
                                    area.split_bounds.insert(split_key.clone(), bounds);
                                });
                            }
                        },
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .size_full(),
                );
                for (ix, child) in items.iter().enumerate() {
                    if ix > 0 {
                        let separator = self.resize_handle(
                            SharedString::from(format!("{id_text}:resize:{ix}")),
                            *horizontal,
                            DockResizeTarget::Split {
                                root,
                                path: path.clone(),
                                after_ix: ix,
                            },
                            cx,
                        );
                        split = split.child(separator);
                    }
                    // Zero the minimum on BOTH axes (not just the main one). Otherwise a child
                    // of a vertical split (e.g. the bottom block with a dense row of side-by-side
                    // panels) keeps the min-WIDTH of its content, won't shrink to the window
                    // width, and its sibling (the top block with the chart) ends up a different
                    // width → an empty unfilled region appears to the right of the chart. With
                    // `overflow_hidden` the content is clipped instead.
                    let mut slot = div()
                        .relative()
                        .overflow_hidden()
                        .min_w(px(0.0))
                        .min_h(px(0.0))
                        .when(*horizontal, |this| this.h_full())
                        .when(!*horizontal, |this| this.w_full());
                    slot = child.background_policy(cx).apply(slot, p.shell, 1.0);
                    let slot_size = if first_child_flexes && ix == 0 {
                        None
                    } else {
                        sizes.get(ix).copied().flatten()
                    };
                    if let Some(size) = slot_size {
                        // A fixed panel PREFERS its size (`flex_basis`) but YIELDS
                        // (`flex_shrink`) when the container is narrower than the sum of sizes —
                        // otherwise the block doesn't fit the window, its sibling in the vertical
                        // split stays wider, and an empty region appears to the right of the
                        // chart. On widening it returns to its size (basis preserved).
                        slot = slot.flex_basis(px(size)).flex_shrink_1();
                        slot = if *horizontal {
                            slot.h_full()
                        } else {
                            slot.w_full()
                        };
                    } else {
                        // Flexible slot (e.g. the chart): grows and shrinks. No minimum here —
                        // in a dense row (bottom) the sum of minimums would inflate the block's
                        // min-width and break width sync in the vertical split. The base slot
                        // already has min_w(0)/overflow_hidden, so content is clipped, not forced.
                        slot = slot.flex_1();
                    }

                    let mut child_path = path.clone();
                    child_path.push(ix);
                    split = split.child(slot.child(self.render_item(
                        SharedString::from(format!("{id_text}:split:{ix}")),
                        root,
                        child_path,
                        child,
                        cx,
                    )));
                }
                split.into_any_element()
            }
        }
    }
}

impl Render for DockArea {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = MoonPalette::active(cx);
        let dock = cx.entity();
        let mut root = div()
            .id(ElementId::from(self.id.clone()))
            .relative()
            .size_full()
            .overflow_hidden()
            .flex()
            .flex_col();
        root = self.background_policy.apply(root, p.shell, 1.0);
        root = root.child(
            canvas(
                {
                    let dock = dock.clone();
                    move |bounds, _, cx| {
                        dock.update(cx, |area, _| area.root_bounds = bounds);
                    }
                },
                |_, _, _, _| {},
            )
            .absolute()
            .size_full(),
        );

        if let Some(panel_name) = self.zoomed_panel.as_ref() {
            if let Some(panel) = self.find_panel_named(panel_name.as_ref(), cx) {
                return root.child(
                    TabPanel::new(
                        SharedString::from(format!("{}:zoom", self.id)),
                        vec![panel.clone()],
                    )
                    .dock_context(cx.entity().downgrade(), DockRoot::Center, Vec::new())
                    .background_policy(self.tab_background_policy)
                    .content_background_policy(
                        self.content_background_policy
                            .unwrap_or_else(|| panel.background_policy(cx)),
                    ),
                );
            }
        }

        let mut row = div().relative().flex_1().flex().overflow_hidden();
        row = row.child(
            canvas(
                {
                    let dock = dock.clone();
                    move |bounds, _, cx| {
                        dock.update(cx, |area, _| area.row_bounds = bounds);
                    }
                },
                |_, _, _, _| {},
            )
            .absolute()
            .size_full(),
        );

        if let Some((item, size, true)) = &self.left {
            row = row.child(
                div()
                    .relative()
                    .w(px(*size))
                    .h_full()
                    .child(self.render_item(
                        SharedString::from(format!("{}:left", self.id)),
                        DockRoot::Left,
                        Vec::new(),
                        item,
                        cx,
                    )),
            );
            row = row.child(self.resize_handle(
                SharedString::from(format!("{}:left-resize", self.id)),
                true,
                DockResizeTarget::OuterLeft,
                cx,
            ));
        }

        // `min_w(0)`: a flex item's min-width defaults to auto (= min-content), so without it
        // the center wrapper could not shrink below its CONTENT min-width (set by a dense bottom
        // row of side-by-side panels). On window narrowing the center overflowed it and the top
        // block with the chart did not stretch to full width → an empty region on the right.
        // `min_w(0)` + the row's `overflow_hidden` let the center shrink to the window.
        row = row.child(
            div()
                .relative()
                .flex_1()
                .h_full()
                .min_w(px(0.))
                .child(self.render_item(
                    SharedString::from(format!("{}:center", self.id)),
                    DockRoot::Center,
                    Vec::new(),
                    &self.center,
                    cx,
                )),
        );

        if let Some((item, size, true)) = &self.right {
            row = row.child(self.resize_handle(
                SharedString::from(format!("{}:right-resize", self.id)),
                true,
                DockResizeTarget::OuterRight,
                cx,
            ));
            row = row.child(
                div()
                    .relative()
                    .w(px(*size))
                    .h_full()
                    .child(self.render_item(
                        SharedString::from(format!("{}:right", self.id)),
                        DockRoot::Right,
                        Vec::new(),
                        item,
                        cx,
                    )),
            );
        }

        root = root.child(row);

        if let Some((item, size, true)) = &self.bottom {
            root = root.child(self.resize_handle(
                SharedString::from(format!("{}:bottom-resize", self.id)),
                false,
                DockResizeTarget::OuterBottom,
                cx,
            ));
            root = root.child(
                div()
                    .relative()
                    .h(px(*size))
                    .w_full()
                    .child(self.render_item(
                        SharedString::from(format!("{}:bottom", self.id)),
                        DockRoot::Bottom,
                        Vec::new(),
                        item,
                        cx,
                    )),
            );
        }

        root
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DOCK_TILE_MIN_H, DockArea, DockItem, DockRoot, MoonDockPanel, PanelView, TileMeta,
    };
    use crate::moon::MoonBackgroundPolicy;
    use gpui::{Bounds, IntoElement as _, div, point, px, size};
    use std::rc::Rc;

    fn panel(name: &'static str) -> Rc<dyn PanelView> {
        Rc::new(MoonDockPanel::new(name, name, |_, _| {
            div().into_any_element()
        }))
    }

    #[test]
    fn moon_dock_panel_builder_flags_are_observable() {
        let panel = MoonDockPanel::new("orders", "Orders", |_, _| div().into_any_element())
            .background_policy(MoonBackgroundPolicy::NoFill)
            .closable(false)
            .zoomable(false)
            .detachable(true)
            .show_dock_header(true)
            .visible(false);

        assert_eq!(panel.background_policy, MoonBackgroundPolicy::NoFill);
        assert!(!panel.closable);
        assert!(!panel.zoomable);
        assert!(panel.detachable);
        assert!(panel.show_dock_header);
        assert!(!panel.visible);
    }

    #[test]
    fn dock_item_add_panel_creates_tabs_and_activates_new_panel() {
        let first = panel("first");
        let second = panel("second");

        let item = DockItem::Panel(first.clone()).with_panel_added(second.clone());

        let DockItem::Tabs { items, active_ix } = item else {
            panic!("expected adding a panel to a panel to create a tab set");
        };
        assert_eq!(active_ix, 1);
        assert_eq!(items.len(), 2);
        assert!(Rc::ptr_eq(&items[0], &first));
        assert!(Rc::ptr_eq(&items[1], &second));
    }

    #[test]
    fn dock_clamps_tile_meta_inside_root_bounds() {
        let bounds = Bounds::new(point(px(0.0), px(0.0)), size(px(300.0), px(200.0)));
        let clamped = DockArea::clamp_tile_meta(
            TileMeta {
                x: -50.0,
                y: 500.0,
                w: 900.0,
                h: 20.0,
                z_index: 7,
            },
            bounds,
        );

        assert_eq!(clamped.x, 0.0);
        assert_eq!(clamped.y, 104.0);
        assert_eq!(clamped.w, 300.0);
        assert_eq!(clamped.h, DOCK_TILE_MIN_H);
        assert_eq!(clamped.z_index, 7);
    }

    #[gpui::test]
    fn move_panel_to_tabs_resolves_target_after_take(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            let first = panel("first");
            let moved = panel("moved");
            let target_a = panel("target_a");
            let target_b = panel("target_b");
            let mut dock = DockArea::test_with_center(DockItem::Split {
                horizontal: true,
                sizes: Vec::new(),
                items: vec![
                    DockItem::Tabs {
                        items: vec![first.clone(), moved.clone()],
                        active_ix: 1,
                    },
                    DockItem::Tabs {
                        items: vec![target_a.clone(), target_b.clone()],
                        active_ix: 0,
                    },
                ],
            });

            assert!(dock.move_panel_to_tabs("moved", DockRoot::Center, &[1], 1, cx));

            let DockItem::Split { items, .. } = &dock.center else {
                panic!("expected root split to survive tab move");
            };
            assert_eq!(items.len(), 2);

            let DockItem::Panel(panel) = &items[0] else {
                panic!("source tab strip should collapse to its remaining panel");
            };
            assert_eq!(panel.panel_name(cx).as_ref(), "first");

            let DockItem::Tabs { items, active_ix } = &items[1] else {
                panic!("target tab strip should stay the target");
            };
            let names = items
                .iter()
                .map(|panel| panel.panel_name(cx).to_string())
                .collect::<Vec<_>>();
            assert_eq!(names, vec!["target_a", "moved", "target_b"]);
            assert_eq!(*active_ix, 1);
        });
    }

    #[gpui::test]
    fn move_panel_to_tabs_ignores_self_drop_before_take(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            let only = panel("only");
            let mut dock = DockArea::test_with_center(DockItem::Panel(only.clone()));

            assert!(!dock.move_panel_to_tabs("only", DockRoot::Center, &[], 0, cx));

            let DockItem::Panel(panel) = &dock.center else {
                panic!("self-drop must leave the original panel in place");
            };
            assert_eq!(panel.panel_name(cx).as_ref(), "only");
        });
    }
}
