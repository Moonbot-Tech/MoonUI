//! Panel abstractions, host-backed panels, and the panel factory registry.

use std::{borrow::BorrowMut, collections::HashMap, rc::Rc};

use gpui::*;

use super::{
    DockArea, MoonBackgroundPolicy, MoonPalette, MoonText, PanelEvent, PanelInfo, PanelState,
    TileMeta,
};

/// Defines the behavior and lifecycle hooks required from a dockable entity.
pub trait Panel: EventEmitter<PanelEvent> + Render {
    /// Return the stable name used to serialize and restore this panel.
    fn panel_name(&self) -> &'static str;

    /// Return an optional label that overrides the panel title in dock tabs.
    fn tab_name(&self, _cx: &App) -> Option<SharedString> {
        None
    }

    /// Build the element displayed as the panel title.
    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.panel_name()
    }

    /// Build optional content displayed after the panel title.
    fn title_suffix(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        None
    }

    /// Return whether the host may close this panel.
    fn closable(&self, _cx: &App) -> bool {
        true
    }

    /// Return whether the host may zoom this panel to fill the dock area.
    fn zoomable(&self, _cx: &App) -> bool {
        true
    }

    /// Return whether the host may detach this panel into another window.
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

    /// Return whether this panel should participate in dock layout and rendering.
    fn visible(&self, _cx: &App) -> bool {
        true
    }

    /// Notify the panel when it becomes or stops being the active item.
    fn set_active(&mut self, _active: bool, _window: &mut Window, _cx: &mut Context<Self>) {}

    /// Notify the panel when its dock slot enters or leaves the zoomed state.
    fn set_zoomed(&mut self, _zoomed: bool, _window: &mut Window, _cx: &mut Context<Self>) {}

    /// Serialize the panel state used to reconstruct the dock layout.
    fn dump(&self, _cx: &App) -> PanelState {
        PanelState::new(self.panel_name())
    }

    /// Return how the dock host should paint behind the panel.
    fn background_policy(&self, _cx: &App) -> MoonBackgroundPolicy {
        MoonBackgroundPolicy::Opaque
    }

    /// Notify the panel after it is attached to a dock area.
    fn on_added_to(
        &mut self,
        _dock_area: WeakEntity<DockArea>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
    }

    /// Build optional toolbar controls for the active panel.
    fn toolbar_buttons(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Vec<AnyElement>> {
        None
    }

    /// Notify the panel immediately before it is removed from the dock area.
    fn on_removed(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {}
}

/// Type-erased interface used by dock containers to operate on heterogeneous panels.
pub trait PanelView: 'static {
    /// Return the stable serialized panel name.
    fn panel_name(&self, cx: &App) -> SharedString;
    /// Return an optional dock-tab label override.
    fn tab_name(&self, cx: &App) -> Option<SharedString>;
    /// Build the panel title element.
    fn title(&self, window: &mut Window, cx: &mut App) -> AnyElement;
    /// Build optional content displayed after the panel title.
    fn title_suffix(&self, window: &mut Window, cx: &mut App) -> Option<AnyElement>;
    /// Build the panel body element.
    fn render_panel(&self, window: &mut Window, cx: &mut App) -> AnyElement;
    /// Serialize the panel state for layout persistence.
    fn dump(&self, cx: &App) -> PanelState;
    /// Return how the dock host should paint behind the panel.
    fn background_policy(&self, cx: &App) -> MoonBackgroundPolicy;
    /// Return whether the host may close the panel.
    fn closable(&self, cx: &App) -> bool;
    /// Return whether the host may zoom the panel.
    fn zoomable(&self, cx: &App) -> bool;
    /// Return whether the host may detach the panel.
    fn detachable(&self, cx: &App) -> bool;
    /// Return whether a single-panel slot should retain its dock header.
    fn show_dock_header(&self, cx: &App) -> bool;
    /// Return whether the panel should participate in layout and rendering.
    fn visible(&self, cx: &App) -> bool;
    /// Notify the panel when its active state changes.
    fn set_active(&self, active: bool, window: &mut Window, cx: &mut App);
    /// Notify the panel when its zoomed state changes.
    fn set_zoomed(&self, zoomed: bool, window: &mut Window, cx: &mut App);
    /// Notify the panel after it is attached to a dock area.
    fn on_added_to(&self, dock_area: WeakEntity<DockArea>, window: &mut Window, cx: &mut App);
    /// Notify the panel immediately before removal from the dock area.
    fn on_removed(&self, window: &mut Window, cx: &mut App);
    /// Build optional toolbar controls for the active panel.
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

/// Lightweight dock panel backed by caller-provided render closures.
pub struct MoonDockPanel {
    panel_name: SharedString,
    title: SharedString,
    render: MoonPanelRender,
    /// Optional element drawn on this panel's dock tab, right of its label — an unread badge, a
    /// status dot. Absent by default, which renders the tab exactly as before.
    tab_suffix: Option<MoonPanelRender>,
    background_policy: MoonBackgroundPolicy,
    closable: bool,
    zoomable: bool,
    detachable: bool,
    show_dock_header: bool,
    visible: bool,
}

/// Closure used by a [`MoonDockPanel`] to build its body or tab suffix.
type MoonPanelRender = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>;

impl MoonDockPanel {
    /// Create a panel with a stable name, display title, and body renderer.
    pub fn new(
        panel_name: impl Into<SharedString>,
        title: impl Into<SharedString>,
        render: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            panel_name: panel_name.into(),
            title: title.into(),
            render: Rc::new(render),
            tab_suffix: None,
            background_policy: MoonBackgroundPolicy::Opaque,
            closable: true,
            zoomable: true,
            detachable: false,
            show_dock_header: false,
            visible: true,
        }
    }

    /// Draw `suffix` on this panel's dock tab, right of the label.
    ///
    /// Takes a closure, not a finished element, because the tab is rebuilt on every frame: a
    /// counter passed by value would freeze at whatever it was when the panel was constructed.
    pub fn tab_suffix(
        mut self,
        suffix: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.tab_suffix = Some(Rc::new(suffix));
        self
    }

    /// Set the background policy used by the dock host.
    pub fn background_policy(mut self, policy: MoonBackgroundPolicy) -> Self {
        self.background_policy = policy;
        self
    }

    /// Set whether the host may close the panel.
    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    /// Set whether the host may zoom the panel.
    pub fn zoomable(mut self, zoomable: bool) -> Self {
        self.zoomable = zoomable;
        self
    }

    /// Set whether the host may detach the panel into another window.
    pub fn detachable(mut self, detachable: bool) -> Self {
        self.detachable = detachable;
        self
    }

    /// Set whether a single-panel slot should retain its dock header.
    pub fn show_dock_header(mut self, show_dock_header: bool) -> Self {
        self.show_dock_header = show_dock_header;
        self
    }

    /// Set whether the panel participates in dock layout and rendering.
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

    fn title_suffix(&self, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        self.tab_suffix.as_ref().map(|suffix| suffix(window, cx))
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

/// Recursive dock-layout node containing panels, tabs, tiles, or nested splits.
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
    /// Wrap an existing type-erased panel as one dock node.
    pub fn panel(panel: Rc<dyn PanelView>) -> Self {
        Self::Panel(panel)
    }

    /// Attach an entity-backed panel to a dock area and wrap it as one node.
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

    /// Attach several panels and create a tab group with the first item active.
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

    /// Create a split node without explicit child sizes.
    pub fn split(horizontal: bool, items: Vec<DockItem>) -> Self {
        Self::Split {
            horizontal,
            items,
            sizes: Vec::new(),
        }
    }

    /// Create a freeform tile group with metadata aligned to its panels.
    pub fn tiles(items: Vec<Rc<dyn PanelView>>, metas: Vec<TileMeta>) -> Self {
        Self::Tiles { items, metas }
    }

    /// Create a split node using the supplied axis and optional rendered sizes.
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

    /// Create a horizontal split without explicit child sizes.
    pub fn h_split(
        items: Vec<DockItem>,
        dock_area: &WeakEntity<DockArea>,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        Self::split_with_sizes(Axis::Horizontal, items, Vec::new(), dock_area, window, cx)
    }

    /// Create a vertical split without explicit child sizes.
    pub fn v_split(
        items: Vec<DockItem>,
        dock_area: &WeakEntity<DockArea>,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        Self::split_with_sizes(Axis::Vertical, items, Vec::new(), dock_area, window, cx)
    }
}

/// Factory used to reconstruct one registered panel from persisted state.
type PanelFactory =
    Rc<dyn Fn(&PanelState, &PanelInfo, &mut Window, &mut App) -> Rc<dyn PanelView> + 'static>;

/// Global registry mapping serialized panel names to reconstruction factories.
#[derive(Default)]
pub(super) struct MoonPanelRegistry {
    factories: HashMap<String, PanelFactory>,
}

impl Global for MoonPanelRegistry {}

impl MoonPanelRegistry {
    /// Rebuild a registered panel or return a visible fallback for an unknown name.
    pub(super) fn build_panel<C>(
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

/// Register the factory used to reconstruct a serialized panel name.
pub fn register_panel<F>(cx: &mut App, panel_name: &str, factory: F)
where
    F: Fn(&PanelState, &PanelInfo, &mut Window, &mut App) -> Rc<dyn PanelView> + 'static,
{
    cx.default_global::<MoonPanelRegistry>()
        .factories
        .insert(panel_name.to_string(), Rc::new(factory));
}
