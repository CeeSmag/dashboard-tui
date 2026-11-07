//! Compact half-block QR matrix renderer (`HalfBlockQr`).
//!
//! Ports `RenderHalfBlockQR` from `ref/dashboard/de_monitors.go`.
//! Packs two vertical QR modules into each terminal cell using the Unicode
//! upper-half block `'▀'` (`U+2580`) with foreground = top module and
//! background = bottom module, halving the required vertical row count while
//! preserving a 1:1 square aspect ratio in typical 1:2 terminal fonts.

use crate::core::buffer::Buffer;
use crate::core::color::Color;
use crate::core::coordinates::{ColumnIndex, RowIndex};
use crate::core::style::Style;

/// Renders a boolean QR code matrix using half-block (`'▀'`) cell packing.
///
/// # Examples
///
/// ```
/// use tui_rs::components::HalfBlockQr;
/// use tui_rs::core::{Buffer, ColumnIndex, Rect, RowIndex, TerminalHeight, TerminalWidth};
///
/// let modules = [
///     true,  false, true,
///     false, true,  false,
///     true,  false, true,
/// ];
/// let qr = HalfBlockQr::new(&modules, 3).with_quiet_zone(1);
/// assert_eq!(qr.rendered_width(), 5);
/// assert_eq!(qr.rendered_height(), 3);
///
/// let area = Rect::from_size(TerminalWidth::new(10), TerminalHeight::new(5));
/// let mut buf = Buffer::empty(area);
/// qr.render(&mut buf, ColumnIndex::new(0), RowIndex::new(0));
/// ```
#[derive(Debug, Clone, Copy)]
pub struct HalfBlockQr<'a> {
    modules: &'a [bool],
    dimension: usize,
    quiet_zone: usize,
    dark_color: Color,
    light_color: Color,
}

impl<'a> HalfBlockQr<'a> {
    /// Creates a new [`HalfBlockQr`] from a flat row-major slice of `dimension × dimension`
    /// dark (`true`) / light (`false`) modules.
    ///
    /// Defaults to a 2-module quiet zone, black dark modules (`Rgb(0, 0, 0)`), and
    /// white light modules (`Rgb(255, 255, 255)`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::HalfBlockQr;
    ///
    /// let qr = HalfBlockQr::new(&[true, false, false, true], 2);
    /// assert_eq!(qr.dimension(), 2);
    /// ```
    #[must_use]
    #[inline]
    pub const fn new(modules: &'a [bool], dimension: usize) -> Self {
        Self {
            modules,
            dimension,
            quiet_zone: 2,
            dark_color: Color::Rgb(0, 0, 0),
            light_color: Color::Rgb(255, 255, 255),
        }
    }

    /// Sets the quiet-zone border thickness in modules (default `2`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::HalfBlockQr;
    ///
    /// let qr = HalfBlockQr::new(&[true], 1).with_quiet_zone(1);
    /// assert_eq!(qr.rendered_width(), 3);
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_quiet_zone(mut self, quiet_zone: usize) -> Self {
        self.quiet_zone = quiet_zone;
        self
    }

    /// Sets custom colors for dark (`true`) and light (`false`) QR modules.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::HalfBlockQr;
    /// use tui_rs::core::Color;
    ///
    /// let qr = HalfBlockQr::new(&[true], 1).with_colors(Color::Black, Color::White);
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_colors(mut self, dark_color: Color, light_color: Color) -> Self {
        self.dark_color = dark_color;
        self.light_color = light_color;
        self
    }

    /// Returns the side dimension of the QR matrix (excluding quiet zone).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::HalfBlockQr;
    ///
    /// assert_eq!(HalfBlockQr::new(&[true; 9], 3).dimension(), 3);
    /// ```
    #[must_use]
    #[inline]
    pub const fn dimension(&self) -> usize {
        self.dimension
    }

    /// Returns the total rendered width in terminal columns (including quiet zone).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::HalfBlockQr;
    ///
    /// let qr = HalfBlockQr::new(&[true; 9], 3).with_quiet_zone(2);
    /// assert_eq!(qr.rendered_width(), 7);
    /// ```
    #[must_use]
    #[inline]
    #[expect(clippy::cast_possible_truncation, reason = "QR dimensions fit in u16")]
    pub const fn rendered_width(&self) -> u16 {
        (self.dimension.saturating_add(self.quiet_zone.saturating_mul(2))) as u16
    }

    /// Returns the total rendered height in terminal rows (2 vertical modules per row).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::HalfBlockQr;
    ///
    /// let qr = HalfBlockQr::new(&[true; 9], 3).with_quiet_zone(2);
    /// assert_eq!(qr.rendered_height(), 4);
    /// ```
    #[must_use]
    #[inline]
    #[expect(clippy::cast_possible_truncation, reason = "QR dimensions fit in u16")]
    pub const fn rendered_height(&self) -> u16 {
        let total = self.dimension.saturating_add(self.quiet_zone.saturating_mul(2));
        total.div_ceil(2) as u16
    }

    #[inline]
    fn is_dark(&self, row_with_qz: usize, col_with_qz: usize) -> bool {
        if row_with_qz < self.quiet_zone || col_with_qz < self.quiet_zone {
            return false;
        }
        let r = row_with_qz - self.quiet_zone;
        let c = col_with_qz - self.quiet_zone;
        if r >= self.dimension || c >= self.dimension {
            return false;
        }
        let idx = r.saturating_mul(self.dimension).saturating_add(c);
        self.modules.get(idx).copied().unwrap_or(false)
    }

    /// Renders the half-block QR matrix into `buffer` with top-left corner at `(col, row)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::HalfBlockQr;
    /// use tui_rs::core::{Buffer, ColumnIndex, Rect, RowIndex, TerminalHeight, TerminalWidth};
    ///
    /// let area = Rect::from_size(TerminalWidth::new(10), TerminalHeight::new(6));
    /// let mut buf = Buffer::empty(area);
    /// let qr = HalfBlockQr::new(&[true, false, false, true], 2);
    /// qr.render(&mut buf, ColumnIndex::new(0), RowIndex::new(0));
    /// ```
    #[expect(clippy::cast_possible_truncation, reason = "QR coordinates fit in u16")]
    pub fn render(&self, buffer: &mut Buffer, col: ColumnIndex, row: RowIndex) {
        if self.dimension == 0 {
            return;
        }
        let total = self.dimension.saturating_add(self.quiet_zone.saturating_mul(2));
        let mut r = 0;
        let mut out_row = row;
        while r < total {
            for c in 0..total {
                let top_dark = self.is_dark(r, c);
                let bot_dark = if r + 1 < total {
                    self.is_dark(r + 1, c)
                } else {
                    false
                };
                let fg = if top_dark {
                    self.dark_color
                } else {
                    self.light_color
                };
                let bg = if bot_dark {
                    self.dark_color
                } else {
                    self.light_color
                };
                let style = Style::new().with_fg(fg).with_bg(bg);
                let _ = buffer.set_cell(col.saturating_add(c as u16), out_row, "▀", style);
            }
            out_row = out_row.saturating_add(1);
            r += 2;
        }
    }
}
