//! Selectable, scrollable table (`Table`, `Row`, `TableState`).
//!
//! Columns are sized by the same [`Constraint`] solver used everywhere else, with an
//! optional header and footer row, per-column alignment, and a highlighted selection that
//! lives in a [`TableState`] owned by the application. Rendering never allocates.

use crate::core::buffer::Buffer;
use crate::core::grapheme::display_width;
use crate::core::layout::{Constraint, Layout};
use crate::core::rect::{Rect, TerminalHeight, TerminalWidth};
use crate::core::style::Style;
use crate::core::text::{Line, TextAlignment};
use crate::core::widget::StatefulWidget;
use crate::widgets::list::{ListState, follow_selection};

/// Selection and scroll position of a [`Table`] (same behavior as [`ListState`]).
pub type TableState = ListState;

/// One row of a [`Table`]: a list of single-line cells.
///
/// # Examples
///
/// ```
/// use dashboard_tui::widgets::Row;
///
/// let row = Row::new(["backup.tar", "UPLOAD", "78%"]);
/// assert_eq!(row.cell_count(), 3);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Row<'a> {
    cells: Vec<Line<'a>>,
    style: Style,
}

impl<'a> Row<'a> {
    /// Creates a row from cell contents (`&str`, `String`, [`Line`], …).
    #[must_use]
    pub fn new<T>(cells: impl IntoIterator<Item = T>) -> Self
    where
        T: Into<Line<'a>>,
    {
        Self {
            cells: cells.into_iter().map(Into::into).collect(),
            style: Style::DEFAULT,
        }
    }

    /// Sets the base style of the whole row.
    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Returns the number of cells.
    #[must_use]
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }
}

impl<'a, T: Into<Line<'a>>> FromIterator<T> for Row<'a> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::new(iter)
    }
}

/// A table with optional header / footer, column constraints, alignment and selection.
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::{Buffer, Constraint, Rect, Style, TextAlignment};
/// use dashboard_tui::widgets::{Row, Table, TableState};
///
/// let area = Rect::from_xywh(0, 0, 24, 4);
/// let mut buffer = Buffer::empty(area);
/// let mut state = TableState::default().with_selected(Some(1));
/// let table = Table::new(
///     [Row::new(["db.tar", "UP"]), Row::new(["spec.pdf", "DOWN"])],
///     [Constraint::Fill(1), Constraint::Fixed(6)],
/// )
/// .header(Row::new(["File", "Mode"]).style(Style::new().bold()))
/// .column_alignment(1, TextAlignment::Right)
/// .highlight_symbol("> ");
/// buffer.render_stateful_widget(table, area, &mut state);
///
/// let lines = buffer.to_visual_lines();
/// assert!(lines[0].contains("File"));
/// assert!(lines[2].starts_with("> spec.pdf"));
/// assert!(lines[2].trim_end().ends_with("DOWN"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table<'a> {
    rows: Vec<Row<'a>>,
    header: Option<Row<'a>>,
    footer: Option<Row<'a>>,
    widths: Vec<Constraint>,
    column_spacing: u16,
    layout: Layout,
    alignments: Vec<TextAlignment>,
    style: Style,
    header_style: Style,
    footer_style: Style,
    highlight_style: Style,
    highlight_symbol: &'a str,
    scroll_padding: usize,
}

impl<'a> Table<'a> {
    /// Creates a table from rows and column [`Constraint`]s (empty `widths` means equal columns).
    #[must_use]
    pub fn new<T>(
        rows: impl IntoIterator<Item = T>,
        widths: impl IntoIterator<Item = Constraint>,
    ) -> Self
    where
        T: Into<Row<'a>>,
    {
        let rows: Vec<Row<'a>> = rows.into_iter().map(Into::into).collect();
        let mut widths: Vec<Constraint> = widths.into_iter().collect();
        if widths.is_empty() {
            let columns = rows.iter().map(Row::cell_count).max().unwrap_or(0);
            widths = vec![Constraint::Fill(1); columns];
        }
        let column_spacing = 1;
        Self {
            layout: Layout::horizontal(widths.clone()).with_gap(column_spacing),
            rows,
            header: None,
            footer: None,
            widths,
            column_spacing,
            alignments: Vec::new(),
            style: Style::DEFAULT,
            header_style: Style::DEFAULT.bold(),
            footer_style: Style::DEFAULT.dim(),
            highlight_style: Style::DEFAULT.reversed(),
            highlight_symbol: "",
            scroll_padding: 0,
        }
    }

    /// Adds a header row drawn above the body.
    #[must_use]
    pub fn header(mut self, header: Row<'a>) -> Self {
        self.header = Some(header);
        self
    }

    /// Adds a footer row drawn below the body.
    #[must_use]
    pub fn footer(mut self, footer: Row<'a>) -> Self {
        self.footer = Some(footer);
        self
    }

    /// Sets the gap between columns (default 1).
    #[must_use]
    pub fn column_spacing(mut self, spacing: u16) -> Self {
        self.column_spacing = spacing;
        self.layout = Layout::horizontal(self.widths.clone()).with_gap(spacing);
        self
    }

    /// Aligns column `column` (`Left` default).
    #[must_use]
    pub fn column_alignment(mut self, column: usize, alignment: TextAlignment) -> Self {
        if self.alignments.len() <= column {
            self.alignments.resize(column + 1, TextAlignment::Left);
        }
        self.alignments[column] = alignment;
        self
    }

    /// Sets the base style behind the whole table.
    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Sets the header row style (default: bold).
    #[must_use]
    pub const fn header_style(mut self, style: Style) -> Self {
        self.header_style = style;
        self
    }

    /// Sets the footer row style (default: dim).
    #[must_use]
    pub const fn footer_style(mut self, style: Style) -> Self {
        self.footer_style = style;
        self
    }

    /// Sets the selected row's style (default: reversed).
    #[must_use]
    pub const fn highlight_style(mut self, style: Style) -> Self {
        self.highlight_style = style;
        self
    }

    /// Sets the marker drawn before the selected row; space is reserved on every row while a
    /// row is selected.
    #[must_use]
    pub const fn highlight_symbol(mut self, symbol: &'a str) -> Self {
        self.highlight_symbol = symbol;
        self
    }

    /// Keeps `rows` rows of context between the selection and the viewport edge.
    #[must_use]
    pub const fn scroll_padding(mut self, rows: usize) -> Self {
        self.scroll_padding = rows;
        self
    }

    fn column_alignment_of(&self, column: usize) -> TextAlignment {
        self.alignments
            .get(column)
            .copied()
            .unwrap_or(TextAlignment::Left)
    }

    fn render_row(
        &self,
        buffer: &mut Buffer,
        columns: &[Rect],
        row_index: crate::core::coordinates::RowIndex,
        row: &Row<'_>,
        extra_style: Style,
    ) {
        for (column, (cell_area, line)) in columns.iter().zip(row.cells.iter()).enumerate() {
            let width = cell_area.width().get();
            if width == 0 {
                continue;
            }
            let content_width = line.width().as_u16().min(width);
            let free = width.saturating_sub(content_width);
            let shift = match self.column_alignment_of(column) {
                TextAlignment::Left => 0,
                TextAlignment::Center => free / 2,
                TextAlignment::Right => free,
            };
            let styled = row.style.patch(extra_style);
            if styled != Style::DEFAULT {
                buffer.set_style(
                    Rect::new(
                        cell_area.x(),
                        row_index,
                        cell_area.width(),
                        TerminalHeight::new(1),
                    ),
                    styled,
                );
            }
            let _ = buffer.set_line(
                cell_area.x().saturating_add(shift),
                row_index,
                line,
                TerminalWidth::new(width.saturating_sub(shift)),
            );
        }
    }
}

impl<'a, T: Into<Line<'a>>> From<Vec<T>> for Row<'a> {
    fn from(value: Vec<T>) -> Self {
        Self::new(value)
    }
}

impl StatefulWidget for Table<'_> {
    type State = TableState;

    fn render(self, area: Rect, buffer: &mut Buffer, state: &mut TableState) {
        let header_rows = u16::from(self.header.is_some());
        let footer_rows = u16::from(self.footer.is_some());
        let body_height = area
            .height()
            .get()
            .saturating_sub(header_rows)
            .saturating_sub(footer_rows);

        let len = self.rows.len();
        state.set_selected_clamped(len);
        state.set_offset(follow_selection(
            state.offset(),
            state.selected(),
            len,
            usize::from(body_height),
            self.scroll_padding,
        ));
        if area.is_empty() {
            state.set_offset(state.offset().min(len.saturating_sub(1)));
            return;
        }
        buffer.set_style(area, self.style);

        let symbol_width = if state.selected().is_some() {
            display_width(self.highlight_symbol)
                .as_u16()
                .min(area.width().get())
        } else {
            0
        };
        let content = Rect::new(
            area.x().saturating_add(symbol_width),
            area.y(),
            TerminalWidth::new(area.width().get().saturating_sub(symbol_width)),
            area.height(),
        );
        let mut slots = [Rect::default(); Layout::MAX_STACK_SLOTS];
        let column_count = self.layout.split_into(content, &mut slots);
        let columns = &slots[..column_count];

        let mut row = area.y();
        if let Some(header) = &self.header {
            self.render_row(buffer, columns, row, header, self.header_style);
            row = row.saturating_add(1);
        }

        for (row_offset, (index, body_row)) in self
            .rows
            .iter()
            .enumerate()
            .skip(state.offset())
            .take(usize::from(body_height))
            .enumerate()
        {
            let offset = u16::try_from(row_offset).unwrap_or(u16::MAX);
            let y = row.saturating_add(offset);
            let is_selected = state.selected() == Some(index);
            if is_selected {
                buffer.set_style(
                    Rect::new(area.x(), y, area.width(), TerminalHeight::new(1)),
                    self.highlight_style,
                );
                if symbol_width > 0 {
                    let _ = buffer.set_string(
                        area.x(),
                        y,
                        self.highlight_symbol,
                        self.highlight_style,
                        symbol_width,
                    );
                }
            }
            self.render_row(buffer, columns, y, body_row, Style::DEFAULT);
        }

        if let Some(footer) = &self.footer {
            let footer_y = area.bottom().saturating_sub(1);
            self.render_row(buffer, columns, footer_y, footer, self.footer_style);
        }
    }
}
