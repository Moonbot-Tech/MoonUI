use std::{collections::HashMap, rc::Rc};

use gpui::prelude::FluentBuilder;
use gpui::*;

use super::{
    background::MoonBackgroundPolicy,
    context_menu::MoonContextMenu,
    dropdown::MoonMenuItem,
    scroll_area::{MoonScrollAxis, MoonScrollbarVisibility, moon_scrollbar_overlay_with_palette},
    table::{MoonTableAlign, MoonTableCell, MoonTableColumn, MoonTableRow, MoonTableStyle},
    theme::MoonTheme,
    tokens::{MoonPalette, MoonRect, MoonTone, rgba_from},
    virtual_list::{MoonVirtualList, MoonVirtualListScrollHandle},
};

#[derive(Clone, Debug)]
struct MoonDataColumnResizeDrag {
    state_id: EntityId,
    key: String,
}

impl Render for MoonDataColumnResizeDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

#[derive(Clone, Debug)]
struct MoonDataColumnDrag {
    state_id: EntityId,
    key: SharedString,
}

impl Render for MoonDataColumnDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MoonDataTableContextTarget {
    Table,
    Row(usize),
    Column(usize),
    Cell(usize, usize),
}

#[derive(Clone, Debug)]
pub struct MoonDataTableContext {
    pub target: MoonDataTableContextTarget,
    pub position: Point<Pixels>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MoonDataTableEvent {
    SelectRow(usize),
    SelectColumn(usize),
    SelectCell(usize, usize),
    DoubleClickedRow(usize),
    DoubleClickedCell(usize, usize),
    RightClickedRow(Option<usize>),
    RightClickedColumn(usize),
    RightClickedCell(usize, usize),
}

type MoonDataRowHandler = Rc<dyn Fn(usize, &mut Window, &mut App)>;
type MoonDataColumnHandler = Rc<dyn Fn(usize, &mut Window, &mut App)>;
type MoonDataCellHandler = Rc<dyn Fn(usize, usize, &mut Window, &mut App)>;
type MoonDataContextMenuBuilder =
    Rc<dyn Fn(&MoonDataTableContextTarget, &mut Window, &mut App) -> Vec<MoonMenuItem>>;

#[derive(Clone)]
pub struct MoonDataTableColumn {
    pub key: SharedString,
    pub title: SharedString,
    /// Base column width in logical design pixels.
    ///
    /// `MoonDataTable` treats this value as both the minimum width and the
    /// proportional weight for auto-width layout. If the viewport is wider than
    /// the sum of all base widths, every column is multiplied by the same scale
    /// factor. If the viewport is narrower, base widths are preserved and the
    /// horizontal scrollbar owns the overflow.
    pub width: f32,
    pub fill: bool,
    pub align: MoonTableAlign,
    pub sortable: bool,
    pub resizable: bool,
    pub movable: bool,
    pub fixed_left: bool,
}

impl MoonDataTableColumn {
    pub fn new(key: impl Into<SharedString>, title: impl Into<SharedString>, width: f32) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            width,
            fill: false,
            align: MoonTableAlign::Left,
            sortable: false,
            resizable: true,
            movable: true,
            fixed_left: false,
        }
    }

    pub fn right(mut self) -> Self {
        self.align = MoonTableAlign::Right;
        self
    }

    pub fn fill(mut self) -> Self {
        self.fill = true;
        self
    }

    pub fn sortable(mut self, sortable: bool) -> Self {
        self.sortable = sortable;
        self
    }

    pub fn fixed_left(mut self) -> Self {
        self.fixed_left = true;
        self
    }

    fn as_table_column(&self) -> MoonTableColumn {
        let column = MoonTableColumn::new(self.title.clone(), self.width).align(self.align);
        if self.fill { column.fill() } else { column }
    }
}

pub struct MoonDataCell {
    cell: MoonTableCell,
}

impl MoonDataCell {
    pub fn text(text: impl Into<SharedString>) -> Self {
        Self {
            cell: MoonTableCell::text(text, MoonTone::Default, 400.0),
        }
    }

    pub fn element(element: impl IntoElement + 'static) -> Self {
        Self {
            cell: MoonTableCell::element(element),
        }
    }

    pub fn tone(mut self, tone: MoonTone) -> Self {
        self.cell = self.cell.tone(tone);
        self
    }

    pub fn text_color(mut self, color: u32) -> Self {
        self.cell = self.cell.text_color(color);
        self
    }

    pub fn weight(mut self, weight: f32) -> Self {
        self.cell = self.cell.weight(weight);
        self
    }

    pub fn font_size(mut self, font_size: f32) -> Self {
        self.cell = self.cell.font_size(font_size);
        self
    }

    pub fn line_height(mut self, line_height: f32) -> Self {
        self.cell = self.cell.line_height(line_height);
        self
    }
}

pub struct MoonDataRow {
    pub cells: Vec<MoonDataCell>,
    pub selected: bool,
}

impl MoonDataRow {
    pub fn new(cells: impl IntoIterator<Item = MoonDataCell>) -> Self {
        Self {
            cells: cells.into_iter().collect(),
            selected: false,
        }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn as_table_row(self) -> MoonTableRow {
        MoonTableRow::new()
            .selected(self.selected)
            .cells(self.cells.into_iter().map(|cell| cell.cell))
    }
}

pub struct MoonDataTableState {
    pub selected_row: Option<usize>,
    pub selected_column: Option<usize>,
    pub selected_cell: Option<(usize, usize)>,
    pub sort_column: Option<SharedString>,
    pub sort_ascending: bool,
    pub column_order: Vec<SharedString>,
    pub column_widths: HashMap<String, f32>,
    pub right_clicked_row: Option<usize>,
    pub right_clicked_column: Option<usize>,
    pub right_clicked_cell: Option<(usize, usize)>,
    pub context_menu: Option<MoonDataTableContext>,
    header_bounds: HashMap<String, Bounds<Pixels>>,
    horizontal_scroll_handle: ScrollHandle,
    viewport_width: f32,
}

impl EventEmitter<MoonDataTableEvent> for MoonDataTableState {}

impl MoonDataTableState {
    pub fn new() -> Self {
        Self {
            selected_row: None,
            selected_column: None,
            selected_cell: None,
            sort_column: None,
            sort_ascending: true,
            column_order: Vec::new(),
            column_widths: HashMap::new(),
            right_clicked_row: None,
            right_clicked_column: None,
            right_clicked_cell: None,
            context_menu: None,
            header_bounds: HashMap::new(),
            horizontal_scroll_handle: ScrollHandle::default(),
            viewport_width: 0.0,
        }
    }

    pub fn select_row(&mut self, row: Option<usize>, cx: &mut Context<Self>) {
        self.selected_row = row;
        if let Some(row) = row {
            let column = self.selected_column.unwrap_or(0);
            self.selected_cell = Some((row, column));
            cx.emit(MoonDataTableEvent::SelectRow(row));
        }
    }

    pub fn select_column(&mut self, column: Option<usize>, cx: &mut Context<Self>) {
        self.selected_column = column;
        if let Some(column) = column {
            let row = self.selected_row.unwrap_or(0);
            self.selected_cell = Some((row, column));
            cx.emit(MoonDataTableEvent::SelectColumn(column));
        }
    }

    pub fn select_cell(&mut self, row: usize, column: usize, cx: &mut Context<Self>) {
        self.selected_row = Some(row);
        self.selected_column = Some(column);
        self.selected_cell = Some((row, column));
        cx.emit(MoonDataTableEvent::SelectCell(row, column));
    }

    pub fn move_selection(
        &mut self,
        row_delta: isize,
        column_delta: isize,
        rows: usize,
        columns: usize,
        cx: &mut Context<Self>,
    ) {
        if let Some((row, column)) =
            self.apply_selection_delta(row_delta, column_delta, rows, columns)
        {
            cx.emit(MoonDataTableEvent::SelectCell(row, column));
        }
    }

    pub fn apply_selection_delta(
        &mut self,
        row_delta: isize,
        column_delta: isize,
        rows: usize,
        columns: usize,
    ) -> Option<(usize, usize)> {
        if rows == 0 || columns == 0 {
            self.selected_row = None;
            self.selected_column = None;
            self.selected_cell = None;
            return None;
        }

        let row = self
            .selected_row
            .unwrap_or(0)
            .saturating_add_signed(row_delta)
            .min(rows - 1);
        let column = self
            .selected_column
            .unwrap_or(0)
            .saturating_add_signed(column_delta)
            .min(columns - 1);
        self.selected_row = Some(row);
        self.selected_column = Some(column);
        self.selected_cell = Some((row, column));
        Some((row, column))
    }

    pub fn horizontal_scroll_handle(&self) -> ScrollHandle {
        self.horizontal_scroll_handle.clone()
    }

    pub fn viewport_width(&self) -> f32 {
        self.viewport_width
    }

    pub fn set_viewport_width(&mut self, width: f32) -> bool {
        let width = width.max(0.0);
        if (self.viewport_width - width).abs() <= 0.5 {
            return false;
        }
        self.viewport_width = width;
        true
    }

    pub fn open_context_menu(
        &mut self,
        target: MoonDataTableContextTarget,
        position: Point<Pixels>,
    ) {
        self.context_menu = Some(MoonDataTableContext {
            target: target.clone(),
            position,
        });
        self.right_clicked_row = None;
        self.right_clicked_column = None;
        self.right_clicked_cell = None;
        match target {
            MoonDataTableContextTarget::Table => {}
            MoonDataTableContextTarget::Row(row) => {
                self.right_clicked_row = Some(row);
            }
            MoonDataTableContextTarget::Column(column) => {
                self.right_clicked_column = Some(column);
            }
            MoonDataTableContextTarget::Cell(row, column) => {
                self.right_clicked_cell = Some((row, column));
            }
        }
    }

    pub fn close_context_menu(&mut self) {
        self.context_menu = None;
        self.right_clicked_row = None;
        self.right_clicked_column = None;
        self.right_clicked_cell = None;
    }

    pub fn set_sort(&mut self, column: impl Into<SharedString>, ascending: bool) {
        self.sort_column = Some(column.into());
        self.sort_ascending = ascending;
    }

    pub fn toggle_sort(&mut self, column: impl Into<SharedString>) {
        let column = column.into();
        if self.sort_column.as_ref() == Some(&column) {
            self.sort_ascending = !self.sort_ascending;
        } else {
            self.sort_column = Some(column);
            self.sort_ascending = true;
        }
    }

    pub fn set_column_width(&mut self, key: impl Into<String>, width: f32) {
        self.column_widths.insert(key.into(), width.max(40.0));
    }

    pub fn set_column_order(&mut self, order: impl IntoIterator<Item = SharedString>) {
        self.column_order = order.into_iter().collect();
    }

    pub fn move_column_before(
        &mut self,
        source: &SharedString,
        target: &SharedString,
        all_columns: impl IntoIterator<Item = SharedString>,
    ) -> bool {
        if source == target {
            return false;
        }
        if self.column_order.is_empty() {
            self.column_order = all_columns.into_iter().collect();
        }

        let Some(source_ix) = self.column_order.iter().position(|key| key == source) else {
            return false;
        };
        let source_key = self.column_order.remove(source_ix);
        let Some(mut target_ix) = self.column_order.iter().position(|key| key == target) else {
            self.column_order.push(source_key);
            return true;
        };
        if source_ix < target_ix {
            target_ix = target_ix.saturating_sub(1);
        }
        self.column_order.insert(target_ix, source_key);
        true
    }

    fn header_bounds(&self, key: &str) -> Option<Bounds<Pixels>> {
        self.header_bounds.get(key).copied()
    }
}

impl Default for MoonDataTableState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(IntoElement)]
pub struct MoonDataTable {
    id: SharedString,
    bounds: Option<MoonRect>,
    columns: Vec<MoonDataTableColumn>,
    row_count: usize,
    row_height: f32,
    header_height: f32,
    state: Option<Entity<MoonDataTableState>>,
    render_row: Rc<dyn Fn(usize, &mut Window, &mut App) -> MoonDataRow>,
    scroll_handle: Option<MoonVirtualListScrollHandle>,
    style: Option<MoonTableStyle>,
    background_policy: MoonBackgroundPolicy,
    cell_selectable: bool,
    column_selectable: bool,
    row_header: bool,
    row_header_width: f32,
    on_select_row: Option<MoonDataRowHandler>,
    on_double_click_row: Option<MoonDataRowHandler>,
    on_right_click_row: Option<MoonDataRowHandler>,
    on_select_column: Option<MoonDataColumnHandler>,
    on_right_click_column: Option<MoonDataColumnHandler>,
    on_select_cell: Option<MoonDataCellHandler>,
    on_double_click_cell: Option<MoonDataCellHandler>,
    on_right_click_cell: Option<MoonDataCellHandler>,
    context_menu_builder: Option<MoonDataContextMenuBuilder>,
    on_sort: Option<Rc<dyn Fn(&SharedString, bool, &mut Window, &mut App)>>,
}

impl MoonDataTable {
    pub fn new(
        id: impl Into<SharedString>,
        row_count: usize,
        render_row: impl Fn(usize, &mut Window, &mut App) -> MoonDataRow + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            bounds: None,
            columns: Vec::new(),
            row_count,
            row_height: 25.0,
            header_height: 26.0,
            state: None,
            render_row: Rc::new(render_row),
            scroll_handle: None,
            style: None,
            background_policy: MoonBackgroundPolicy::Opaque,
            cell_selectable: false,
            column_selectable: true,
            row_header: false,
            row_header_width: 28.0,
            on_select_row: None,
            on_double_click_row: None,
            on_right_click_row: None,
            on_select_column: None,
            on_right_click_column: None,
            on_select_cell: None,
            on_double_click_cell: None,
            on_right_click_cell: None,
            context_menu_builder: None,
            on_sort: None,
        }
    }

    pub fn bounds(mut self, bounds: MoonRect) -> Self {
        self.bounds = Some(bounds);
        self
    }

    pub fn columns(mut self, columns: impl IntoIterator<Item = MoonDataTableColumn>) -> Self {
        self.columns = columns.into_iter().collect();
        self
    }

    pub fn state(mut self, state: &Entity<MoonDataTableState>) -> Self {
        self.state = Some(state.clone());
        self
    }

    pub fn row_height(mut self, row_height: f32) -> Self {
        self.row_height = row_height;
        self
    }

    pub fn header_height(mut self, header_height: f32) -> Self {
        self.header_height = header_height;
        self
    }

    pub fn track_scroll(mut self, handle: &MoonVirtualListScrollHandle) -> Self {
        self.scroll_handle = Some(handle.clone());
        self
    }

    pub fn style(mut self, style: MoonTableStyle) -> Self {
        self.style = Some(style);
        self
    }

    pub fn background_policy(mut self, policy: MoonBackgroundPolicy) -> Self {
        self.background_policy = policy;
        self
    }

    pub fn cell_selectable(mut self, selectable: bool) -> Self {
        self.cell_selectable = selectable;
        self
    }

    pub fn column_selectable(mut self, selectable: bool) -> Self {
        self.column_selectable = selectable;
        self
    }

    pub fn row_header(mut self, row_header: bool) -> Self {
        self.row_header = row_header;
        self
    }

    pub fn row_header_width(mut self, width: f32) -> Self {
        self.row_header_width = width.max(18.0);
        self
    }

    pub fn on_select_row(
        mut self,
        handler: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select_row = Some(Rc::new(handler));
        self
    }

    pub fn on_double_click_row(
        mut self,
        handler: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_double_click_row = Some(Rc::new(handler));
        self
    }

    pub fn on_right_click_row(
        mut self,
        handler: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_right_click_row = Some(Rc::new(handler));
        self
    }

    pub fn on_select_column(
        mut self,
        handler: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select_column = Some(Rc::new(handler));
        self
    }

    pub fn on_right_click_column(
        mut self,
        handler: impl Fn(usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_right_click_column = Some(Rc::new(handler));
        self
    }

    pub fn on_select_cell(
        mut self,
        handler: impl Fn(usize, usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select_cell = Some(Rc::new(handler));
        self
    }

    pub fn on_double_click_cell(
        mut self,
        handler: impl Fn(usize, usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_double_click_cell = Some(Rc::new(handler));
        self
    }

    pub fn on_right_click_cell(
        mut self,
        handler: impl Fn(usize, usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_right_click_cell = Some(Rc::new(handler));
        self
    }

    pub fn context_menu(
        mut self,
        builder: impl Fn(&MoonDataTableContextTarget, &mut Window, &mut App) -> Vec<MoonMenuItem>
        + 'static,
    ) -> Self {
        self.context_menu_builder = Some(Rc::new(builder));
        self
    }

    pub fn on_sort(
        mut self,
        handler: impl Fn(&SharedString, bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_sort = Some(Rc::new(handler));
        self
    }

    fn ordered_columns(
        columns: Vec<MoonDataTableColumn>,
        state: &MoonDataTableState,
    ) -> Vec<MoonDataTableColumn> {
        let mut ordered = if state.column_order.is_empty() {
            columns
        } else {
            let mut by_key = columns
                .iter()
                .cloned()
                .map(|column| (column.key.to_string(), column))
                .collect::<HashMap<_, _>>();
            let mut ordered = Vec::new();
            for key in &state.column_order {
                if let Some(column) = by_key.remove(key.as_ref()) {
                    ordered.push(column);
                }
            }
            for column in columns {
                if by_key.remove(column.key.as_ref()).is_some() {
                    ordered.push(column);
                }
            }
            ordered
        };

        ordered.sort_by_key(|column| !column.fixed_left);
        ordered
            .into_iter()
            .map(|mut column| {
                if let Some(width) = state.column_widths.get(column.key.as_ref()) {
                    column.width = *width;
                }
                column
            })
            .collect()
    }

    fn auto_width_columns(
        mut columns: Vec<MoonDataTableColumn>,
        viewport_width: f32,
        row_header_width: f32,
    ) -> Vec<MoonDataTableColumn> {
        let base_width = columns.iter().map(|column| column.width).sum::<f32>();
        let available = (viewport_width - row_header_width).max(0.0);
        if base_width > 0.0 && available > base_width {
            let scale = available / base_width;
            for column in &mut columns {
                column.width *= scale;
                column.fill = false;
            }
        }
        columns
    }

    fn render_header(
        id: &SharedString,
        columns: &[MoonDataTableColumn],
        state: &Entity<MoonDataTableState>,
        height: f32,
        left_offset: f32,
        style: MoonTableStyle,
        column_selectable: bool,
        on_select_column: Option<MoonDataColumnHandler>,
        on_right_click_column: Option<MoonDataColumnHandler>,
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
                                state.header_bounds.insert(key_string.clone(), bounds);
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
                        if column_selectable {
                            state.update(cx, |state, cx| {
                                state.select_column(Some(column_ix), cx);
                                cx.notify();
                            });
                            if let Some(on_select_column) = &on_select_column {
                                on_select_column(column_ix, window, cx);
                            }
                        }
                        if sortable {
                            state.update(cx, |state, _| state.toggle_sort(key.clone()));
                            let ascending = state.read(cx).sort_ascending;
                            if let Some(on_sort) = &on_sort {
                                on_sort(&key, ascending, window, cx);
                            }
                        }
                    });
            }

            {
                let state = state.clone();
                let on_right_click_column = on_right_click_column.clone();
                cell = cell.on_mouse_down(MouseButton::Right, move |event, window, cx| {
                    state.update(cx, |state, cx| {
                        state.open_context_menu(
                            MoonDataTableContextTarget::Column(column_ix),
                            event.position,
                        );
                        cx.emit(MoonDataTableEvent::RightClickedColumn(column_ix));
                        cx.notify();
                    });
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
                            .border_color(rgba_from(p.blue, 0.86))
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
                        .hover(|this| this.bg(rgba_from(p.blue, 0.14)))
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
                                    let width = (f32::from(event.event.position.x)
                                        - f32::from(bounds.origin.x))
                                    .max(40.0);
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
}

impl RenderOnce for MoonDataTable {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let p = MoonPalette::active(cx);
        let tokens = MoonTheme::active_tokens(cx);
        let style = self
            .style
            .unwrap_or_else(|| MoonTableStyle::for_palette(p))
            .themed(p);
        let state = self.state.unwrap_or_else(|| {
            window.use_keyed_state(
                ElementId::from(SharedString::from(format!("{}:state", self.id))),
                cx,
                |_, _| MoonDataTableState::default(),
            )
        });
        let render_row = self.render_row.clone();
        let row_height = tokens.fit_height(self.row_height, 14.0, 5.5);
        let header_height = tokens.fit_height(self.header_height, 11.0, 7.5);
        let id = self.id.clone();
        let state_for_rows = state.clone();
        let background_policy = self.background_policy;
        let cell_selectable = self.cell_selectable;
        let column_selectable = self.column_selectable;
        let row_header = self.row_header;
        let row_header_width = if row_header {
            tokens.ui(self.row_header_width)
        } else {
            0.0
        };
        let horizontal_scroll_handle = state.read(cx).horizontal_scroll_handle();
        let viewport_from_scroll =
            f32::from(horizontal_scroll_handle.bounds().size.width).max(0.0);
        let viewport_from_state = state.read(cx).viewport_width();
        let viewport_width = viewport_from_scroll.max(viewport_from_state);
        let columns = Self::auto_width_columns(
            Self::ordered_columns(self.columns, state.read(cx)),
            viewport_width,
            row_header_width,
        );
        let table_columns = columns
            .iter()
            .map(MoonDataTableColumn::as_table_column)
            .collect::<Vec<_>>();
        let row_columns = table_columns.clone();
        let columns_min_width = columns.iter().map(|column| column.width).sum::<f32>();
        let table_min_width = (columns_min_width + row_header_width).max(1.0);
        let on_select_row = self.on_select_row.clone();
        let on_double_click_row = self.on_double_click_row.clone();
        let on_right_click_row = self.on_right_click_row.clone();
        let on_select_column = self.on_select_column.clone();
        let on_right_click_column = self.on_right_click_column.clone();
        let on_select_cell = self.on_select_cell.clone();
        let on_double_click_cell = self.on_double_click_cell.clone();
        let on_right_click_cell = self.on_right_click_cell.clone();
        let context_menu_builder = self.context_menu_builder.clone();
        let on_sort = self.on_sort.clone();
        let scroll_handle = self.scroll_handle.unwrap_or_else(|| {
            window
                .use_keyed_state(
                    ElementId::from(SharedString::from(format!("{id}:scroll"))),
                    cx,
                    |_, _| MoonVirtualListScrollHandle::new(),
                )
                .read(cx)
                .clone()
        });
        let focus_handle = window
            .use_keyed_state(
                ElementId::from(SharedString::from(format!("{id}:focus"))),
                cx,
                |_, cx| cx.focus_handle().tab_stop(true),
            )
            .read(cx)
            .clone();
        let row_count = self.row_count;
        let column_count = columns.len();
        let keyboard_state = state.clone();
        let keyboard_scroll = scroll_handle.clone();
        let table_context_state = state.clone();
        let mut root = background_policy
            .apply(
                div()
                    .id(ElementId::from(id.clone()))
                    .relative()
                    .overflow_hidden(),
                style.body_bg,
                1.0,
            )
            .track_focus(&focus_handle)
            .on_mouse_down(MouseButton::Right, move |event, _window, cx| {
                table_context_state.update(cx, |state, cx| {
                    state.open_context_menu(MoonDataTableContextTarget::Table, event.position);
                    cx.emit(MoonDataTableEvent::RightClickedRow(None));
                    cx.notify();
                });
            })
            .on_key_down(move |event, window, cx| {
                let key = event.keystroke.key.as_str();
                let page_rows = {
                    let viewport_h =
                        f32::from(keyboard_scroll.0.borrow().base_handle.bounds().size.height);
                    (viewport_h / row_height).floor().max(1.0) as isize
                };
                let mut selected_row = None;
                let handled = match key {
                    "down" => {
                        keyboard_state.update(cx, |state, cx| {
                            state.move_selection(1, 0, row_count, column_count, cx);
                            selected_row = state.selected_row;
                            cx.notify();
                        });
                        true
                    }
                    "up" => {
                        keyboard_state.update(cx, |state, cx| {
                            state.move_selection(-1, 0, row_count, column_count, cx);
                            selected_row = state.selected_row;
                            cx.notify();
                        });
                        true
                    }
                    "right" => {
                        keyboard_state.update(cx, |state, cx| {
                            state.move_selection(0, 1, row_count, column_count, cx);
                            selected_row = state.selected_row;
                            cx.notify();
                        });
                        true
                    }
                    "left" => {
                        keyboard_state.update(cx, |state, cx| {
                            state.move_selection(0, -1, row_count, column_count, cx);
                            selected_row = state.selected_row;
                            cx.notify();
                        });
                        true
                    }
                    "home" => {
                        keyboard_state.update(cx, |state, cx| {
                            let column = state.selected_column.unwrap_or(0);
                            state.select_cell(0, column.min(column_count.saturating_sub(1)), cx);
                            selected_row = state.selected_row;
                            cx.notify();
                        });
                        true
                    }
                    "end" => {
                        keyboard_state.update(cx, |state, cx| {
                            let column = state.selected_column.unwrap_or(0);
                            state.select_cell(
                                row_count.saturating_sub(1),
                                column.min(column_count.saturating_sub(1)),
                                cx,
                            );
                            selected_row = state.selected_row;
                            cx.notify();
                        });
                        true
                    }
                    "pageup" | "page_up" => {
                        keyboard_state.update(cx, |state, cx| {
                            state.move_selection(-page_rows, 0, row_count, column_count, cx);
                            selected_row = state.selected_row;
                            cx.notify();
                        });
                        true
                    }
                    "pagedown" | "page_down" => {
                        keyboard_state.update(cx, |state, cx| {
                            state.move_selection(page_rows, 0, row_count, column_count, cx);
                            selected_row = state.selected_row;
                            cx.notify();
                        });
                        true
                    }
                    _ => false,
                };

                if handled {
                    if let Some(row) = selected_row {
                        keyboard_scroll.scroll_to_item(row, ScrollStrategy::Nearest);
                    }
                    window.prevent_default();
                    cx.stop_propagation();
                }
            });
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

        let rows_id = id.clone();
        let mut rows_list = MoonVirtualList::new(
            SharedString::from(format!("{id}:rows")),
            self.row_count,
            row_height,
            move |ix, window, cx| {
                let mut row = (render_row)(ix, window, cx);
                let selected_row = state_for_rows.read(cx).selected_row == Some(ix);
                let selected_cell = state_for_rows.read(cx).selected_cell;
                row.selected = row.selected || selected_row;

                let state_for_row_click = state_for_rows.clone();
                let state_for_row_right = state_for_rows.clone();
                let on_select_row = on_select_row.clone();
                let on_double_click_row = on_double_click_row.clone();
                let on_right_click_row = on_right_click_row.clone();
                let on_select_cell = on_select_cell.clone();
                let on_double_click_cell = on_double_click_cell.clone();
                let on_right_click_cell = on_right_click_cell.clone();
                let rows_id_for_cell = rows_id.clone();
                let state_for_cells = state_for_rows.clone();

                let row_content = super::table::MoonTable::render_row_inline_with_cells(
                    &row_columns,
                    row.as_table_row(),
                    row_height,
                    style,
                    p,
                    move |column_ix, cell| {
                        let mut cell = cell
                            .id(ElementId::from(SharedString::from(format!(
                                "{rows_id_for_cell}:cell:{ix}:{column_ix}"
                            ))))
                            .when(selected_cell == Some((ix, column_ix)), |this| {
                                this.bg(rgba_from(p.panel_high, 0.58))
                            });

                        if cell_selectable {
                            let state_for_cell_click = state_for_cells.clone();
                            let state_for_cell_right = state_for_cells.clone();
                            let on_select_cell = on_select_cell.clone();
                            let on_double_click_cell = on_double_click_cell.clone();
                            let on_right_click_cell = on_right_click_cell.clone();
                            cell = cell
                                .cursor_pointer()
                                .on_click(move |event, window, cx| {
                                    state_for_cell_click.update(cx, |state, cx| {
                                        state.select_cell(ix, column_ix, cx);
                                        if event.click_count() == 2 {
                                            cx.emit(MoonDataTableEvent::DoubleClickedCell(
                                                ix, column_ix,
                                            ));
                                        }
                                        cx.notify();
                                    });
                                    if let Some(on_select_cell) = &on_select_cell {
                                        on_select_cell(ix, column_ix, window, cx);
                                    }
                                    if event.click_count() == 2
                                        && let Some(on_double_click_cell) = &on_double_click_cell
                                    {
                                        on_double_click_cell(ix, column_ix, window, cx);
                                    }
                                    cx.stop_propagation();
                                })
                                .on_mouse_down(MouseButton::Right, move |event, window, cx| {
                                    state_for_cell_right.update(cx, |state, cx| {
                                        state.open_context_menu(
                                            MoonDataTableContextTarget::Cell(ix, column_ix),
                                            event.position,
                                        );
                                        cx.emit(MoonDataTableEvent::RightClickedCell(
                                            ix, column_ix,
                                        ));
                                        cx.notify();
                                    });
                                    if let Some(on_right_click_cell) = &on_right_click_cell {
                                        on_right_click_cell(ix, column_ix, window, cx);
                                    }
                                    cx.stop_propagation();
                                });
                        }

                        cell.into_any_element()
                    },
                )
                .absolute()
                .left(px(row_header_width))
                .top(px(0.0))
                .right(px(0.0));

                let mut row_el = div()
                    .id(ElementId::from(SharedString::from(format!(
                        "{rows_id}:row:{ix}"
                    ))))
                    .relative()
                    .w_full()
                    .h(px(row_height))
                    .cursor_pointer()
                    .hover(|this| this.bg(rgba_from(p.panel_high, 0.28)))
                    .on_click({
                        let on_select_row = on_select_row.clone();
                        move |event, window, cx| {
                            state_for_row_click.update(cx, |state, cx| {
                                state.select_row(Some(ix), cx);
                                if event.click_count() == 2 {
                                    cx.emit(MoonDataTableEvent::DoubleClickedRow(ix));
                                }
                                cx.notify();
                            });
                            if let Some(on_select_row) = &on_select_row {
                                on_select_row(ix, window, cx);
                            }
                            if event.click_count() == 2
                                && let Some(on_double_click_row) = &on_double_click_row
                            {
                                on_double_click_row(ix, window, cx);
                            }
                        }
                    })
                    .on_mouse_down(MouseButton::Right, move |event, window, cx| {
                        state_for_row_right.update(cx, |state, cx| {
                            state.open_context_menu(
                                MoonDataTableContextTarget::Row(ix),
                                event.position,
                            );
                            cx.emit(MoonDataTableEvent::RightClickedRow(Some(ix)));
                            cx.notify();
                        });
                        if let Some(on_right_click_row) = &on_right_click_row {
                            on_right_click_row(ix, window, cx);
                        }
                        cx.stop_propagation();
                    });

                if row_header {
                    let state_for_header_click = state_for_rows.clone();
                    let on_select_row = on_select_row.clone();
                    row_el = row_el.child(
                        div()
                            .id(ElementId::from(SharedString::from(format!(
                                "{rows_id}:row-header:{ix}"
                            ))))
                            .absolute()
                            .left(px(0.0))
                            .top(px(0.0))
                            .w(px(row_header_width))
                            .h(px(row_height))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(tokens.font(9.0)))
                            .line_height(px(tokens.line_height(11.0)))
                            .text_color(rgba_from(p.text_muted, 1.0))
                            .border_r(px(1.0))
                            .border_color(rgba_from(
                                style.header_separator,
                                style.header_separator_alpha,
                            ))
                            .bg(rgba_from(
                                if selected_row {
                                    style.selected_bg
                                } else {
                                    style.body_bg
                                },
                                if selected_row { 1.0 } else { 0.0 },
                            ))
                            .child((ix + 1).to_string())
                            .on_click(move |_, window, cx| {
                                state_for_header_click.update(cx, |state, cx| {
                                    state.select_row(Some(ix), cx);
                                    cx.notify();
                                });
                                if let Some(on_select_row) = &on_select_row {
                                    on_select_row(ix, window, cx);
                                }
                                cx.stop_propagation();
                            }),
                    );
                }

                row_el.child(row_content)
            },
        )
        .surface(true)
        .background_policy(background_policy)
        .border(false)
        .radius(0.0)
        .tail_fill_color(style.body_bg);
        rows_list = rows_list.track_scroll(&scroll_handle);

        let mut content = div()
            .id(ElementId::from(SharedString::from(format!("{id}:content"))))
            .relative()
            .min_w(px(table_min_width))
            .w_full()
            .h_full()
            .child(Self::render_header(
                &id,
                &columns,
                &state,
                header_height,
                row_header_width,
                style,
                column_selectable,
                on_select_column,
                on_right_click_column,
                on_sort,
                window,
                cx,
            ))
            .child(
                div()
                    .absolute()
                    .left(px(0.0))
                    .top(px(header_height))
                    .right(px(0.0))
                    .bottom(px(0.0))
                    .child(rows_list),
            );

        if row_header {
            content = content.child(
                div()
                    .id(ElementId::from(SharedString::from(format!(
                        "{id}:row-header-corner"
                    ))))
                    .absolute()
                    .left(px(0.0))
                    .top(px(0.0))
                    .w(px(row_header_width))
                    .h(px(header_height))
                    .bg(rgba_from(style.header_bg, 1.0))
                    .border_b(px(1.0))
                    .border_r(px(1.0))
                    .border_color(rgba_from(
                        style.header_separator,
                        style.header_separator_alpha,
                    )),
            );
        }

        root = root.child(
            div()
                .id(ElementId::from(SharedString::from(format!(
                    "{id}:x-scroll"
                ))))
                .absolute()
                .left(px(0.0))
                .top(px(0.0))
                .right(px(0.0))
                .bottom(px(0.0))
                .overflow_x_scroll()
                .track_scroll(&horizontal_scroll_handle)
                .child(
                    canvas(
                        {
                            let state = state.clone();
                            move |bounds, _, cx| {
                                state.update(cx, |state, cx| {
                                    if state.set_viewport_width(f32::from(bounds.size.width)) {
                                        cx.notify();
                                    }
                                });
                            }
                        },
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .size_full(),
                )
                .child(content),
        );

        if let Some(scrollbar) = moon_scrollbar_overlay_with_palette(
            SharedString::from(format!("{id}:h-scrollbar")),
            &horizontal_scroll_handle,
            MoonScrollAxis::Horizontal,
            MoonScrollbarVisibility::Hover,
            p,
            window,
            cx,
        ) {
            root = root.child(scrollbar);
        }

        if let (Some(builder), Some(context)) = (
            context_menu_builder.as_ref(),
            state.read(cx).context_menu.clone(),
        ) {
            let items = builder(&context.target, window, cx);
            root = root.child(
                MoonContextMenu::new(format!("{id}:context-menu"))
                    .bounds(MoonRect {
                        x: f32::from(context.position.x),
                        y: f32::from(context.position.y),
                        w: 0.0,
                        h: 0.0,
                    })
                    .items(items)
                    .open(true)
                    .render(window, cx),
            );
        }

        root
    }
}
