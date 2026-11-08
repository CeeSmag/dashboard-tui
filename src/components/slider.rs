//! Discrete interactive slider bar (`SliderBar`) with adaptive segment compression.
//!
//! Ports `SliderBar` from `ref/dashboard/de_widgets.go`:
//! `0 (min) [■■■■■■●──────] 6 (max)`
//! Automatically shrinks segment width (`3` -> `2` -> `1`) to fit narrow
//! panels without overflowing.

use core::fmt::Write;

use crate::core::buffer::{Buffer, StackStr};
use crate::core::coordinates::{ColumnIndex, RowIndex};
use crate::core::palette::ColorPalette;
use crate::core::style::Style;

/// Discrete horizontal slider component (`min (min) [■■●──] max (max)`).
///
/// # Examples
///
/// ```
/// use dashboard_tui::components::SliderBar;
/// use dashboard_tui::core::{Buffer, ColorPalette, ColumnIndex, Rect, RowIndex, TerminalHeight, TerminalWidth};
///
/// let area = Rect::from_size(TerminalWidth::new(50), TerminalHeight::new(1));
/// let mut buf = Buffer::empty(area);
/// let slider = SliderBar::new(2, 0, 6);
/// slider.render(&mut buf, ColumnIndex::new(0), RowIndex::new(0), 50, &ColorPalette::DEFAULT);
/// assert!(buf.to_visual_lines()[0].contains('●'));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SliderBar {
    value: i32,
    min: i32,
    max: i32,
    tty_mode: bool,
}

impl SliderBar {
    /// Creates a new [`SliderBar`] with `value` clamped to `min..=max`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::components::SliderBar;
    ///
    /// let slider = SliderBar::new(4, 0, 10);
    /// assert_eq!(slider.value(), 4);
    /// ```
    #[must_use]
    #[inline]
    pub const fn new(value: i32, min: i32, max: i32) -> Self {
        let effective_max = if max <= min { min + 1 } else { max };
        let clamped = if value < min {
            min
        } else if value > effective_max {
            effective_max
        } else {
            value
        };
        Self {
            value: clamped,
            min,
            max: effective_max,
            tty_mode: false,
        }
    }

    /// Configures whether TTY-safe glyphs (`'#', 'O', '-'`) are used instead of
    /// `'■', '●', '─'`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::components::SliderBar;
    ///
    /// let slider = SliderBar::new(1, 0, 6).with_tty_mode(true);
    /// assert!(slider.is_tty_mode());
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_tty_mode(mut self, tty_mode: bool) -> Self {
        self.tty_mode = tty_mode;
        self
    }

    /// Returns the current clamped slider value.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::components::SliderBar;
    ///
    /// assert_eq!(SliderBar::new(99, 0, 6).value(), 6);
    /// ```
    #[must_use]
    #[inline]
    pub const fn value(&self) -> i32 {
        self.value
    }

    /// Returns `true` if TTY mode glyphs are active.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::components::SliderBar;
    ///
    /// assert!(!SliderBar::new(1, 0, 6).is_tty_mode());
    /// ```
    #[must_use]
    #[inline]
    pub const fn is_tty_mode(&self) -> bool {
        self.tty_mode
    }

    /// Renders the slider bar at `(col, row)` within `max_width` columns and returns
    /// the column index immediately following the rendered slider.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::components::SliderBar;
    /// use dashboard_tui::core::{Buffer, ColorPalette, ColumnIndex, Rect, RowIndex, TerminalHeight, TerminalWidth};
    ///
    /// let area = Rect::from_size(TerminalWidth::new(40), TerminalHeight::new(1));
    /// let mut buf = Buffer::empty(area);
    /// let slider = SliderBar::new(3, 0, 6);
    /// let next_col = slider.render(&mut buf, ColumnIndex::new(0), RowIndex::new(0), 40, &ColorPalette::DEFAULT);
    /// assert!(next_col.get() > 10);
    /// ```
    #[expect(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        reason = "max > min is enforced in constructor"
    )]
    pub fn render(
        &self,
        buffer: &mut Buffer,
        col: ColumnIndex,
        row: RowIndex,
        max_width: u16,
        palette: &ColorPalette,
    ) -> ColumnIndex {
        let steps = (self.max - self.min) as u16;
        let cur = (self.value - self.min) as u16;

        let mut left_label = StackStr::<24>::new();
        let _ = write!(left_label, "{} (min) ", self.min);
        let mut right_label = StackStr::<24>::new();
        let _ = write!(right_label, " {} (max)", self.max);

        let overhead = (left_label.len() + right_label.len() + 3) as u16;
        let mut seg_w: u16 = 3;
        while seg_w > 1 && steps.saturating_mul(seg_w).saturating_add(overhead) > max_width {
            seg_w -= 1;
        }

        let muted_style = Style::new().with_fg(palette.muted);
        let dim_style = Style::new().with_fg(palette.dim);
        let filled_style = Style::new().with_fg(palette.accent);
        let thumb_style = Style::new().with_fg(palette.accent2).bold();

        let filled_glyph = if self.tty_mode { "#" } else { "■" };
        let thumb_glyph = if self.tty_mode { "O" } else { "●" };
        let empty_glyph = if self.tty_mode { "-" } else { "─" };

        let right_edge = col.get().saturating_add(max_width);
        let mut cx = buffer.set_string(
            col,
            row,
            left_label.as_str(),
            muted_style,
            right_edge.saturating_sub(col.get()),
        );
        cx = buffer.set_string(cx, row, "[", dim_style, right_edge.saturating_sub(cx.get()));

        let filled_cells = cur.saturating_mul(seg_w);
        let empty_cells = steps.saturating_sub(cur).saturating_mul(seg_w);

        for _ in 0..filled_cells {
            if cx.get() >= right_edge {
                break;
            }
            let _ = buffer.set_cell(cx, row, filled_glyph, filled_style);
            cx = cx.saturating_add(1);
        }
        if cx.get() < right_edge {
            let _ = buffer.set_cell(cx, row, thumb_glyph, thumb_style);
            cx = cx.saturating_add(1);
        }
        for _ in 0..empty_cells {
            if cx.get() >= right_edge {
                break;
            }
            let _ = buffer.set_cell(cx, row, empty_glyph, dim_style);
            cx = cx.saturating_add(1);
        }

        cx = buffer.set_string(cx, row, "]", dim_style, right_edge.saturating_sub(cx.get()));
        buffer.set_string(
            cx,
            row,
            right_label.as_str(),
            muted_style,
            right_edge.saturating_sub(cx.get()),
        )
    }
}
