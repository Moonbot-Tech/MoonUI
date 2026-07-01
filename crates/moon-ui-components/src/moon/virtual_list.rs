use std::ops::Range;

use gpui::prelude::FluentBuilder;
use gpui::*;

use super::{
    background::MoonBackgroundPolicy,
    scroll_area::{MoonScrollAxis, MoonScrollbarVisibility, moon_scrollbar_overlay_with_palette},
    tokens::{MoonPalette, MoonRect, rgba_from},
};

pub type MoonVirtualListScrollHandle = UniformListScrollHandle;

type MoonVirtualItemRenderer =
    Box<dyn for<'a> Fn(usize, &'a mut Window, &'a mut App) -> AnyElement>;
type MoonVirtualRangeObserver = Box<dyn for<'a> Fn(Range<usize>, &'a mut Window, &'a mut App)>;

#[derive(IntoElement)]
pub struct MoonVirtualList {
    id: SharedString,
    bounds: Option<MoonRect>,
    item_count: usize,
    item_height: f32,
    render_item: MoonVirtualItemRenderer,
    on_visible_range: Option<MoonVirtualRangeObserver>,
    scroll_handle: Option<MoonVirtualListScrollHandle>,
    scrollbar_visibility: MoonScrollbarVisibility,
    surface: bool,
    background_policy: MoonBackgroundPolicy,
    border: bool,
    radius: f32,
    padding: f32,
    y_flipped: bool,
    tail_fill: Option<u32>,
}

impl MoonVirtualList {
    pub fn new<R>(
        id: impl Into<SharedString>,
        item_count: usize,
        item_height: f32,
        render_item: impl 'static + Fn(usize, &mut Window, &mut App) -> R,
    ) -> Self
    where
        R: IntoElement,
    {
        let render_item = Box::new(move |ix, window: &mut Window, cx: &mut App| {
            render_item(ix, window, cx).into_any_element()
        });

        Self {
            id: id.into(),
            bounds: None,
            item_count,
            item_height,
            render_item,
            on_visible_range: None,
            scroll_handle: None,
            scrollbar_visibility: MoonScrollbarVisibility::Hover,
            surface: true,
            background_policy: MoonBackgroundPolicy::Opaque,
            border: true,
            radius: 5.0,
            padding: 0.0,
            y_flipped: false,
            tail_fill: None,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn track_scroll(mut self, scroll_handle: &MoonVirtualListScrollHandle) -> Self {
        self.scroll_handle = Some(scroll_handle.clone());
        self
    }

    pub fn on_visible_range(
        mut self,
        on_visible_range: impl 'static + Fn(Range<usize>, &mut Window, &mut App),
    ) -> Self {
        self.on_visible_range = Some(Box::new(on_visible_range));
        self
    }

    pub fn scrollbar_visibility(mut self, visibility: MoonScrollbarVisibility) -> Self {
        self.scrollbar_visibility = visibility;
        self
    }

    pub fn surface(mut self, surface: bool) -> Self {
        self.surface = surface;
        self
    }

    pub fn background_policy(mut self, policy: MoonBackgroundPolicy) -> Self {
        self.background_policy = policy;
        self
    }

    pub fn border(mut self, border: bool) -> Self {
        self.border = border;
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    pub fn y_flipped(mut self, y_flipped: bool) -> Self {
        self.y_flipped = y_flipped;
        self
    }

    pub fn tail_fill_color(mut self, color: u32) -> Self {
        self.tail_fill = Some(color);
        self
    }

    fn render_range(
        render_item: &MoonVirtualItemRenderer,
        item_height: f32,
        range: Range<usize>,
        window: &mut Window,
        cx: &mut App,
    ) -> Vec<AnyElement> {
        let mut elements = Vec::with_capacity(range.len());
        for ix in range {
            elements.push(
                div()
                    .relative()
                    .w_full()
                    .h(px(item_height))
                    .overflow_hidden()
                    .child(render_item(ix, window, cx))
                    .into_any_element(),
            );
        }
        elements
    }
}

impl RenderOnce for MoonVirtualList {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = MoonPalette::active(cx);
        let id = self.id.clone();
        let scroll_handle = self.scroll_handle.unwrap_or_else(|| {
            window
                .use_keyed_state(
                    ElementId::from(SharedString::from(format!("{}:scroll", self.id))),
                    cx,
                    |_, _| MoonVirtualListScrollHandle::new(),
                )
                .read(cx)
                .clone()
        });

        let render_item = self.render_item;
        let on_visible_range = self.on_visible_range;
        let item_height = self.item_height;
        let mut root = div()
            .id(ElementId::from(id.clone()))
            .relative()
            .overflow_hidden()
            .rounded(px(self.radius))
            .when(self.border, |this| {
                this.border(px(1.0)).border_color(rgba_from(p.border, 1.0))
            });
        if self.surface {
            root = self.background_policy.apply(root, p.shell_high, 0.98);
        }

        if let Some(bounds) = self.bounds {
            root = root
                .absolute()
                .left(px(bounds.x))
                .top(px(bounds.y))
                .w(px(bounds.w))
                .h(px(bounds.h));
        } else {
            root = root.size_full();
        }

        let list_id = ElementId::from(SharedString::from(format!("{}:list", id)));
        let mut list = uniform_list(list_id, self.item_count, move |range, window, cx| {
            if let Some(on_visible_range) = &on_visible_range {
                on_visible_range(range.clone(), window, cx);
            }
            Self::render_range(&render_item, item_height, range, window, cx)
        })
        .size_full()
        .p(px(self.padding))
        .track_scroll(&scroll_handle)
        .y_flipped(self.y_flipped);
        if self.surface {
            list = self.background_policy.apply(list, p.shell_high, 0.98);
        }

        if let Some(color) = self.tail_fill {
            list = list.with_decoration(MoonVirtualListTailFill { color });
        }

        root = root.child(list);

        let base_handle = scroll_handle.0.borrow().base_handle.clone();
        if let Some(scrollbar) = moon_scrollbar_overlay_with_palette(
            SharedString::from(format!("{id}:scrollbar")),
            &base_handle,
            MoonScrollAxis::Vertical,
            self.scrollbar_visibility,
            p,
            window,
            cx,
        ) {
            root = root.child(scrollbar);
        }

        root
    }
}

struct MoonVirtualListTailFill {
    color: u32,
}

impl UniformListDecoration for MoonVirtualListTailFill {
    fn compute(
        &self,
        _visible_range: Range<usize>,
        _bounds: Bounds<Pixels>,
        _scroll_offset: Point<Pixels>,
        item_height: Pixels,
        item_count: usize,
        _window: &mut Window,
        _cx: &mut App,
    ) -> AnyElement {
        div()
            .relative()
            .size_full()
            .child(
                div()
                    .absolute()
                    .left(px(0.0))
                    .right(px(0.0))
                    .top(item_height * item_count)
                    .bottom(px(0.0))
                    .bg(rgba_from(self.color, 1.0)),
            )
            .into_any_element()
    }
}
