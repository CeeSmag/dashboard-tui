//! Horizontal progress and capacity meter (`GradientMeter`) with smooth gradient fill.
//!
//! Ports `GradientBar` from `ref/dashboard/de_monitors.go` (`[■■■■□□□□]`)
//! and `btop`'s `meter` class, writing directly into a [`Buffer`] without
//! heap string allocations.

use crate::core::buffer::Buffer;
use crate::core::coordinates::{ColumnIndex, RowIndex};
use crate::core::palette::ColorPalette;
use crate::core::style::Style;

/// Horizontal capacity/utilization meter with smooth green-amber-red gradient fill.
///
/// # Examples
///
/// ```
/// use tui_rs::components::GradientMeter;
/// use tui_rs::core::{Buffer, ColorPalette, ColumnIndex, Rect, RowIndex, TerminalHeight, TerminalWidth};
///
/// let area = Rect::from_size(TerminalWidth::new(20), TerminalHeight::new(1));
/// let mut buf = Buffer::empty(area);
/// let meter = GradientMeter::new(0.65, 12);
/// let next_col = meter.render(&mut buf, ColumnIndex::new(0), RowIndex::new(0), &ColorPalette::DEFAULT);
/// assert_eq!(next_col.get(), 12);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct GradientMeter {
    fraction: f64,
    total_width: u16,
    tty_mode: bool,
}

impl GradientMeter {
    /// Creates a new [`GradientMeter`] for `fraction` (`0.0..=1.0`) and `total_width`
    /// columns (including the outer `[` and `]` brackets).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::GradientMeter;
    ///
    /// let meter = GradientMeter::new(0.5, 10);
    /// assert_eq!(meter.total_width(), 10);
    /// ```
    #[must_use]
    #[inline]
    pub const fn new(fraction: f64, total_width: u16) -> Self {
        let clamped = if fraction < 0.0 {
            0.0
        } else if fraction > 1.0 {
            1.0
        } else {
            fraction
        };
        Self {
            fraction: clamped,
            total_width,
            tty_mode: false,
        }
    }

    /// Configures whether CP437/TTY-safe glyphs (`'#'` and `'.'`) are used instead
    /// of Unicode squares (`'■'` and `'□'`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::GradientMeter;
    ///
    /// let meter = GradientMeter::new(0.5, 10).with_tty_mode(true);
    /// assert!(meter.is_tty_mode());
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_tty_mode(mut self, tty_mode: bool) -> Self {
        self.tty_mode = tty_mode;
        self
    }

    /// Returns the clamped fill fraction (`0.0..=1.0`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::GradientMeter;
    ///
    /// assert_eq!(GradientMeter::new(1.5, 10).fraction(), 1.0);
    /// ```
    #[must_use]
    #[inline]
    pub const fn fraction(&self) -> f64 {
        self.fraction
    }

    /// Returns the total column width of the meter (including brackets).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::GradientMeter;
    ///
    /// assert_eq!(GradientMeter::new(0.5, 14).total_width(), 14);
    /// ```
    #[must_use]
    #[inline]
    pub const fn total_width(&self) -> u16 {
        self.total_width
    }

    /// Returns `true` if TTY-safe ASCII/CP437 glyphs are enabled.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::GradientMeter;
    ///
    /// assert!(!GradientMeter::new(0.5, 10).is_tty_mode());
    /// ```
    #[must_use]
    #[inline]
    pub const fn is_tty_mode(&self) -> bool {
        self.tty_mode
    }

    /// Renders `[■■■■□□□□]` starting at `(col, row)` in `buffer` and returns the
    /// column index immediately following the meter.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::GradientMeter;
    /// use tui_rs::core::{Buffer, ColorPalette, ColumnIndex, Rect, RowIndex, TerminalHeight, TerminalWidth};
    ///
    /// let area = Rect::from_size(TerminalWidth::new(15), TerminalHeight::new(1));
    /// let mut buf = Buffer::empty(area);
    /// let end_col = GradientMeter::new(0.5, 10).render(
    ///     &mut buf,
    ///     ColumnIndex::new(0),
    ///     RowIndex::new(0),
    ///     &ColorPalette::DEFAULT,
    /// );
    /// assert_eq!(end_col.get(), 10);
    /// assert_eq!(buf.to_visual_lines()[0].trim(), "[■■■■□□□□]");
    /// ```
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "bar width is bounded by u16"
    )]
    pub fn render(
        &self,
        buffer: &mut Buffer,
        col: ColumnIndex,
        row: RowIndex,
        palette: &ColorPalette,
    ) -> ColumnIndex {
        let bar_w = self.total_width.saturating_sub(2).max(2);
        let mut filled = ((self.fraction * f64::from(bar_w)).round() as u16).min(bar_w);
        if self.fraction > 0.0 && filled == 0 {
            filled = 1;
        }

        let dim_style = Style::new().with_fg(palette.dim);
        let lut = palette.gradient_lut();
        let filled_glyph = if self.tty_mode { "#" } else { "■" };
        let empty_glyph = if self.tty_mode { "." } else { "□" };

        let mut cx = col;
        let _ = buffer.set_cell(cx, row, "[", dim_style);
        cx = cx.saturating_add(1);

        for i in 0..bar_w {
            if i < filled {
                let ratio = if bar_w <= 1 {
                    1.0
                } else {
                    f32::from(i) / f32::from(bar_w - 1)
                };
                let color = lut.at_ratio(ratio);
                let _ = buffer.set_cell(cx, row, filled_glyph, Style::new().with_fg(color));
            } else {
                let _ = buffer.set_cell(cx, row, empty_glyph, dim_style);
            }
            cx = cx.saturating_add(1);
        }

        let _ = buffer.set_cell(cx, row, "]", dim_style);
        cx.saturating_add(1)
    }
}
