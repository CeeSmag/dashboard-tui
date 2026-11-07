//! Contiguous 2D terminal cell buffer (`Buffer`) and zero-allocation stack string formatter (`StackStr`).
//!
//! All widgets render directly into a pre-allocated `Buffer` without allocating heap strings.

use core::fmt;
use unicode_segmentation::UnicodeSegmentation;

use crate::core::cell::Cell;
use crate::core::coordinates::{ColumnIndex, RowIndex, TerminalWidth};
use crate::core::grapheme::{display_width, grapheme_width, truncate_with_ellipsis_parts};
use crate::core::rect::Rect;
use crate::core::style::Style;
use crate::core::text::{Line, Span, TextAlignment};

/// Fixed-capacity stack-allocated UTF-8 string buffer (`N` bytes) for zero-allocation formatting.
///
/// Implements [`core::fmt::Write`] so numeric telemetry values (`137.4 MB/s`, `98.5%`, `[15]`)
/// can be formatted on the stack and written directly into a [`Buffer`].
///
/// # Examples
///
/// ```
/// use core::fmt::Write;
/// use tui_rs::core::StackStr;
///
/// let mut buf = StackStr::<32>::new();
/// let _ = write!(buf, "[{}] {:.1}%", 1, 99.5);
/// assert_eq!(buf.as_str(), "[1] 99.5%");
/// ```
#[derive(Clone, Copy)]
pub struct StackStr<const N: usize = 64> {
    bytes: [u8; N],
    len: usize,
}

impl<const N: usize> fmt::Debug for StackStr<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "StackStr({:?})", self.as_str())
    }
}

impl<const N: usize> Default for StackStr<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> StackStr<N> {
    /// Creates an empty `StackStr`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::StackStr;
    /// let s = StackStr::<16>::new();
    /// assert!(s.is_empty());
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self {
            bytes: [0u8; N],
            len: 0,
        }
    }

    /// Returns the valid UTF-8 slice written so far.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::StackStr;
    /// let mut s = StackStr::<16>::new();
    /// s.push_str("ok");
    /// assert_eq!(s.as_str(), "ok");
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.bytes[..self.len]).unwrap_or("")
    }

    /// Returns true if no bytes have been written.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::StackStr;
    /// assert!(StackStr::<8>::new().is_empty());
    /// ```
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the number of UTF-8 bytes currently stored.
    #[must_use]
    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Appends a single `char` if capacity permits, returning `true` if written.
    pub fn push_char(&mut self, ch: char) -> bool {
        let mut buf = [0u8; 4];
        let s = ch.encode_utf8(&mut buf);
        if self.len.saturating_add(s.len()) > N {
            return false;
        }
        self.push_str(s);
        true
    }

    /// Appends as much of `text` as fits on valid UTF-8 character boundaries.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::StackStr;
    /// let mut s = StackStr::<4>::new();
    /// s.push_str("hello");
    /// assert_eq!(s.as_str(), "hell");
    /// ```
    pub fn push_str(&mut self, text: &str) {
        let avail = N.saturating_sub(self.len);
        if avail == 0 {
            return;
        }
        let mut take = text.len().min(avail);
        while take > 0 && !text.is_char_boundary(take) {
            take = take.saturating_sub(1);
        }
        if take > 0 {
            self.bytes[self.len..self.len + take].copy_from_slice(&text.as_bytes()[..take]);
            self.len = self.len.saturating_add(take);
        }
    }
}

impl<const N: usize> fmt::Write for StackStr<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push_str(s);
        Ok(())
    }
}

/// A 2D contiguous grid of [`Cell`] elements representing a renderable terminal surface.
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Buffer, ColumnIndex, Rect, RowIndex, Style};
///
/// let rect = Rect::from_xywh(0, 0, 10, 2);
/// let mut buf = Buffer::empty(rect);
/// buf.set_str_clipped(ColumnIndex(0), RowIndex(0), "VaultDrive", ColumnIndex(10), Style::DEFAULT);
/// assert_eq!(buf.to_visual_lines(), vec!["VaultDrive", "          "]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Buffer {
    bounds: Rect,
    cells: Vec<Cell>,
}

impl Buffer {
    /// Creates a new `Buffer` filled with blank `Cell::EMPTY` cells for `bounds`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Buffer, Rect};
    ///
    /// let buf = Buffer::empty(Rect::from_xywh(0, 0, 20, 5));
    /// assert_eq!(buf.as_cell_slice().len(), 100);
    /// ```
    #[must_use]
    pub fn empty(bounds: Rect) -> Self {
        let count = bounds.area();
        Self {
            bounds,
            cells: vec![Cell::EMPTY; count],
        }
    }

    /// Returns the bounding [`Rect`] of this buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Buffer, Rect};
    /// let r = Rect::from_xywh(0, 0, 10, 4);
    /// assert_eq!(Buffer::empty(r).bounds(), r);
    /// ```
    #[must_use]
    pub const fn bounds(&self) -> Rect {
        self.bounds
    }

    /// Alias for [`Self::bounds`]: returns the bounding [`Rect`] of this buffer.
    #[must_use]
    #[inline]
    pub const fn area(&self) -> Rect {
        self.bounds
    }

    /// Returns a read-only slice of all cells in row-major order (`C-CONV` `as_cell_slice`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Buffer, Rect};
    /// let buf = Buffer::empty(Rect::from_xywh(0, 0, 4, 2));
    /// assert_eq!(buf.as_cell_slice().len(), 8);
    /// ```
    #[must_use]
    pub fn as_cell_slice(&self) -> &[Cell] {
        &self.cells
    }

    /// Resets all cells in the buffer to `Cell::EMPTY` without deallocating memory.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Buffer, ColumnIndex, Rect, RowIndex, Style};
    /// let mut buf = Buffer::empty(Rect::from_xywh(0, 0, 4, 1));
    /// buf.set_str_clipped(ColumnIndex(0), RowIndex(0), "Test", ColumnIndex(4), Style::DEFAULT);
    /// buf.reset();
    /// assert_eq!(buf.to_visual_lines(), vec!["    "]);
    /// ```
    pub fn reset(&mut self) {
        for cell in &mut self.cells {
            cell.reset();
        }
    }

    /// Resizes the buffer to `new_bounds` and resets all cells.
    ///
    /// Reuses existing heap capacity whenever `new_bounds.area() <= capacity`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Buffer, Rect};
    /// let mut buf = Buffer::empty(Rect::from_xywh(0, 0, 10, 10));
    /// buf.resize(Rect::from_xywh(0, 0, 5, 5));
    /// assert_eq!(buf.bounds(), Rect::from_xywh(0, 0, 5, 5));
    /// ```
    pub fn resize(&mut self, new_bounds: Rect) {
        self.bounds = new_bounds;
        let needed = new_bounds.area();
        self.cells.resize(needed, Cell::EMPTY);
        self.reset();
    }

    /// Computes the flat slice index for `(column, row)`, or `None` if out of bounds.
    #[must_use]
    pub const fn index_of(&self, column: ColumnIndex, row: RowIndex) -> Option<usize> {
        if !self.bounds.contains(column, row) {
            return None;
        }
        let rel_col = column.0.saturating_sub(self.bounds.left().0) as usize;
        let rel_row = row.0.saturating_sub(self.bounds.top().0) as usize;
        let stride = self.bounds.width().as_usize();
        Some(rel_row.saturating_mul(stride).saturating_add(rel_col))
    }

    /// Returns a shared reference to the [`Cell`] at `(column, row)`, if within bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Buffer, ColumnIndex, Rect, RowIndex};
    /// let buf = Buffer::empty(Rect::from_xywh(0, 0, 5, 5));
    /// assert!(buf.cell_at(ColumnIndex(2), RowIndex(2)).is_some());
    /// assert!(buf.cell_at(ColumnIndex(9), RowIndex(2)).is_none());
    /// ```
    #[must_use]
    pub fn cell_at(&self, column: ColumnIndex, row: RowIndex) -> Option<&Cell> {
        let idx = self.index_of(column, row)?;
        self.cells.get(idx)
    }

    /// Returns a mutable reference to the [`Cell`] at `(column, row)`, if within bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Buffer, ColumnIndex, Rect, RowIndex};
    /// let mut buf = Buffer::empty(Rect::from_xywh(0, 0, 5, 5));
    /// if let Some(cell) = buf.cell_at_mut(ColumnIndex(0), RowIndex(0)) {
    ///     cell.set_char('X');
    /// }
    /// assert_eq!(buf.cell_at(ColumnIndex(0), RowIndex(0)).unwrap().symbol(), "X");
    /// ```
    #[must_use]
    pub fn cell_at_mut(&mut self, column: ColumnIndex, row: RowIndex) -> Option<&mut Cell> {
        let idx = self.index_of(column, row)?;
        self.cells.get_mut(idx)
    }

    /// Alias for [`Self::cell_at`].
    #[must_use]
    #[inline]
    pub fn get(&self, column: ColumnIndex, row: RowIndex) -> Option<&Cell> {
        self.cell_at(column, row)
    }

    /// Alias for [`Self::cell_at_mut`].
    #[must_use]
    #[inline]
    pub fn get_mut(&mut self, column: ColumnIndex, row: RowIndex) -> Option<&mut Cell> {
        self.cell_at_mut(column, row)
    }

    /// Sets a single cell's symbol and style at `(column, row)`, returning `true` if within bounds.
    pub fn set_cell(
        &mut self,
        column: ColumnIndex,
        row: RowIndex,
        symbol: &str,
        style: Style,
    ) -> bool {
        if let Some(cell) = self.cell_at_mut(column, row) {
            cell.set_symbol(symbol);
            cell.replace_style(style);
            cell.set_wide_continuation(false);
            true
        } else {
            false
        }
    }

    /// Writes `text` at `(column, row)` with `style`, clipped to `max_width` columns,
    /// and returns the next [`ColumnIndex`].
    pub fn set_string(
        &mut self,
        column: ColumnIndex,
        row: RowIndex,
        text: &str,
        style: Style,
        max_width: u16,
    ) -> ColumnIndex {
        self.set_str_clipped(column, row, text, column.saturating_add(max_width), style)
    }

    /// Fills `area` with `ch` and `style` (clearing wide continuations).
    pub fn fill_rect(&mut self, area: Rect, ch: char, style: Style) {
        self.clear_area(area, style);
        if ch != ' ' {
            let clipped = self.bounds.intersection(area);
            for r in clipped.top().0..clipped.bottom().0 {
                for c in clipped.left().0..clipped.right().0 {
                    self.set_char(ColumnIndex(c), RowIndex(r), ch, style);
                }
            }
        }
    }

    /// Applies `style` to every cell inside `area` (clipped to buffer bounds).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Buffer, Color, ColumnIndex, Rect, RowIndex, Style};
    /// let rect = Rect::from_xywh(0, 0, 4, 2);
    /// let mut buf = Buffer::empty(rect);
    /// buf.set_style(rect, Style::new().fg(Color::Cyan));
    /// assert_eq!(buf.cell_at(ColumnIndex(1), RowIndex(1)).unwrap().style().foreground(), Some(Color::Cyan));
    /// ```
    pub fn set_style(&mut self, area: Rect, style: Style) {
        let clipped = self.bounds.intersection(area);
        if clipped.is_empty() {
            return;
        }
        let mut r = clipped.top().0;
        while r < clipped.bottom().0 {
            let mut c = clipped.left().0;
            while c < clipped.right().0 {
                if let Some(cell) = self.cell_at_mut(ColumnIndex(c), RowIndex(r)) {
                    cell.set_style(style);
                }
                c = c.saturating_add(1);
            }
            r = r.saturating_add(1);
        }
    }

    /// Clears `area` to blank spaces with `style` (used before drawing modal overlays).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Buffer, ColumnIndex, Rect, RowIndex, Style};
    /// let rect = Rect::from_xywh(0, 0, 4, 1);
    /// let mut buf = Buffer::empty(rect);
    /// buf.set_str_clipped(ColumnIndex(0), RowIndex(0), "ABCD", ColumnIndex(4), Style::DEFAULT);
    /// buf.clear_area(Rect::from_xywh(1, 0, 2, 1), Style::DEFAULT);
    /// assert_eq!(buf.to_visual_lines(), vec!["A  D"]);
    /// ```
    pub fn clear_area(&mut self, area: Rect, style: Style) {
        let clipped = self.bounds.intersection(area);
        if clipped.is_empty() {
            return;
        }
        let mut r = clipped.top().0;
        while r < clipped.bottom().0 {
            // If the left edge of the cleared area lands on the 2nd column of a wide char,
            // clear the leading half at left - 1 so no orphaned wide glyph remains.
            if clipped.left().0 > self.bounds.left().0 {
                let left_col = ColumnIndex(clipped.left().0);
                let is_cont = self
                    .cell_at(left_col, RowIndex(r))
                    .is_some_and(Cell::is_wide_continuation);
                if is_cont {
                    if let Some(prev) = self.cell_at_mut(left_col.saturating_sub(1), RowIndex(r)) {
                        prev.reset();
                    }
                }
            }
            let mut c = clipped.left().0;
            while c < clipped.right().0 {
                if let Some(cell) = self.cell_at_mut(ColumnIndex(c), RowIndex(r)) {
                    cell.reset();
                    cell.replace_style(style);
                }
                c = c.saturating_add(1);
            }
            r = r.saturating_add(1);
        }
    }

    /// Writes a single `char` at `(column, row)` with `style`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Buffer, ColumnIndex, Rect, RowIndex, Style};
    /// let mut buf = Buffer::empty(Rect::from_xywh(0, 0, 3, 1));
    /// buf.set_char(ColumnIndex(1), RowIndex(0), '●', Style::DEFAULT);
    /// assert_eq!(buf.to_visual_lines(), vec![" ● "]);
    /// ```
    pub fn set_char(&mut self, column: ColumnIndex, row: RowIndex, ch: char, style: Style) {
        let mut utf8 = [0u8; 4];
        let s = ch.encode_utf8(&mut utf8);
        let max_col = column.saturating_add(2).min(self.bounds.right());
        let _ = self.set_str_clipped(column, row, s, max_col, style);
    }

    /// Writes `text` starting at `(column, row)`, stopping before `max_column` (exclusive).
    ///
    /// Properly accounts for UAX #29 grapheme clusters and 2-column East Asian / emoji widths.
    /// If a 2-column grapheme lands on the final single column (`max_column - 1`), it writes a
    /// space `' '` instead of overflowing `max_column` (`RULES.md` §5.1).
    ///
    /// Returns the new [`ColumnIndex`] after the last written grapheme.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Buffer, ColumnIndex, Rect, RowIndex, Style};
    ///
    /// let mut buf = Buffer::empty(Rect::from_xywh(0, 0, 5, 1));
    /// let end = buf.set_str_clipped(ColumnIndex(0), RowIndex(0), "日本A", ColumnIndex(3), Style::DEFAULT);
    /// // "日" takes 2 cols; "本" needs 2 cols but only 1 col remains before col 3, so it pads with ' '.
    /// assert_eq!(end, ColumnIndex(3));
    /// assert_eq!(buf.to_visual_lines(), vec!["日   "]);
    /// ```
    pub fn set_str_clipped(
        &mut self,
        column: ColumnIndex,
        row: RowIndex,
        text: &str,
        max_column: ColumnIndex,
        style: Style,
    ) -> ColumnIndex {
        if row < self.bounds.top() || row >= self.bounds.bottom() {
            return column;
        }
        let limit = max_column.min(self.bounds.right()).as_u16();
        let mut cur_col = column.max(self.bounds.left()).as_u16();

        for grapheme in text.graphemes(true) {
            if cur_col >= limit {
                break;
            }
            let gw = grapheme_width(grapheme);
            if gw == 0 {
                continue;
            }
            if cur_col.saturating_add(gw) > limit {
                // 2-column grapheme cannot fit in the 1 remaining column: pad with space
                if let Some(cell) = self.cell_at_mut(ColumnIndex(cur_col), row) {
                    cell.set_char(' ');
                    cell.set_style(style);
                }
                cur_col = cur_col.saturating_add(1);
                break;
            }

            // If we overwrite the 2nd half of an existing wide grapheme, clear its 1st half
            if cur_col > self.bounds.left().0 {
                let was_cont = self
                    .cell_at(ColumnIndex(cur_col), row)
                    .is_some_and(Cell::is_wide_continuation);
                if was_cont {
                    if let Some(prev) =
                        self.cell_at_mut(ColumnIndex(cur_col.saturating_sub(1)), row)
                    {
                        prev.set_char(' ');
                    }
                }
            }

            if let Some(cell) = self.cell_at_mut(ColumnIndex(cur_col), row) {
                cell.set_symbol(grapheme);
                cell.set_style(style);
            }

            if gw == 2 {
                let next_col = cur_col.saturating_add(1);
                if let Some(next_cell) = self.cell_at_mut(ColumnIndex(next_col), row) {
                    next_cell.set_style(style);
                    next_cell.set_wide_continuation(true);
                }
            }

            cur_col = cur_col.saturating_add(gw);
        }

        ColumnIndex(cur_col)
    }

    /// Writes `text` truncated with `"…"` if its width exceeds `max_width`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Buffer, ColumnIndex, Rect, RowIndex, Style, TerminalWidth};
    /// let mut buf = Buffer::empty(Rect::from_xywh(0, 0, 6, 1));
    /// buf.set_str_truncated(ColumnIndex(0), RowIndex(0), "VaultDrive", TerminalWidth(6), Style::DEFAULT);
    /// assert_eq!(buf.to_visual_lines(), vec!["Vault…"]);
    /// ```
    pub fn set_str_truncated(
        &mut self,
        column: ColumnIndex,
        row: RowIndex,
        text: &str,
        max_width: TerminalWidth,
        style: Style,
    ) -> ColumnIndex {
        let (prefix, ellipsis, _) = truncate_with_ellipsis_parts(text, max_width);
        let max_col = column.saturating_add(max_width.as_u16());
        let mut pos = self.set_str_clipped(column, row, prefix, max_col, style);
        if ellipsis && pos < max_col {
            pos = self.set_str_clipped(pos, row, "…", max_col, style);
        }
        pos
    }

    /// Writes a [`Span`] starting at `(column, row)`, clipped at `max_column`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Buffer, ColumnIndex, Rect, RowIndex, Span};
    /// let mut buf = Buffer::empty(Rect::from_xywh(0, 0, 5, 1));
    /// buf.set_span(ColumnIndex(0), RowIndex(0), &Span::raw("Hello"), ColumnIndex(5));
    /// assert_eq!(buf.to_visual_lines(), vec!["Hello"]);
    /// ```
    pub fn set_span(
        &mut self,
        column: ColumnIndex,
        row: RowIndex,
        span: &Span<'_>,
        max_column: ColumnIndex,
    ) -> ColumnIndex {
        let end_col = self.set_str_clipped(column, row, span.as_str(), max_column, span.style());
        if let Some(url) = span.hyperlink_url() {
            let mut c = column.max(self.bounds.left());
            while c < end_col {
                if let Some(cell) = self.cell_at_mut(c, row) {
                    cell.set_hyperlink(Some(url));
                }
                c = c.saturating_add(1);
            }
        }
        end_col
    }

    /// Writes `text` with `style` and attaches a clickable OSC 8 `url` across all written cells.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Buffer, ColumnIndex, Rect, RowIndex, Style, TerminalHeight, TerminalWidth};
    ///
    /// let mut buf = Buffer::empty(Rect::from_size(TerminalWidth::new(20), TerminalHeight::new(1)));
    /// buf.set_hyperlink(
    ///     ColumnIndex::new(0),
    ///     RowIndex::new(0),
    ///     "https://github.com",
    ///     "GitHub",
    ///     Style::DEFAULT,
    ///     10,
    /// );
    /// assert_eq!(buf.cell_at(ColumnIndex::new(0), RowIndex::new(0)).and_then(|c| c.hyperlink()), Some("https://github.com"));
    /// ```
    pub fn set_hyperlink(
        &mut self,
        column: ColumnIndex,
        row: RowIndex,
        url: &str,
        text: &str,
        style: Style,
        max_width: u16,
    ) -> ColumnIndex {
        let max_col = column.saturating_add(max_width);
        let end_col = self.set_str_clipped(column, row, text, max_col, style);
        let mut c = column.max(self.bounds.left());
        while c < end_col {
            if let Some(cell) = self.cell_at_mut(c, row) {
                cell.set_hyperlink(Some(url));
            }
            c = c.saturating_add(1);
        }
        end_col
    }

    /// Writes a [`Line`] inside `(column, row)` across `width` columns respecting [`TextAlignment`].
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Buffer, ColumnIndex, Line, Rect, RowIndex, TerminalWidth, TextAlignment};
    /// let mut buf = Buffer::empty(Rect::from_xywh(0, 0, 8, 1));
    /// let line = Line::raw("OK").alignment(TextAlignment::Center);
    /// buf.set_line(ColumnIndex(0), RowIndex(0), &line, TerminalWidth(8));
    /// assert_eq!(buf.to_visual_lines(), vec!["   OK   "]);
    /// ```
    pub fn set_line(
        &mut self,
        column: ColumnIndex,
        row: RowIndex,
        line: &Line<'_>,
        width: TerminalWidth,
    ) -> ColumnIndex {
        if width.is_zero() {
            return column;
        }
        let max_col = column.saturating_add(width.as_u16());
        if line.base_style() != Style::DEFAULT {
            self.set_style(
                Rect::new(column, row, width, crate::core::TerminalHeight(1)),
                line.base_style(),
            );
        }
        let line_w = line.width().as_u16();
        let offset = match line.text_alignment() {
            TextAlignment::Left => 0,
            TextAlignment::Center => width.as_u16().saturating_sub(line_w) / 2,
            TextAlignment::Right => width.as_u16().saturating_sub(line_w),
        };
        let mut cur = column.saturating_add(offset);
        for span in line.as_spans() {
            if cur >= max_col {
                break;
            }
            let effective_style = line.base_style().patch(span.style());
            let next_cur = self.set_str_clipped(cur, row, span.as_str(), max_col, effective_style);
            if let Some(url) = span.hyperlink_url() {
                let mut c = cur.max(self.bounds.left());
                while c < next_cur {
                    if let Some(cell) = self.cell_at_mut(c, row) {
                        cell.set_hyperlink(Some(url));
                    }
                    c = c.saturating_add(1);
                }
            }
            cur = next_cur;
        }
        cur
    }

    /// Renders `text` centered horizontally on `row` within `[column .. column + width)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Buffer, ColumnIndex, Rect, RowIndex, Style, TerminalWidth};
    /// let mut buf = Buffer::empty(Rect::from_xywh(0, 0, 6, 1));
    /// buf.set_str_centered(ColumnIndex(0), RowIndex(0), "HI", TerminalWidth(6), Style::DEFAULT);
    /// assert_eq!(buf.to_visual_lines(), vec!["  HI  "]);
    /// ```
    pub fn set_str_centered(
        &mut self,
        column: ColumnIndex,
        row: RowIndex,
        text: &str,
        width: TerminalWidth,
        style: Style,
    ) {
        let text_w = display_width(text).as_u16();
        let offset = width.as_u16().saturating_sub(text_w) / 2;
        let start = column.saturating_add(offset);
        let max_col = column.saturating_add(width.as_u16());
        let _ = self.set_str_clipped(start, row, text, max_col, style);
    }

    /// Converts the buffer into a vector of visual plain-text lines for headless snapshot assertions (`C-CONV` `to_visual_lines`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Buffer, Rect};
    /// let buf = Buffer::empty(Rect::from_xywh(0, 0, 3, 2));
    /// assert_eq!(buf.to_visual_lines(), vec!["   ", "   "]);
    /// ```
    #[must_use]
    pub fn to_visual_lines(&self) -> Vec<String> {
        let width = self.bounds.width().as_usize();
        let height = self.bounds.height().as_usize();
        let mut lines = Vec::with_capacity(height);
        for row_idx in 0..height {
            let mut row_str = String::with_capacity(width);
            let row_start = row_idx.saturating_mul(width);
            for col_idx in 0..width {
                if let Some(cell) = self.cells.get(row_start.saturating_add(col_idx)) {
                    if !cell.is_wide_continuation() {
                        row_str.push_str(cell.symbol());
                    }
                }
            }
            lines.push(row_str);
        }
        lines
    }
}
