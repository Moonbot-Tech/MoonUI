use gpui::prelude::FluentBuilder;
use gpui::*;

use super::{
    foundation::selected_background,
    text::MoonText,
    theme::MoonThemeTokens,
    tokens::{MoonPalette, MoonTone, rgb_from, rgba_from},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoonTableAlign {
    Left,
    Right,
}

#[derive(Clone, Debug)]
pub struct MoonTableColumn {
    width: f32,
    fill: bool,
    align: MoonTableAlign,
    header_pad_left: f32,
    header_pad_right: f32,
    cell_pad_left: f32,
    cell_pad_right: f32,
}

impl MoonTableColumn {
    pub fn new(_title: impl Into<SharedString>, width: f32) -> Self {
        Self {
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

    /// Base (unscaled) font size. The table scales it with `tokens.font()` at
    /// render time — pass design-reference values (e.g. `10.5`), never values that
    /// were already scaled, or the UI font scale gets applied twice.
    pub fn font_size(mut self, font_size: f32) -> Self {
        self.font_size = font_size;
        self
    }

    /// Base (unscaled) line height — scaled at render like [`Self::font_size`].
    pub fn line_height(mut self, line_height: f32) -> Self {
        self.line_height = line_height;
        self
    }
}

pub struct MoonTableRow {
    cells: Vec<MoonTableCell>,
    selected: bool,
    text_alpha: f32,
    banner: Option<AnyElement>,
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
            banner: None,
        }
    }

    /// Lay one element across the WHOLE row, above the cells and outside their clipping.
    ///
    /// Every cell is `overflow_hidden`, which is what keeps a long value from spilling into its
    /// neighbour — and is also why a row that wants to say ONE thing across its full width cannot
    /// say it through a cell: a section heading put in the leftmost cell is cut at that column's
    /// edge, however much empty room the rest of the row has. The banner is the escape hatch, and
    /// it is deliberately the only one: widening a cell's clipping would let ordinary values
    /// overlap.
    ///
    /// It is painted AFTER the cells, so it sits ON TOP of them VISUALLY. It does NOT take their
    /// pointer events: GPUI stops hit-testing at a hitbox only when that hitbox is
    /// `HitboxBehavior::BlockMouse` (`window.rs::hit_test` breaks on exactly that), and only
    /// `occlude()` / `occlude_mouse()` set it. This wrapper calls neither, so a cell underneath
    /// stays clickable and a banner is safe on a row whose cells are interactive.
    ///
    /// The consequence to design around is the OPPOSITE of occlusion: put a click handler on the
    /// banner AND on a cell beneath it and BOTH fire for one click. A caller that wants the banner
    /// to win calls `cx.stop_propagation()` in its own handler, or `occlude()`s its own element.
    ///
    /// Args:
    ///     banner: The element to lay across the row.
    ///
    /// Returns:
    ///     The row, carrying the banner.
    pub fn banner(mut self, banner: impl IntoElement) -> Self {
        self.banner = Some(banner.into_any_element());
        self
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

    /// Whether a banner was laid across this row.
    ///
    /// `pub(crate)` and a PREDICATE rather than a field: `MoonDataRow::as_table_row` forwards the
    /// banner across a module boundary, and nothing downstream of that forward is observable
    /// without a rendering harness — so the forward itself is untestable unless the receiving side
    /// can be asked. Exposing the field instead would let any module in the crate MOVE the element
    /// out of a row it does not own; a bool answers the only question a caller has.
    ///
    /// `allow(dead_code)` OUTSIDE a test build, and only there: the crate's own callers are all in
    /// `#[cfg(test)] mod tests`, which is stripped before dead-code analysis runs, so an ordinary
    /// `cargo build` would warn about a method that is doing its job. The attribute is conditional
    /// rather than blanket so that a REAL orphaning — the tests going away — still warns.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn has_banner(&self) -> bool {
        self.banner.is_some()
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
            selected_bg: p.accent,
            selected_bar: p.accent,
            header_text: p.text_muted,
            header_separator: p.border,
            header_separator_alpha: 1.0,
            selection_bar_width: 3.0,
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
        tokens: &MoonThemeTokens,
        mut decorate_cell: impl FnMut(usize, Div) -> AnyElement,
    ) -> Div {
        let row_bg: Background = if row.selected {
            selected_background(p)
        } else {
            rgba_from(style.body_bg, 0.0).into()
        };

        let mut row_el = div()
            .relative()
            .w_full()
            .h(px(row_height))
            .flex()
            .items_center()
            .bg(row_bg);

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
            let cell = Self::render_cell(column, cell, row.text_alpha, p, tokens);
            row_el = row_el.child(decorate_cell(column_ix, cell));
        }

        // LAST, so it paints over the cells rather than under them, and absolute so it spans the
        // row instead of joining the cell flex line. The row itself is `relative` and carries no
        // `overflow_hidden`, which is the whole reason a banner can reach past a column edge.
        //
        // Deliberately NOT `occlude()`d — see the builder's doc. Painting last decides what the
        // eye sees, never what the mouse reaches.
        if let Some(banner) = row.banner {
            row_el = row_el.child(
                div()
                    .absolute()
                    .left(px(0.0))
                    .top(px(0.0))
                    .size_full()
                    .child(banner),
            );
        }

        row_el
    }

    fn render_cell(
        column: &MoonTableColumn,
        cell: MoonTableCell,
        text_alpha: f32,
        p: MoonPalette,
        tokens: &MoonThemeTokens,
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
            .overflow_hidden()
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
                // Element cells inherit the cell's text style through the GPUI style
                // cascade — the same style the Text branch applies explicitly. Children
                // that set their own text properties (e.g. MoonText, MoonButton) still
                // override it, so consumers no longer have to duplicate the table's
                // default metrics inside every clickable cell.
                let color = cell.color.unwrap_or_else(|| cell.tone.color(p));
                let text_metrics = tokens.text(cell.font_size, cell.line_height);
                el = el
                    .font_family(tokens.font_family(true))
                    .text_size(px(text_metrics.font_size))
                    .line_height(px(text_metrics.line_height))
                    .font_weight(FontWeight(cell.weight))
                    .text_color(rgba_from(color, text_alpha))
                    .child(element);
            }
        }

        el
    }
}
