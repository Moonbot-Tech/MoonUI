//! Actual-render accounting must exclude cache reuse, idle notifications, and disabled periods.

use std::any::type_name;

use crate::{
    AnyView, AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render,
    StyleRefinement, TestAppContext, Window, div,
};

/// An uninstrumented application view: the runtime must discover its concrete type itself.
struct CountedView;

impl Render for CountedView {
    /// Return an ordinary element without any diagnostic call site.
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().child("rendered")
    }
}

/// Keep a child cached while the uncached parent still builds on each draw.
struct CachedChild {
    child: Entity<CountedView>,
}

impl Render for CachedChild {
    /// Return the normal AnyView cache path, without custom instrumentation.
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().child(AnyView::from(self.child.clone()).cached(StyleRefinement::default()))
    }
}

/// A counter in request_layout/prepaint would incorrectly count cached reuse as a render.
#[test]
fn cached_reuse_does_not_count_as_rendering_the_child() {
    let mut app = TestAppContext::single();
    let window = app.add_window(|_, cx| CachedChild {
        child: cx.new(|_| CountedView),
    });
    app.update_window(window.into(), |_, window, cx| {
        cx.set_view_render_tracking(true);
        window.refresh();
        window.draw(cx).clear();
        window.draw(cx).clear();
        let counts: std::collections::BTreeMap<_, _> =
            cx.take_view_render_counts().into_iter().collect();
        assert_eq!(counts.get(type_name::<CachedChild>()), Some(&2));
        assert_eq!(counts.get(type_name::<CountedView>()), Some(&1));
    })
    .unwrap();
}

/// Removing common dispatch accounting, counting notify instead, retaining old samples, or
/// leaving tracking active after disable must break this full view-render path.
#[test]
fn tracking_counts_actual_renders_and_clears_disabled_intervals() {
    let mut app = TestAppContext::single();
    let window = app.add_window(|_, _| CountedView);
    app.update_window(window.into(), |_, window, cx| {
        window.draw(cx).clear();
        assert!(
            cx.take_view_render_counts().is_empty(),
            "tracking defaults off"
        );
        cx.set_view_render_tracking(true);
        window.draw(cx).clear();
        window.draw(cx).clear();
        assert_eq!(
            cx.take_view_render_counts(),
            vec![(type_name::<CountedView>(), 2)]
        );
        assert!(
            cx.take_view_render_counts().is_empty(),
            "sampling drains the interval"
        );
        window.draw(cx).clear();
        cx.set_view_render_tracking(true);
        assert_eq!(
            cx.take_view_render_counts(),
            vec![(type_name::<CountedView>(), 1)],
            "reapplying the gate preserves the current interval"
        );
        window.draw(cx).clear();
        cx.set_view_render_tracking(false);
        window.draw(cx).clear();
        cx.set_view_render_tracking(true);
        assert!(
            cx.take_view_render_counts().is_empty(),
            "disabled work cannot leak into a new interval"
        );
    })
    .unwrap();
}
