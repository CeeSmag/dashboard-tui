//! Saturating vertical scroll state (`VerticalScrollState`) and 1/8th sub-cell
//! fractional scrollbar (`SubCellScrollbar`).
//!
//! Synthesizes:
//! - `gitui`'s `VerticalScroll` (`src/components/utils/scroll_vertical.rs`) for
//!   saturating viewport clamping and cursor-follow scrolling.
//! - `textual`'s `ScrollBarRender` (`src/textual/scrollbar.py`) for 1/8th sub-cell
//!   fractional block thumb rendering (`▁▂▃▄▅▆▇█`).

use crate::core::buffer::Buffer;
use crate::core::coordinates::{ColumnIndex, RowIndex};
use crate::core::palette::ColorPalette;
use crate::core::style::Style;

/// Lower 1/8th..8/8th fractional block glyphs (`U+2581..U+2588`).
const LOWER_EIGHTHS: [&str; 8] = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];

/// Saturating vertical scroll state tracker.
///
/// # Examples
///
/// ```
/// use tui_rs::widgets::VerticalScrollState;
///
/// let mut scroll = VerticalScrollState::new();
/// scroll.update(0, 50, 10);
/// assert_eq!(scroll.top(), 0);
/// scroll.ensure_visible(15, 50, 10);
/// assert_eq!(scroll.top(), 6);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VerticalScrollState {
    top: usize,
    max_top: usize,
}

impl VerticalScrollState {
    /// Creates a new [`VerticalScrollState`] positioned at the top (`0`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::VerticalScrollState;
    ///
    /// let state = VerticalScrollState::new();
    /// assert_eq!(state.top(), 0);
    /// ```
    #[must_use]
    #[inline]
    pub const fn new() -> Self {
        Self { top: 0, max_top: 0 }
    }

    /// Returns the current top visible item index.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::VerticalScrollState;
    ///
    /// assert_eq!(VerticalScrollState::new().top(), 0);
    /// ```
    #[must_use]
    #[inline]
    pub const fn top(&self) -> usize {
        self.top
    }

    /// Returns the maximum valid top index (`content_len.saturating_sub(viewport_height)`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::VerticalScrollState;
    ///
    /// let mut s = VerticalScrollState::new();
    /// s.update(0, 30, 10);
    /// assert_eq!(s.max_top(), 20);
    /// ```
    #[must_use]
    #[inline]
    pub const fn max_top(&self) -> usize {
        self.max_top
    }

    /// Updates the scroll offset, clamping `desired_top` to `0..=content_len - viewport_len`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::VerticalScrollState;
    ///
    /// let mut s = VerticalScrollState::new();
    /// assert_eq!(s.update(99, 25, 10), 15);
    /// ```
    #[inline]
    pub fn update(&mut self, desired_top: usize, content_len: usize, viewport_len: usize) -> usize {
        self.max_top = content_len.saturating_sub(viewport_len);
        self.top = desired_top.min(self.max_top);
        self.top
    }

    /// Adjusts `top` minimally so `selected_index` is visible within `viewport_len`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::VerticalScrollState;
    ///
    /// let mut s = VerticalScrollState::new();
    /// s.ensure_visible(12, 30, 10);
    /// assert_eq!(s.top(), 3);
    /// s.ensure_visible(1, 30, 10);
    /// assert_eq!(s.top(), 1);
    /// ```
    #[inline]
    pub fn ensure_visible(
        &mut self,
        selected_index: usize,
        content_len: usize,
        viewport_len: usize,
    ) -> usize {
        if viewport_len == 0 {
            self.top = 0;
            self.max_top = 0;
            return 0;
        }
        self.max_top = content_len.saturating_sub(viewport_len);
        if selected_index < self.top {
            self.top = selected_index;
        } else if selected_index >= self.top.saturating_add(viewport_len) {
            self.top = selected_index
                .saturating_add(1)
                .saturating_sub(viewport_len);
        }
        self.top = self.top.min(self.max_top);
        self.top
    }

    /// Centers `selected_index` within `viewport_len`, clamped to `0..=max_top`.
    /// Matches the centered scrolling window in `ref/dashboard/de_widgets.go` (`MenuList`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::VerticalScrollState;
    ///
    /// let mut s = VerticalScrollState::new();
    /// assert_eq!(s.center_on(10, 30, 10), 5);
    /// ```
    #[inline]
    pub fn center_on(
        &mut self,
        selected_index: usize,
        content_len: usize,
        viewport_len: usize,
    ) -> usize {
        if content_len <= viewport_len || viewport_len == 0 {
            self.top = 0;
            self.max_top = 0;
            return 0;
        }
        self.max_top = content_len.saturating_sub(viewport_len);
        let half = viewport_len / 2;
        self.top = selected_index.saturating_sub(half).min(self.max_top);
        self.top
    }
}

/// 1/8th sub-cell fractional vertical scrollbar renderer inspired by Textual.
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Buffer, ColorPalette, ColumnIndex, Rect, RowIndex, TerminalHeight, TerminalWidth};
/// use tui_rs::widgets::SubCellScrollbar;
///
/// let area = Rect::from_size(TerminalWidth::new(5), TerminalHeight::new(8));
/// let mut buf = Buffer::empty(area);
/// SubCellScrollbar::new(100, 20, 30)
///     .render(&mut buf, ColumnIndex::new(4), RowIndex::new(0), 8, &ColorPalette::DEFAULT);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct SubCellScrollbar {
    virtual_size: usize,
    window_size: usize,
    position: usize,
    tty_mode: bool,
}

impl SubCellScrollbar {
    /// Creates a new [`SubCellScrollbar`].
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::SubCellScrollbar;
    ///
    /// let sb = SubCellScrollbar::new(50, 10, 5);
    /// assert!(sb.should_render());
    /// ```
    #[must_use]
    #[inline]
    pub const fn new(virtual_size: usize, window_size: usize, position: usize) -> Self {
        Self {
            virtual_size,
            window_size,
            position,
            tty_mode: false,
        }
    }

    /// Configures whether TTY-safe glyphs (`'█'` and `'│'`) are used instead of
    /// 1/8th fractional blocks.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::SubCellScrollbar;
    ///
    /// let sb = SubCellScrollbar::new(50, 10, 0).with_tty_mode(true);
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_tty_mode(mut self, tty_mode: bool) -> Self {
        self.tty_mode = tty_mode;
        self
    }

    /// Returns `true` if `virtual_size > window_size` (scrolling is active).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::SubCellScrollbar;
    ///
    /// assert!(!SubCellScrollbar::new(10, 10, 0).should_render());
    /// assert!(SubCellScrollbar::new(11, 10, 0).should_render());
    /// ```
    #[must_use]
    #[inline]
    pub const fn should_render(&self) -> bool {
        self.virtual_size > self.window_size && self.window_size > 0
    }

    /// Renders the vertical scrollbar column at `col` spanning `row..row + track_height`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Buffer, ColorPalette, ColumnIndex, Rect, RowIndex, TerminalHeight, TerminalWidth};
    /// use tui_rs::widgets::SubCellScrollbar;
    ///
    /// let area = Rect::from_size(TerminalWidth::new(2), TerminalHeight::new(6));
    /// let mut buf = Buffer::empty(area);
    /// SubCellScrollbar::new(60, 15, 10)
    ///     .render(&mut buf, ColumnIndex::new(1), RowIndex::new(0), 6, &ColorPalette::DEFAULT);
    /// ```
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "sub-cell 1/8th unit calculations bounded by u16 track_height"
    )]
    pub fn render(
        &self,
        buffer: &mut Buffer,
        col: ColumnIndex,
        row: RowIndex,
        track_height: u16,
        palette: &ColorPalette,
    ) {
        if track_height == 0 || !self.should_render() {
            return;
        }
        let total_sub_units = usize::from(track_height).saturating_mul(8);
        let ratio = (self.window_size as f64) / (self.virtual_size as f64);
        let thumb_sub_size = ((total_sub_units as f64) * ratio).round() as usize;
        let thumb_sub_size = thumb_sub_size.clamp(8, total_sub_units);

        let max_pos = self.virtual_size.saturating_sub(self.window_size).max(1);
        let clamped_pos = self.position.min(max_pos);
        let pos_ratio = (clamped_pos as f64) / (max_pos as f64);
        let travel_sub = total_sub_units.saturating_sub(thumb_sub_size);
        let thumb_start_sub = ((travel_sub as f64) * pos_ratio).round() as usize;
        let thumb_end_sub = thumb_start_sub.saturating_add(thumb_sub_size);

        let track_color = palette.dim;
        let thumb_color = palette.accent;

        for cell_idx in 0..usize::from(track_height) {
            let cell_start = cell_idx * 8;
            let cell_end = cell_start + 8;
            let ry = row.saturating_add(cell_idx as u16);

            // Check overlap between [cell_start, cell_end) and [thumb_start_sub, thumb_end_sub)
            let overlap_start = cell_start.max(thumb_start_sub);
            let overlap_end = cell_end.min(thumb_end_sub);

            if overlap_start >= overlap_end {
                let _ = buffer.set_cell(col, ry, "│", Style::new().with_fg(track_color));
            } else if self.tty_mode || (overlap_start == cell_start && overlap_end == cell_end) {
                let _ = buffer.set_cell(col, ry, "█", Style::new().with_fg(thumb_color));
            } else if overlap_end == cell_end {
                // Bottom portion of cell is thumb -> lower block with fg = thumb, bg = track
                let eighths = (overlap_end - overlap_start).clamp(1, 8);
                let _ = buffer.set_cell(
                    col,
                    ry,
                    LOWER_EIGHTHS[eighths - 1],
                    Style::new().with_fg(thumb_color),
                );
            } else {
                // Top portion of cell is thumb -> lower block inverted or full thumb
                let eighths = (overlap_end - overlap_start).clamp(1, 8);
                let _ = buffer.set_cell(
                    col,
                    ry,
                    LOWER_EIGHTHS[eighths - 1],
                    Style::new().with_fg(thumb_color),
                );
            }
        }
    }
}
