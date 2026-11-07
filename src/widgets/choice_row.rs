//! Modal choice row with indented muted detail description (`ChoiceRow`).
//!
//! Ports `ModalChoiceRow` from `ref/dashboard/de_widgets.go`:
//! - Row 1: `❯ [1] Label                  Badge` (highlighted when selected)
//! - Row 2 (optional): `      Muted detail text`

use core::fmt::Write;

use crate::core::buffer::{Buffer, StackStr};
use crate::core::color::Color;
use crate::core::coordinates::{ColumnIndex, RowIndex, TerminalWidth};
use crate::core::grapheme::{display_width, truncate_with_ellipsis_parts};
use crate::core::palette::ColorPalette;
use crate::core::style::Style;

/// Interactive choice row with optional second-line detail text for modals and pickers.
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Buffer, ColorPalette, ColumnIndex, Rect, RowIndex, TerminalHeight, TerminalWidth};
/// use tui_rs::widgets::ChoiceRow;
///
/// let area = Rect::from_size(TerminalWidth::new(50), TerminalHeight::new(2));
/// let mut buf = Buffer::empty(area);
/// let rows_used = ChoiceRow::new(1, "Catppuccin Mocha")
///     .with_badge("● ACTIVE")
///     .with_detail("Warm pastel dark palette")
///     .with_selected(true)
///     .render(&mut buf, ColumnIndex::new(0), RowIndex::new(0), 50, &ColorPalette::DEFAULT);
/// assert_eq!(rows_used, 2);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChoiceRow<'a> {
    number: u16,
    label: &'a str,
    badge: &'a str,
    badge_color: Option<Color>,
    detail: &'a str,
    selected: bool,
    tty_mode: bool,
}

impl<'a> ChoiceRow<'a> {
    /// Creates a new [`ChoiceRow`] with shortcut `number` (`0` omits `[n]`) and `label`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::ChoiceRow;
    ///
    /// let row = ChoiceRow::new(1, "Default Dark");
    /// assert_eq!(row.label(), "Default Dark");
    /// ```
    #[must_use]
    #[inline]
    pub const fn new(number: u16, label: &'a str) -> Self {
        Self {
            number,
            label,
            badge: "",
            badge_color: None,
            detail: "",
            selected: false,
            tty_mode: false,
        }
    }

    /// Sets the right-aligned status badge string.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::ChoiceRow;
    ///
    /// let row = ChoiceRow::new(1, "Nord").with_badge("ACTIVE");
    /// assert_eq!(row.badge(), "ACTIVE");
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_badge(mut self, badge: &'a str) -> Self {
        self.badge = badge;
        self
    }

    /// Sets a custom foreground color for the status badge.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::Color;
    /// use tui_rs::widgets::ChoiceRow;
    ///
    /// let row = ChoiceRow::new(1, "Nord").with_badge_color(Color::Green);
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_badge_color(mut self, color: Color) -> Self {
        self.badge_color = Some(color);
        self
    }

    /// Sets an indented second-row description string.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::ChoiceRow;
    ///
    /// let row = ChoiceRow::new(1, "Nord").with_detail("Arctic bluish-gray");
    /// assert_eq!(row.detail(), "Arctic bluish-gray");
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_detail(mut self, detail: &'a str) -> Self {
        self.detail = detail;
        self
    }

    /// Configures whether this choice row is currently selected.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::ChoiceRow;
    ///
    /// let row = ChoiceRow::new(1, "Nord").with_selected(true);
    /// assert!(row.is_selected());
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Configures whether ASCII `'>'` is used instead of `'❯'`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::ChoiceRow;
    ///
    /// let row = ChoiceRow::new(1, "Nord").with_tty_mode(true);
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_tty_mode(mut self, tty_mode: bool) -> Self {
        self.tty_mode = tty_mode;
        self
    }

    /// Returns the primary label text.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::ChoiceRow;
    ///
    /// assert_eq!(ChoiceRow::new(1, "Nord").label(), "Nord");
    /// ```
    #[must_use]
    #[inline]
    pub const fn label(&self) -> &'a str {
        self.label
    }

    /// Returns the right-aligned badge text.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::ChoiceRow;
    ///
    /// assert_eq!(ChoiceRow::new(1, "Nord").with_badge("ON").badge(), "ON");
    /// ```
    #[must_use]
    #[inline]
    pub const fn badge(&self) -> &'a str {
        self.badge
    }

    /// Returns the optional second-line detail text.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::ChoiceRow;
    ///
    /// assert_eq!(ChoiceRow::new(1, "Nord").detail(), "");
    /// ```
    #[must_use]
    #[inline]
    pub const fn detail(&self) -> &'a str {
        self.detail
    }

    /// Returns `true` if this row is selected.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::ChoiceRow;
    ///
    /// assert!(!ChoiceRow::new(1, "Nord").is_selected());
    /// ```
    #[must_use]
    #[inline]
    pub const fn is_selected(&self) -> bool {
        self.selected
    }

    /// Renders the choice row (and optional detail row) starting at `(col, row)`
    /// across `width` columns, returning the number of vertical rows consumed (`1` or `2`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Buffer, ColorPalette, ColumnIndex, Rect, RowIndex, TerminalHeight, TerminalWidth};
    /// use tui_rs::widgets::ChoiceRow;
    ///
    /// let area = Rect::from_size(TerminalWidth::new(40), TerminalHeight::new(2));
    /// let mut buf = Buffer::empty(area);
    /// let used = ChoiceRow::new(1, "Option A")
    ///     .render(&mut buf, ColumnIndex::new(0), RowIndex::new(0), 40, &ColorPalette::DEFAULT);
    /// assert_eq!(used, 1);
    /// ```
    #[expect(
        clippy::cast_possible_truncation,
        clippy::too_many_lines,
        reason = "widths bounded by u16; cohesive single/two-line choice renderer"
    )]
    pub fn render(
        &self,
        buffer: &mut Buffer,
        col: ColumnIndex,
        row: RowIndex,
        width: u16,
        palette: &ColorPalette,
    ) -> u16 {
        if width < 6 {
            return 0;
        }
        let right_edge = col.get().saturating_add(width);

        if self.selected {
            let bg = Style::new().with_bg(palette.highlight_bg);
            for offset in 0..width {
                let _ = buffer.set_cell(col.saturating_add(offset), row, " ", bg);
            }
        }

        let apply_bg = |s: Style| {
            if self.selected {
                s.with_bg(palette.highlight_bg)
            } else {
                s
            }
        };

        let cursor_str = if self.selected {
            if self.tty_mode { "> " } else { "❯ " }
        } else {
            "  "
        };
        let mut num_tag = StackStr::<16>::new();
        if self.number > 0 {
            let _ = write!(num_tag, "[{}] ", self.number);
        }

        let prefix_w = 2 + num_tag.len();
        let badge_w = display_width(self.badge).as_usize();
        let avail_for_label = usize::from(width)
            .saturating_sub(prefix_w)
            .saturating_sub(if badge_w > 0 { badge_w + 2 } else { 0 })
            .max(1);

        let (lbl_pre, lbl_ell, lbl_tw) =
            truncate_with_ellipsis_parts(self.label, TerminalWidth::new(avail_for_label as u16));
        let lbl_rendered_w = lbl_tw.as_usize();

        let cursor_style = apply_bg(Style::new().with_fg(palette.accent2).bold());
        let num_style = if self.selected {
            apply_bg(Style::new().with_fg(palette.accent2).bold())
        } else {
            apply_bg(Style::new().with_fg(palette.accent).bold())
        };
        let label_style = if self.selected {
            apply_bg(Style::new().with_fg(palette.accent2).bold())
        } else {
            apply_bg(Style::new().with_fg(palette.text))
        };

        let mut cx = buffer.set_string(col, row, cursor_str, cursor_style, 2);
        if !num_tag.is_empty() {
            cx = buffer.set_string(
                cx,
                row,
                num_tag.as_str(),
                num_style,
                right_edge.saturating_sub(cx.get()),
            );
        }
        cx = buffer.set_string(
            cx,
            row,
            lbl_pre,
            label_style,
            right_edge.saturating_sub(cx.get()),
        );
        if lbl_ell {
            let _ = buffer.set_string(
                cx,
                row,
                "…",
                label_style,
                right_edge.saturating_sub(cx.get()),
            );
        }

        if badge_w > 0 {
            let used = prefix_w + lbl_rendered_w + badge_w;
            let gap = usize::from(width).saturating_sub(used).max(1);
            let badge_x = col.saturating_add((prefix_w + lbl_rendered_w + gap) as u16);
            if badge_x.get() < right_edge {
                let b_fg = self.badge_color.unwrap_or(palette.muted);
                let b_style = apply_bg(Style::new().with_fg(b_fg));
                let _ = buffer.set_string(
                    badge_x,
                    row,
                    self.badge,
                    b_style,
                    right_edge.saturating_sub(badge_x.get()),
                );
            }
        }

        if self.detail.is_empty() {
            return 1;
        }

        let detail_row = row.saturating_add(1);
        let detail_indent: u16 = 6;
        if width > detail_indent {
            let max_det = width - detail_indent;
            let (det_pre, det_ell, _) =
                truncate_with_ellipsis_parts(self.detail, TerminalWidth::new(max_det));
            let det_style = Style::new().with_fg(palette.muted);
            let mut dx = buffer.set_string(
                col.saturating_add(detail_indent),
                detail_row,
                det_pre,
                det_style,
                width - detail_indent,
            );
            if det_ell {
                let _ = buffer.set_string(
                    dx,
                    detail_row,
                    "…",
                    det_style,
                    right_edge.saturating_sub(dx.get()),
                );
            } else {
                let _ = &mut dx;
            }
        }
        2
    }
}
