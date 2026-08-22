//! Data-table header rendering and column interaction.

use std::rc::Rc;

use gpui::prelude::FluentBuilder;
use gpui::*;

use super::super::{
    table::{MoonTableAlign, MoonTableStyle},
    theme::MoonTheme,
    tokens::{MoonPalette, rgba_from},
};
use super::drag::{MoonDataColumnDrag, MoonDataColumnResizeDrag};
use super::{
    MIN_COLUMN_WIDTH, MoonDataColumnHandler, MoonDataContextMenuBuilder, MoonDataTable,
    MoonDataTableColumn, MoonDataTableContextTarget, MoonDataTableEvent, MoonDataTableState,
};

/// Render the header using the resolved visual column order.
///
/// # Arguments
///
/// * `id` - Stable table identity used to derive header element IDs.
/// * `columns` - Columns in their resolved visual order.
/// * `state` - Retained table state for sorting, widths, and drag interactions.
/// * `height` - Header height in pixels.
/// * `left_offset` - Horizontal offset reserved for the row-header gutter.
/// * `style` - Resolved table style.
/// * `column_selectable` - Whether header clicks select columns.
/// * `controlled_row_selection` - Whether the caller owns all selection highlights.
/// * `on_select_column` - Optional column-selection callback.
/// * `on_right_click_column` - Optional column context callback.
/// * `context_menu_builder` - Optional context-menu content builder.
/// * `on_sort` - Optional sort-change callback.
/// * `window` - Active GPUI window.
/// * `cx` - Application context.
///
/// # Returns
///
/// The complete header element.
pub(super) fn render_header(
    id: &SharedString,
    columns: &[MoonDataTableColumn],
    state: &Entity<MoonDataTableState>,
    height: f32,
    left_offset: f32,
    style: MoonTableStyle,
    column_selectable: bool,
    controlled_row_selection: bool,
    on_select_column: Option<MoonDataColumnHandler>,
    on_right_click_column: Option<MoonDataColumnHandler>,
    context_menu_builder: Option<MoonDataContextMenuBuilder>,
    on_sort: Option<Rc<dyn Fn(&SharedString, bool, &mut Window, &mut App)>>,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    let p = MoonPalette::active(cx);
    let tokens = MoonTheme::active_tokens(cx);
    let state_id = state.entity_id();
    let mut header = div()
        .id(ElementId::from(SharedString::from(format!("{id}:header"))))
        .absolute()
        .left(px(left_offset))
        .top(px(0.0))
        .right(px(0.0))
        .h(px(height))
        .flex()
        .items_center()
        .bg(rgba_from(style.header_bg, 1.0))
        .border_b(px(1.0))
        .border_color(rgba_from(
            style.header_separator,
            style.header_separator_alpha,
        ));

    let sort_column = state.read(cx).sort_column.clone();
    let sort_ascending = state.read(cx).sort_ascending;
    let all_keys = columns
        .iter()
        .map(|column| column.key.clone())
        .collect::<Vec<_>>();
    for (column_ix, column) in columns.iter().enumerate() {
        let key = column.key.clone();
        let key_string = key.to_string();
        let sortable = column.sortable;
        let resizable = column.resizable;
        let movable = column.movable;
        let sorted = sort_column.as_ref() == Some(&key);
        let label = if sorted {
            format!(
                "{} {}",
                column.title,
                if sort_ascending { "↑" } else { "↓" }
            )
        } else {
            column.title.to_string()
        };
        let mut cell = div()
            .id(ElementId::from(SharedString::from(format!(
                "{id}:header:{key_string}"
            ))))
            .relative()
            .when(column.fill, |this| this.min_w(px(column.width)).flex_1())
            .when(!column.fill, |this| this.w(px(column.width)).flex_none())
            .h_full()
            .flex()
            .items_center()
            .when(matches!(column.align, MoonTableAlign::Right), |this| {
                this.justify_end()
            })
            .when(matches!(column.align, MoonTableAlign::Left), |this| {
                this.justify_start()
            })
            .pl(px(tokens.ui(10.0)))
            .pr(px(tokens.ui(8.0)))
            .text_size(px(tokens.font(9.5)))
            .line_height(px(tokens.line_height(11.0)))
            .text_color(rgba_from(
                if sorted {
                    p.text_soft
                } else {
                    style.header_text
                },
                1.0,
            ))
            .child(label);

        cell = cell.child(
            canvas(
                {
                    let state = state.clone();
                    let key_string = key_string.clone();
                    move |bounds, _, cx| {
                        state.update(cx, |state, _| {
                            state.record_header_bounds(key_string.clone(), bounds);
                        });
                    }
                },
                |_, _, _, _| {},
            )
            .absolute()
            .size_full(),
        );

        if sortable || column_selectable {
            let state = state.clone();
            let key = key.clone();
            let on_sort = on_sort.clone();
            let on_select_column = on_select_column.clone();
            cell = cell
                .cursor_pointer()
                .hover(|this| this.bg(rgba_from(p.panel_high, 0.72)))
                .on_click(move |_, window, cx| {
                    MoonDataTable::handle_header_click(
                        &state,
                        column_ix,
                        &key,
                        sortable,
                        column_selectable,
                        controlled_row_selection,
                        on_select_column.as_ref(),
                        on_sort.as_ref(),
                        window,
                        cx,
                    );
                });
        }

        {
            let state = state.clone();
            let table_id = id.clone();
            let on_right_click_column = on_right_click_column.clone();
            let context_menu_builder = context_menu_builder.clone();
            cell = cell.on_mouse_down(MouseButton::Right, move |event, window, cx| {
                let target = MoonDataTableContextTarget::Column(column_ix);
                state.update(cx, |state, cx| {
                    state.open_context_menu(target.clone(), event.position);
                    cx.emit(MoonDataTableEvent::RightClickedColumn(column_ix));
                    cx.notify();
                });
                MoonDataTable::show_context_menu_layer(
                    &table_id,
                    &state,
                    target,
                    event.position,
                    context_menu_builder.as_ref(),
                    window,
                    cx,
                );
                if let Some(on_right_click_column) = &on_right_click_column {
                    on_right_click_column(column_ix, window, cx);
                }
                cx.stop_propagation();
            });
        }

        if movable {
            let drag = MoonDataColumnDrag {
                state_id,
                key: key.clone(),
            };
            let state_for_drop = state.clone();
            let all_keys_for_drop = all_keys.clone();
            let target_key = key.clone();
            cell = cell
                .on_drag(drag, |drag, _, _, cx| {
                    cx.stop_propagation();
                    cx.new(|_| drag.clone())
                })
                .drag_over::<MoonDataColumnDrag>(|style, _, _, cx| {
                    let p = MoonPalette::active(cx);
                    style
                        .border_l(px(2.0))
                        .border_color(rgba_from(p.accent, 0.86))
                })
                .on_drop(move |drag: &MoonDataColumnDrag, _window, cx| {
                    if drag.state_id != state_id {
                        return;
                    }
                    state_for_drop.update(cx, |state, cx| {
                        if state.move_column_before(
                            &drag.key,
                            &target_key,
                            all_keys_for_drop.clone(),
                        ) {
                            cx.notify();
                        }
                    });
                });
        }

        if resizable {
            let drag = MoonDataColumnResizeDrag {
                state_id,
                key: key_string.clone(),
            };
            let state_for_move = state.clone();
            let state_for_reset = state.clone();
            let reset_key = key_string.clone();
            cell = cell.child(
                div()
                    .id(ElementId::from(SharedString::from(format!(
                        "{id}:resize:{key_string}"
                    ))))
                    .absolute()
                    .right(px(0.0))
                    .top(px(0.0))
                    .bottom(px(0.0))
                    .w(px(tokens.ui(6.0)))
                    .cursor(CursorStyle::ResizeColumn)
                    .hover(|this| this.bg(rgba_from(p.accent, 0.14)))
                    // Double-clicking a divider restores automatic width. A plain double-click
                    // resets only this column, so it rejoins auto-fill while other resized
                    // columns stay fixed. Shift+double-click resets every table width.
                    .on_click(move |event, _, cx| {
                        if event.click_count() >= 2 {
                            let full = event.modifiers().shift;
                            state_for_reset.update(cx, |state, cx| {
                                let changed = if full {
                                    let had = !state.column_widths.is_empty();
                                    state.column_widths.clear();
                                    had
                                } else {
                                    state.column_widths.remove(&reset_key).is_some()
                                };
                                if changed {
                                    cx.notify();
                                }
                            });
                        }
                    })
                    .on_drag(drag, |drag, _, _, cx| {
                        cx.stop_propagation();
                        cx.new(|_| drag.clone())
                    })
                    .on_drag_move(window.listener_for(
                        &state_for_move,
                        move |state, event: &DragMoveEvent<MoonDataColumnResizeDrag>, _, cx| {
                            let drag = event.drag(cx);
                            if drag.state_id != cx.entity_id() {
                                return;
                            }
                            if let Some(bounds) = state.header_bounds(&drag.key) {
                                // On the first resize, retain every column at its current
                                // rendered width from `header_bounds`. All columns then count as
                                // user-sized, so dragging one divider moves that column and the
                                // columns to its right without shifting left-side columns.
                                if state.column_widths.is_empty() {
                                    for (key, width) in state.header_widths() {
                                        state
                                            .column_widths
                                            .entry(key)
                                            .or_insert(width.max(MIN_COLUMN_WIDTH));
                                    }
                                }
                                let width = (f32::from(event.event.position.x)
                                    - f32::from(bounds.origin.x))
                                .max(MIN_COLUMN_WIDTH);
                                state.set_column_width(drag.key.clone(), width);
                                cx.notify();
                            }
                        },
                    )),
            );
        }

        header = header.child(cell);
    }

    header
}
