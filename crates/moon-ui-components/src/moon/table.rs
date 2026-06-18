use gpui::prelude::FluentBuilder;
use gpui::*;

use super::{
    text::MoonText,
    tokens::{MoonPalette, MoonTone, rgb_from, rgba_from},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoonTableAlign {
    Left,
    Right,
}

#[derive(Clone, Debug)]
pub struct MoonTableColumn {
    title: SharedString,
    width: f32,
    fill: bool,
    align: MoonTableAlign,
    header_pad_left: f32,
    header_pad_right: f32,
    cell_pad_left: f32,
    cell_pad_right: f32,
}

impl MoonTableColumn {
    pub fn new(title: impl Into<SharedString>, width: f32) -> Self {
        Self {
            title: title.into(),
            width,
            fill: false,
            align: MoonTableAlign::Left,
            header_pad_left: 10.0,
            header_pad_right: 8.0,
            cell_pad_left: 12.0,
            cell_pad_right: 8.0,
        }
    }

    pub fn right(mut self) -> Self {
        self.align = MoonTableAlign::Right;
        self
    }

    pub fn align(mut self, align: MoonTableAlign) -> Self {
        self.align = align;
        self
    }

    pub fn fill(mut self) -> Self {
        self.fill = true;
        self
    }

    pub fn header_padding(mut self, left: f32, right: f32) -> Self {
        self.header_pad_left = left;
        self.header_pad_right = right;
        self
    }

    pub fn cell_padding(mut self, left: f32, right: f32) -> Self {
        self.cell_pad_left = left;
        self.cell_pad_right = right;
        self
    }
}

pub struct MoonTableCell {
    content: MoonTableCellContent,
    tone: MoonTone,
    color: Option<u32>,
    weight: f32,
    font_size: f32,
    line_height: f32,
}

enum MoonTableCellContent {
    Text(SharedString),
    Element(AnyElement),
}

impl MoonTableCell {
    pub fn text(text: impl Into<SharedString>, tone: MoonTone, weight: f32) -> Self {
        Self {
            content: MoonTableCellContent::Text(text.into()),
            tone,
            color: None,
            weight,
            font_size: 10.5,
            line_height: 14.0,
        }
    }

    pub fn element(element: impl IntoElement + 'static) -> Self {
        Self {
            content: MoonTableCellContent::Element(element.into_any_element()),
            tone: MoonTone::Default,
            color: None,
            weight: 400.0,
            font_size: 10.5,
            line_height: 14.0,
        }
    }

    pub fn tone(mut self, tone: MoonTone) -> Self {
        self.tone = tone;
        self
    }

    pub fn text_color(mut self, color: u32) -> Self {
        self.color = Some(color);
        self
    }

    pub fn weight(mut self, weight: f32) -> Self {
        self.weight = weight;
        self
    }

    pub fn font_size(mut self, font_size: f32) -> Self {
        self.font_size = font_size;
        self
    }

    pub fn line_height(mut self, line_height: f32) -> Self {
        self.line_height = line_height;
        self
    }
}

pub struct MoonTableRow {
    cells: Vec<MoonTableCell>,
    selected: bool,
    text_alpha: f32,
}

impl Default for MoonTableRow {
    fn default() -> Self {
        Self::new()
    }
}

impl MoonTableRow {
    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
            selected: false,
            text_alpha: 1.0,
        }
    }

    pub fn cell(mut self, cell: MoonTableCell) -> Self {
        self.cells.push(cell);
        self
    }

    pub fn cells(mut self, cells: impl IntoIterator<Item = MoonTableCell>) -> Self {
        self.cells.extend(cells);
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn text_alpha(mut self, text_alpha: f32) -> Self {
        self.text_alpha = text_alpha;
        self
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MoonTableStyle {
    pub body_bg: u32,
    pub header_bg: u32,
    pub selected_bg: u32,
    pub selected_bar: u32,
    pub header_text: u32,
    pub header_separator: u32,
    pub header_separator_alpha: f32,
    pub selection_bar_width: f32,
}

impl Default for MoonTableStyle {
    fn default() -> Self {
        Self::for_palette(MoonPalette::TERMINAL)
    }
}

impl MoonTableStyle {
    pub fn for_palette(p: MoonPalette) -> Self {
        Self {
            body_bg: p.table_body,
            header_bg: p.table_head,
            selected_bg: p.table_selected,
            selected_bar: p.amber,
            header_text: p.text_muted,
            header_separator: p.border,
            header_separator_alpha: 1.0,
            selection_bar_width: 2.0,
        }
    }

    pub fn themed(self, p: MoonPalette) -> Self {
        let terminal = Self::for_palette(MoonPalette::TERMINAL);
        let themed = Self::for_palette(p);
        Self {
            body_bg: if self.body_bg == terminal.body_bg {
                themed.body_bg
            } else {
                self.body_bg
            },
            header_bg: if self.header_bg == terminal.header_bg {
                themed.header_bg
            } else {
                self.header_bg
            },
            selected_bg: if self.selected_bg == terminal.selected_bg {
                themed.selected_bg
            } else {
                self.selected_bg
            },
            selected_bar: if self.selected_bar == terminal.selected_bar {
                themed.selected_bar
            } else {
                self.selected_bar
            },
            header_text: if self.header_text == terminal.header_text {
                themed.header_text
            } else {
                self.header_text
            },
            header_separator: if self.header_separator == terminal.header_separator {
                themed.header_separator
            } else {
                self.header_separator
            },
            header_separator_alpha: self.header_separator_alpha,
            selection_bar_width: self.selection_bar_width,
        }
    }
}

pub(crate) struct MoonTable;

impl MoonTable {

    pub(crate) fn render_row_inline_with_cells(
        columns: &[MoonTableColumn],
        row: MoonTableRow,
        row_height: f32,
        style: MoonTableStyle,
        p: MoonPalette,
        mut decorate_cell: impl FnMut(usize, Div) -> AnyElement,
    ) -> Div {
        let mut row_el = div()
            .relative()
            .w_full()
            .h(px(row_height))
            .flex()
            .items_center()
            .bg(if row.selected {
                rgba_from(style.selected_bg, 1.0)
            } else {
                rgba_from(style.body_bg, 0.0)
            });

        if row.selected {
            row_el = row_el.child(
                div()
                    .absolute()
                    .left(px(0.0))
                    .top(px(0.0))
                    .w(px(style.selection_bar_width))
                    .h_full()
                    .bg(rgb_from(style.selected_bar)),
            );
        }

        for (column_ix, (column, cell)) in columns.iter().zip(row.cells).enumerate() {
            let cell = Self::render_cell(column, cell, row.text_alpha, p);
            row_el = row_el.child(decorate_cell(column_ix, cell));
        }

        row_el
    }

    fn render_cell(
        column: &MoonTableColumn,
        cell: MoonTableCell,
        text_alpha: f32,
        p: MoonPalette,
    ) -> Div {
        let justify_right = matches!(column.align, MoonTableAlign::Right);

        let mut el = div()
            .when(column.fill, |this| this.min_w(px(column.width)).flex_1())
            .when(!column.fill, |this| this.w(px(column.width)).flex_none())
            .h_full()
            .flex()
            .items_center()
            .when(justify_right, |this| this.justify_end())
            .when(!justify_right, |this| this.justify_start())
            .pl(px(column.cell_pad_left))
            .pr(px(column.cell_pad_right))
            .whitespace_nowrap();

        match cell.content {
            MoonTableCellContent::Text(text) => {
                let color = cell.color.unwrap_or_else(|| cell.tone.color(p));
                el = el.child(
                    MoonText::new(text)
                        .color(color)
                        .alpha(text_alpha)
                        .font_size(cell.font_size)
                        .line_height(cell.line_height)
                        .weight(cell.weight)
                        .mono(true)
                        .uppercase(false)
                        .render(),
                );
            }
            MoonTableCellContent::Element(element) => {
                el = el.child(element);
            }
        }

        el
    }
}

