//! Full-path regression coverage for optional virtual-list section chrome.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use gpui::{
    App, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, Styled as _, Window, div, px,
};

use super::{List, ListState};
use crate::{
    IndexPath,
    list::{ListDelegate, ListItem, cache::RowEntry},
};

/// Delegate with a headerless global section, one headed section, and a large trailing section.
struct OptionalChromeDelegate {
    /// Number of item elements constructed across measurement and visible rendering.
    item_renders: Rc<Cell<usize>>,
}

impl ListDelegate for OptionalChromeDelegate {
    type Item = ListItem;

    /// Return the three test sections.
    ///
    /// Args:
    ///     _cx: Application context unused by the fixed fixture.
    ///
    /// Returns:
    ///     Three sections.
    fn sections_count(&self, _cx: &App) -> usize {
        3
    }

    /// Return one global row, two grouped rows, and a large virtualized tail.
    ///
    /// Args:
    ///     section: Fixture section index.
    ///     _cx: Application context unused by the fixed fixture.
    ///
    /// Returns:
    ///     The fixture row count for the section.
    fn items_count(&self, section: usize, _cx: &App) -> usize {
        match section {
            0 => 1,
            1 => 2,
            2 => 10_000,
            _ => 0,
        }
    }

    /// Render one fixed-height item and count the work performed.
    ///
    /// Args:
    ///     ix: Section and row identity to expose through a debug selector.
    ///     _window: Test window unused by the fixed item.
    ///     _cx: List context unused by the fixed item.
    ///
    /// Returns:
    ///     One 20-pixel list item.
    fn render_item(
        &mut self,
        ix: IndexPath,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        self.item_renders.set(self.item_renders.get() + 1);
        let selector = format!("optional-row-{}-{}", ix.section, ix.row);
        Some(
            ListItem::new(selector.clone())
                .h(px(20.0))
                .child(div().debug_selector(move || selector.clone()).size_full()),
        )
    }

    /// Render a header only for the middle section.
    ///
    /// Args:
    ///     section: Fixture section index.
    ///     _window: Test window unused by the fixed header.
    ///     _cx: List context unused by the fixed header.
    ///
    /// Returns:
    ///     A 12-pixel header for section 1, otherwise no virtual row.
    fn render_section_header(
        &mut self,
        section: usize,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) -> Option<impl IntoElement> {
        (section == 1).then(|| {
            div()
                .debug_selector(|| "optional-header-1".to_string())
                .h(px(12.0))
        })
    }

    /// Render a footer only for the middle section.
    ///
    /// Args:
    ///     section: Fixture section index.
    ///     _window: Test window unused by the fixed footer.
    ///     _cx: List context unused by the fixed footer.
    ///
    /// Returns:
    ///     An 8-pixel footer for section 1, otherwise no virtual row.
    fn render_section_footer(
        &mut self,
        section: usize,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) -> Option<impl IntoElement> {
        (section == 1).then(|| {
            div()
                .debug_selector(|| "optional-footer-1".to_string())
                .h(px(8.0))
        })
    }

    /// Store no keyboard selection in the read-only geometry fixture.
    ///
    /// Args:
    ///     _ix: Ignored selected index.
    ///     _window: Test window.
    ///     _cx: List context.
    ///
    /// Returns:
    ///     Nothing.
    fn set_selected_index(
        &mut self,
        _ix: Option<IndexPath>,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) {
    }
}

/// Root view that gives the virtual list a bounded viewport.
struct OptionalChromeHarness {
    /// List state inspected after the render path prepares its row cache.
    list: Entity<ListState<OptionalChromeDelegate>>,
}

impl Render for OptionalChromeHarness {
    /// Render the bounded list fixture.
    ///
    /// Args:
    ///     _window: Test window.
    ///     _cx: Test view context.
    ///
    /// Returns:
    ///     A 320-by-240-pixel virtual list.
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .debug_selector(|| "optional-list-root".to_string())
            .w(px(320.0))
            .h(px(240.0))
            .child(List::new(&self.list))
    }
}

/// `list.rs:ListState::prepare_items_if_needed` must inspect every section's optional chrome while
/// laying out only the first present header/footer. Restoring section-0-only measurement makes the
/// section-1 header/footer zero-height and overlap adjacent Report rows; unconditionally inserting
/// chrome rows creates phantom gaps. Eagerly rendering all 10,003 items violates virtualization.
#[gpui::test]
fn later_optional_chrome_has_space_without_phantom_rows_or_eager_items(
    cx: &mut gpui::TestAppContext,
) {
    cx.update(crate::init);
    let item_renders = Rc::new(Cell::new(0));
    let list_slot = Rc::new(RefCell::new(None));
    let window = cx.add_window({
        let item_renders = item_renders.clone();
        let list_slot = list_slot.clone();
        move |window, cx| {
            let list = cx.new(|cx| {
                ListState::new(
                    OptionalChromeDelegate {
                        item_renders: item_renders.clone(),
                    },
                    window,
                    cx,
                )
            });
            list_slot.replace(Some(list.clone()));
            OptionalChromeHarness { list }
        }
    });
    let mut cx = gpui::VisualTestContext::from_window(window.into(), cx);
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();

    let list_root = cx
        .debug_bounds("optional-list-root")
        .expect("list root must render");
    let _global_row = cx
        .debug_bounds("optional-row-0-0")
        .expect("global row must render");
    let header = cx
        .debug_bounds("optional-header-1")
        .expect("later section header must render");
    let first_group_row = cx
        .debug_bounds("optional-row-1-0")
        .expect("first grouped row must render");
    let second_group_row = cx
        .debug_bounds("optional-row-1-1")
        .expect("second grouped row must render");
    let footer = cx
        .debug_bounds("optional-footer-1")
        .expect("later section footer must render");

    assert_eq!(header.origin.y, list_root.origin.y + px(20.0));
    assert_eq!(first_group_row.origin.y, header.origin.y + px(22.0));
    assert_eq!(
        second_group_row.origin.y,
        first_group_row.origin.y + px(20.0)
    );
    assert_eq!(footer.origin.y, header.origin.y + px(52.0));

    let list = list_slot
        .borrow()
        .clone()
        .expect("window construction must publish its list state");
    cx.update(|_, app| {
        let state = list.read(app);
        assert_eq!(
            state.rows_cache.entities.get(0..6),
            Some(
                [
                    RowEntry::Entry(IndexPath::new(0).section(0)),
                    RowEntry::SectionHeader(1),
                    RowEntry::Entry(IndexPath::new(0).section(1)),
                    RowEntry::Entry(IndexPath::new(1).section(1)),
                    RowEntry::SectionFooter(1),
                    RowEntry::Entry(IndexPath::new(0).section(2)),
                ]
                .as_slice()
            )
        );
        assert_eq!(state.rows_cache.entries_sizes[1].height, px(12.0));
        assert_eq!(state.rows_cache.entries_sizes[4].height, px(8.0));
    });
    assert!(
        item_renders.get() < 100,
        "bounded viewport must not construct the 10,003-item population"
    );
}
