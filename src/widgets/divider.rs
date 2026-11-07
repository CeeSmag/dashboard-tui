//! Horizontal section divider (`LabeledDivider`) inside a panel (`├── LABEL ────┤`).
//!
//! Ports `PanelSubHeader` from `ref/dashboard/de_panel.go`.

use crate::core::buffer::Buffer;
use crate::core::coordinates::{ColumnIndex, RowIndex};
use crate::core::grapheme::display_width;
use crate::core::palette::ColorPalette;
use crate::core::style::Style;

/// Horizontal labeled sub-header rule (`── LABEL ──────`).
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Buffer, ColorPalette, ColumnIndex, Rect, RowIndex, TerminalHeight, TerminalWidth};
/// use tui_rs::widgets::LabeledDivider;
///
/// let area = Rect::from_size(TerminalWidth::new(30), TerminalHeight::new(1));
/// let mut buf = Buffer::empty(area);
/// LabeledDivider::new("THROUGHPUT")
///     .render(&mut buf, ColumnIndex::new(0), RowIndex::new(0), 30, &ColorPalette::DEFAULT);
/// assert!(buf.to_visual_lines()[0].contains("THROUGHPUT"));
/// ```
#[derive(Debug, Clone, Copy)]
pub struct LabeledDivider<'a> {
    label: &'a str,
    tty_mode: bool,
}

impl<'a> LabeledDivider<'a> {
    /// Creates a new [`LabeledDivider`] with the given label.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::LabeledDivider;
    ///
    /// let div = LabeledDivider::new("STATUS");
    /// assert_eq!(div.label(), "STATUS");
    /// ```
    #[must_use]
    #[inline]
    pub const fn new(label: &'a str) -> Self {
        Self {
            label,
            tty_mode: false,
        }
    }

    /// Configures whether ASCII `'-'` is used instead of `'─'`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::LabeledDivider;
    ///
    /// let div = LabeledDivider::new("STATUS").with_tty_mode(true);
    /// assert!(div.is_tty_mode());
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_tty_mode(mut self, tty_mode: bool) -> Self {
        self.tty_mode = tty_mode;
        self
    }

    /// Returns the label string.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::LabeledDivider;
    ///
    /// assert_eq!(LabeledDivider::new("SYNC").label(), "SYNC");
    /// ```
    #[must_use]
    #[inline]
    pub const fn label(&self) -> &'a str {
        self.label
    }

    /// Returns `true` if TTY mode is enabled.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::LabeledDivider;
    ///
    /// assert!(!LabeledDivider::new("SYNC").is_tty_mode());
    /// ```
    #[must_use]
    #[inline]
    pub const fn is_tty_mode(&self) -> bool {
        self.tty_mode
    }

    /// Renders `── LABEL ────────` starting at `(col, row)` across `width` columns.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Buffer, ColorPalette, ColumnIndex, Rect, RowIndex, TerminalHeight, TerminalWidth};
    /// use tui_rs::widgets::LabeledDivider;
    ///
    /// let area = Rect::from_size(TerminalWidth::new(25), TerminalHeight::new(1));
    /// let mut buf = Buffer::empty(area);
    /// LabeledDivider::new("LOGS")
    ///     .render(&mut buf, ColumnIndex::new(0), RowIndex::new(0), 25, &ColorPalette::DEFAULT);
    /// ```
    pub fn render(
        &self,
        buffer: &mut Buffer,
        col: ColumnIndex,
        row: RowIndex,
        width: u16,
        palette: &ColorPalette,
    ) {
        if width == 0 {
            return;
        }
        let rule_char = if self.tty_mode { "-" } else { "─" };
        let rule_style = Style::new().with_fg(palette.border);
        let label_style = Style::new().with_fg(palette.accent).bold();

        for i in 0..width {
            let _ = buffer.set_cell(col.saturating_add(i), row, rule_char, rule_style);
        }

        if self.label.is_empty() || width < 6 {
            return;
        }
        let mut cx = col.saturating_add(2);
        let _ = buffer.set_cell(cx, row, " ", rule_style);
        cx = cx.saturating_add(1);
        let max_label_w = width.saturating_sub(5);
        cx = buffer.set_string(cx, row, self.label, label_style, max_label_w);
        if (cx.get() - col.get()) < width && display_width(self.label).as_u16() <= max_label_w {
            let _ = buffer.set_cell(cx, row, " ", rule_style);
        }
    }
}
