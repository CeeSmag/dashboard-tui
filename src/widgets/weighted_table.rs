//! Content-measured weighted column table (`WeightedTable`) inspired by `duf`.
//!
//! Ports the 2-pass column width solver from `ref/duf-master/table.go`:
//! 1. **Pass 1 (Measurement)**: Computes the natural maximum display width of
//!    each column across its header and all visible rows.
//! 2. **Pass 2 (Proportional Distribution)**: Distributes the available table
//!    width according to each column's `width_weight`, capping columns at their
//!    natural content width first and redistributing surplus width to wider
//!    columns so space is never wasted on short columns.

use crate::core::buffer::Buffer;
use crate::core::coordinates::RowIndex;
use crate::core::grapheme::display_width;
use crate::core::palette::ColorPalette;
use crate::core::rect::Rect;
use crate::core::style::Style;
use crate::core::text::TextAlignment;

/// Maximum number of columns supported in stack-allocated table layout calculations.
pub const MAX_TABLE_COLUMNS: usize = 16;

/// Specification for a single column in a [`WeightedTable`].
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::TextAlignment;
/// use dashboard_tui::widgets::TableColumn;
///
/// let col = TableColumn::new("Mountpoint", 3).with_alignment(TextAlignment::Left);
/// assert_eq!(col.header(), "Mountpoint");
/// assert_eq!(col.weight(), 3);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableColumn<'a> {
    header: &'a str,
    weight: u16,
    min_width: u16,
    alignment: TextAlignment,
}

impl<'a> TableColumn<'a> {
    /// Creates a new [`TableColumn`] with a header label and relative width `weight`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::widgets::TableColumn;
    ///
    /// let col = TableColumn::new("Size", 1);
    /// assert_eq!(col.weight(), 1);
    /// ```
    #[must_use]
    #[inline]
    pub const fn new(header: &'a str, weight: u16) -> Self {
        Self {
            header,
            weight: if weight > 0 { weight } else { 1 },
            min_width: 3,
            alignment: TextAlignment::Left,
        }
    }

    /// Sets the minimum column width in cells (default `3`).
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::widgets::TableColumn;
    ///
    /// let col = TableColumn::new("ID", 1).with_min_width(4);
    /// assert_eq!(col.min_width(), 4);
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_min_width(mut self, min_width: u16) -> Self {
        self.min_width = min_width;
        self
    }

    /// Sets the text alignment for cells in this column.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::TextAlignment;
    /// use dashboard_tui::widgets::TableColumn;
    ///
    /// let col = TableColumn::new("Used", 1).with_alignment(TextAlignment::Right);
    /// assert_eq!(col.alignment(), TextAlignment::Right);
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_alignment(mut self, alignment: TextAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Returns the column header text.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::widgets::TableColumn;
    ///
    /// assert_eq!(TableColumn::new("Device", 2).header(), "Device");
    /// ```
    #[must_use]
    #[inline]
    pub const fn header(&self) -> &'a str {
        self.header
    }

    /// Returns the relative width weight.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::widgets::TableColumn;
    ///
    /// assert_eq!(TableColumn::new("Device", 2).weight(), 2);
    /// ```
    #[must_use]
    #[inline]
    pub const fn weight(&self) -> u16 {
        self.weight
    }

    /// Returns the minimum column width in cells.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::widgets::TableColumn;
    ///
    /// assert_eq!(TableColumn::new("Device", 2).min_width(), 3);
    /// ```
    #[must_use]
    #[inline]
    pub const fn min_width(&self) -> u16 {
        self.min_width
    }

    /// Returns the column [`TextAlignment`].
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::TextAlignment;
    /// use dashboard_tui::widgets::TableColumn;
    ///
    /// assert_eq!(TableColumn::new("A", 1).alignment(), TextAlignment::Left);
    /// ```
    #[must_use]
    #[inline]
    pub const fn alignment(&self) -> TextAlignment {
        self.alignment
    }
}

/// Content-measured weighted column table widget.
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::{Buffer, ColorPalette, Rect, TerminalHeight, TerminalWidth, TextAlignment};
/// use dashboard_tui::widgets::{TableColumn, WeightedTable};
///
/// let cols = [
///     TableColumn::new("Filesystem", 2),
///     TableColumn::new("Size", 1).with_alignment(TextAlignment::Right),
/// ];
/// let rows: [&[&str]; 2] = [
///     &["vaultdrive:/encrypted", "50.0 GB"],
///     &["local:/cache", "4.2 GB"],
/// ];
/// let table = WeightedTable::new(&cols, &rows);
/// let area = Rect::from_size(TerminalWidth::new(45), TerminalHeight::new(4));
/// let mut buf = Buffer::empty(area);
/// table.render(&mut buf, area, &ColorPalette::DEFAULT);
/// assert!(buf.to_visual_lines()[0].contains("Filesystem"));
/// ```
#[derive(Debug, Clone, Copy)]
pub struct WeightedTable<'a> {
    columns: &'a [TableColumn<'a>],
    rows: &'a [&'a [&'a str]],
    selected_row: Option<usize>,
    column_gap: u16,
}

impl<'a> WeightedTable<'a> {
    /// Creates a new [`WeightedTable`] over `columns` and `rows`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::widgets::{TableColumn, WeightedTable};
    ///
    /// let cols = [TableColumn::new("Name", 1)];
    /// let rows: [&[&str]; 1] = [&["item"]];
    /// let table = WeightedTable::new(&cols, &rows);
    /// assert_eq!(table.column_gap(), 2);
    /// ```
    #[must_use]
    #[inline]
    pub const fn new(columns: &'a [TableColumn<'a>], rows: &'a [&'a [&'a str]]) -> Self {
        Self {
            columns,
            rows,
            selected_row: None,
            column_gap: 2,
        }
    }

    /// Highlights a selected row index (`0`-based).
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::widgets::{TableColumn, WeightedTable};
    ///
    /// let cols = [TableColumn::new("Name", 1)];
    /// let table = WeightedTable::new(&cols, &[]).with_selected_row(Some(0));
    /// assert_eq!(table.selected_row(), Some(0));
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_selected_row(mut self, selected: Option<usize>) -> Self {
        self.selected_row = selected;
        self
    }

    /// Sets the horizontal gap in cells between adjacent columns (default `2`).
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::widgets::{TableColumn, WeightedTable};
    ///
    /// let cols = [TableColumn::new("Name", 1)];
    /// let table = WeightedTable::new(&cols, &[]).with_column_gap(1);
    /// assert_eq!(table.column_gap(), 1);
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_column_gap(mut self, gap: u16) -> Self {
        self.column_gap = gap;
        self
    }

    /// Returns the currently highlighted row index, if any.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::widgets::{TableColumn, WeightedTable};
    ///
    /// assert_eq!(WeightedTable::new(&[], &[]).selected_row(), None);
    /// ```
    #[must_use]
    #[inline]
    pub const fn selected_row(&self) -> Option<usize> {
        self.selected_row
    }

    /// Returns the inter-column gap in cells.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::widgets::WeightedTable;
    ///
    /// assert_eq!(WeightedTable::new(&[], &[]).column_gap(), 2);
    /// ```
    #[must_use]
    #[inline]
    pub const fn column_gap(&self) -> u16 {
        self.column_gap
    }

    /// Solves the column widths for `total_width` using `duf`'s 2-pass content-capped
    /// proportional distribution algorithm, writing the resolved widths into `out_widths`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::widgets::{TableColumn, WeightedTable};
    ///
    /// let cols = [TableColumn::new("ID", 1), TableColumn::new("Path", 3)];
    /// let rows: [&[&str]; 1] = [&["1", "/home/vaultdrive/mount"]];
    /// let table = WeightedTable::new(&cols, &rows);
    /// let mut widths = [0_u16; 2];
    /// table.solve_column_widths(40, &mut widths);
    /// assert!(widths[1] > widths[0]);
    /// ```
    #[expect(
        clippy::cast_possible_truncation,
        reason = "column widths are clamped to u16"
    )]
    pub fn solve_column_widths(&self, total_width: u16, out_widths: &mut [u16]) {
        let n = self
            .columns
            .len()
            .min(out_widths.len())
            .min(MAX_TABLE_COLUMNS);
        if n == 0 {
            return;
        }
        let total_gaps = self.column_gap.saturating_mul((n.saturating_sub(1)) as u16);
        let budget = total_width.saturating_sub(total_gaps);

        let mut natural = [0_u16; MAX_TABLE_COLUMNS];
        for (i, col) in self.columns.iter().take(n).enumerate() {
            let mut max_w = display_width(col.header).as_u16();
            for row in self.rows {
                if let Some(&cell_text) = row.get(i) {
                    let cw = display_width(cell_text).as_u16();
                    if cw > max_w {
                        max_w = cw;
                    }
                }
            }
            natural[i] = max_w.max(col.min_width);
        }

        let natural_sum: u32 = natural[..n].iter().map(|&w| u32::from(w)).sum();
        if natural_sum <= u32::from(budget) {
            // Everything fits naturally! Distribute extra space by weight.
            let extra = u32::from(budget).saturating_sub(natural_sum);
            let total_weight: u32 = self
                .columns
                .iter()
                .take(n)
                .map(|c| u32::from(c.weight))
                .sum::<u32>()
                .max(1);
            let mut used = 0_u16;
            for i in 0..n {
                let share = ((extra * u32::from(self.columns[i].weight)) / total_weight) as u16;
                out_widths[i] = natural[i].saturating_add(share);
                used = used.saturating_add(out_widths[i]);
            }
            if used < budget {
                out_widths[n - 1] = out_widths[n - 1].saturating_add(budget - used);
            }
            return;
        }

        // Otherwise, allocate proportionally by weight while capping at natural width first
        let total_weight: u32 = self
            .columns
            .iter()
            .take(n)
            .map(|c| u32::from(c.weight))
            .sum::<u32>()
            .max(1);
        let mut used = 0_u16;
        for i in 0..n {
            let share =
                ((u32::from(budget) * u32::from(self.columns[i].weight)) / total_weight) as u16;
            let w = share.max(self.columns[i].min_width).min(natural[i]);
            out_widths[i] = w;
            used = used.saturating_add(w);
        }
        // Distribute any leftover budget to columns that were truncated below natural width
        let mut rem = budget.saturating_sub(used);
        for i in 0..n {
            if rem == 0 {
                break;
            }
            if out_widths[i] < natural[i] {
                let add = (natural[i] - out_widths[i]).min(rem);
                out_widths[i] += add;
                rem -= add;
            }
        }
    }

    /// Renders the table header and visible rows into `area` inside `buffer`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::{Buffer, ColorPalette, Rect, TerminalHeight, TerminalWidth};
    /// use dashboard_tui::widgets::{TableColumn, WeightedTable};
    ///
    /// let cols = [TableColumn::new("Key", 1), TableColumn::new("Val", 1)];
    /// let rows: [&[&str]; 1] = [&["Remote", "gdrive:vault"]];
    /// let area = Rect::from_size(TerminalWidth::new(30), TerminalHeight::new(3));
    /// let mut buf = Buffer::empty(area);
    /// WeightedTable::new(&cols, &rows).render(&mut buf, area, &ColorPalette::DEFAULT);
    /// ```
    #[expect(clippy::cast_possible_truncation, reason = "widths bounded by u16")]
    pub fn render(&self, buffer: &mut Buffer, area: Rect, palette: &ColorPalette) {
        let n = self.columns.len().min(MAX_TABLE_COLUMNS);
        if n == 0 || area.is_empty() {
            return;
        }
        let mut widths = [0_u16; MAX_TABLE_COLUMNS];
        self.solve_column_widths(area.width().get(), &mut widths[..n]);

        let header_style = Style::new().with_fg(palette.accent).bold();
        let mut cx = area.x();
        let y0 = area.y();
        for (i, &col_w) in widths.iter().enumerate().take(n) {
            Self::render_cell_aligned(
                buffer,
                cx,
                y0,
                col_w,
                self.columns[i].header,
                header_style,
                self.columns[i].alignment,
            );
            cx = cx.saturating_add(col_w).saturating_add(self.column_gap);
        }

        let max_rows = usize::from(area.height().get().saturating_sub(1));
        for (row_idx, row_cells) in self.rows.iter().take(max_rows).enumerate() {
            let ry = y0.saturating_add((row_idx + 1) as u16);
            let is_sel = self.selected_row == Some(row_idx);
            let row_style = if is_sel {
                Style::new()
                    .with_fg(palette.accent2)
                    .with_bg(palette.highlight_bg)
                    .bold()
            } else {
                Style::new().with_fg(palette.text)
            };

            let mut rx = area.x();
            for (i, &col_w) in widths.iter().enumerate().take(n) {
                let text = row_cells.get(i).copied().unwrap_or("");
                Self::render_cell_aligned(
                    buffer,
                    rx,
                    ry,
                    col_w,
                    text,
                    row_style,
                    self.columns[i].alignment,
                );
                rx = rx.saturating_add(col_w).saturating_add(self.column_gap);
            }
        }
    }

    fn render_cell_aligned(
        buffer: &mut Buffer,
        col: crate::core::coordinates::ColumnIndex,
        row: RowIndex,
        col_w: u16,
        text: &str,
        style: Style,
        alignment: TextAlignment,
    ) {
        let text_w = display_width(text).as_u16().min(col_w);
        let pad = match alignment {
            TextAlignment::Left => 0,
            TextAlignment::Center => col_w.saturating_sub(text_w) / 2,
            TextAlignment::Right => col_w.saturating_sub(text_w),
        };
        let _ = buffer.set_string(
            col.saturating_add(pad),
            row,
            text,
            style,
            col_w.saturating_sub(pad),
        );
    }
}
