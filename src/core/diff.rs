//! Zero-allocation double-buffer delta calculator (`CellDiff`).
//!
//! Compares `previous` and `current` [`Buffer`] grids and yields only mutated cells
//! so the ANSI backend emits minimal escape sequences per frame (`RULES.md` §3.2).

use crate::core::buffer::Buffer;
use crate::core::cell::Cell;
use crate::core::coordinates::{ColumnIndex, RowIndex};

/// A single mutated cell coordinate discovered between two consecutive frames.
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Buffer, ColumnIndex, diff_buffers, Rect, RowIndex, Style};
///
/// let rect = Rect::from_xywh(0, 0, 4, 1);
/// let prev = Buffer::empty(rect);
/// let mut curr = Buffer::empty(rect);
/// curr.set_str_clipped(ColumnIndex(1), RowIndex(0), "A", ColumnIndex(4), Style::DEFAULT);
///
/// let diffs: Vec<_> = diff_buffers(&prev, &curr).collect();
/// assert_eq!(diffs.len(), 1);
/// assert_eq!(diffs[0].column(), ColumnIndex(1));
/// assert_eq!(diffs[0].cell().symbol(), "A");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellDiff<'a> {
    column: ColumnIndex,
    row: RowIndex,
    cell: &'a Cell,
}

impl<'a> CellDiff<'a> {
    /// Returns the column coordinate of the mutated cell.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Buffer, ColumnIndex, diff_buffers, Rect, RowIndex, Style};
    /// let r = Rect::from_xywh(0, 0, 2, 1);
    /// let prev = Buffer::empty(r);
    /// let mut curr = Buffer::empty(r);
    /// curr.set_char(ColumnIndex(0), RowIndex(0), 'X', Style::DEFAULT);
    /// let diff = diff_buffers(&prev, &curr).next().unwrap();
    /// assert_eq!(diff.column(), ColumnIndex(0));
    /// ```
    #[must_use]
    pub const fn column(self) -> ColumnIndex {
        self.column
    }

    /// Returns the row coordinate of the mutated cell.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Buffer, ColumnIndex, diff_buffers, Rect, RowIndex, Style};
    /// let r = Rect::from_xywh(0, 0, 2, 1);
    /// let prev = Buffer::empty(r);
    /// let mut curr = Buffer::empty(r);
    /// curr.set_char(ColumnIndex(0), RowIndex(0), 'X', Style::DEFAULT);
    /// let diff = diff_buffers(&prev, &curr).next().unwrap();
    /// assert_eq!(diff.row(), RowIndex(0));
    /// ```
    #[must_use]
    pub const fn row(self) -> RowIndex {
        self.row
    }

    /// Returns a reference to the new [`Cell`] state.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Buffer, ColumnIndex, diff_buffers, Rect, RowIndex, Style};
    /// let r = Rect::from_xywh(0, 0, 2, 1);
    /// let prev = Buffer::empty(r);
    /// let mut curr = Buffer::empty(r);
    /// curr.set_char(ColumnIndex(0), RowIndex(0), 'X', Style::DEFAULT);
    /// let diff = diff_buffers(&prev, &curr).next().unwrap();
    /// assert_eq!(diff.cell().symbol(), "X");
    /// ```
    #[must_use]
    pub const fn cell(self) -> &'a Cell {
        self.cell
    }
}

/// Returns a lazy, zero-allocation iterator over all cells in `current` that differ from `previous`.
///
/// Trailing wide-character continuation cells (`cell.is_wide_continuation()`) are skipped
/// because writing the leading 2-column grapheme at `col - 1` already advances the terminal cursor by 2.
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Buffer, diff_buffers, Rect};
///
/// let r = Rect::from_xywh(0, 0, 10, 5);
/// let a = Buffer::empty(r);
/// let b = Buffer::empty(r);
/// assert_eq!(diff_buffers(&a, &b).count(), 0);
/// ```
#[must_use]
pub fn diff_buffers<'a>(previous: &'a Buffer, current: &'a Buffer) -> DiffIter<'a> {
    let bounds = current.bounds();
    let force_all = previous.bounds() != bounds;
    DiffIter {
        prev_cells: previous.as_cell_slice(),
        curr_cells: current.as_cell_slice(),
        origin_col: bounds.left().as_u16(),
        origin_row: bounds.top().as_u16(),
        stride: bounds.width().as_usize(),
        index: 0,
        force_all,
    }
}

/// Lazy iterator returned by [`diff_buffers`].
#[derive(Debug, Clone)]
pub struct DiffIter<'a> {
    prev_cells: &'a [Cell],
    curr_cells: &'a [Cell],
    origin_col: u16,
    origin_row: u16,
    stride: usize,
    index: usize,
    force_all: bool,
}

impl<'a> Iterator for DiffIter<'a> {
    type Item = CellDiff<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.stride == 0 {
            return None;
        }
        while self.index < self.curr_cells.len() {
            let i = self.index;
            self.index = self.index.saturating_add(1);

            let curr = &self.curr_cells[i];
            if curr.is_wide_continuation() {
                continue;
            }

            let changed = self.force_all || self.prev_cells.get(i).is_none_or(|prev| prev != curr);

            if changed {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "buffer dimensions are bounded by u16"
                )]
                let rel_row = (i / self.stride) as u16;
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "buffer width is bounded by u16"
                )]
                let rel_col = (i % self.stride) as u16;
                return Some(CellDiff {
                    column: ColumnIndex(self.origin_col.saturating_add(rel_col)),
                    row: RowIndex(self.origin_row.saturating_add(rel_row)),
                    cell: curr,
                });
            }
        }
        None
    }
}
