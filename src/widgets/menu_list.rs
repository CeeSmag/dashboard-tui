//! Numbered menu list widget (`MenuList`) with 1-col / 2-col responsive layout
//! and 4-directional `section_option_gap` inset.
//!
//! Directly ports `MenuList` and `menuLine` from `ref/dashboard/de_widgets.go`:
//! - **4-directional inset**: When `option_gap > 0`, insets top/bottom by
//!   `option_gap` rows (capped so at least 2 item rows remain) and left/right
//!   by `option_gap * 2` columns (capped so at least 16 columns remain).
//! - **Automatic 2-column flow**: When `items.len() > inner_height && inner_width >= 72`,
//!   items flow in two balanced vertical columns separated by a 2-column gap.
//! - **Column-aligned `[n]` tags and badges**: Right-pads single-digit `[1]`
//!   tags when 2-digit `[10]` tags exist in the list, and aligns badges at a
//!   consistent column offset.
//! - **Centered scrolling window**: Keeps the active cursor centered in the
//!   visible viewport when the list exceeds available height.

use core::fmt::Write;

use crate::core::buffer::{Buffer, StackStr};
use crate::core::color::Color;
use crate::core::coordinates::{ColumnIndex, RowIndex, TerminalWidth};
use crate::core::grapheme::{display_width, truncate_with_ellipsis_parts};
use crate::core::palette::ColorPalette;
use crate::core::rect::{Rect, Spacing4D};
use crate::core::style::Style;
use crate::widgets::scrollbar::VerticalScrollState;

/// A single numbered menu option entry for [`MenuList`].
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::Color;
/// use dashboard_tui::widgets::MenuItem;
///
/// let item = MenuItem::new(1, "Mount Encrypted Vault")
///     .with_badge("MOUNTED")
///     .with_badge_color(Color::Green);
/// assert_eq!(item.number(), 1);
/// assert_eq!(item.label(), "Mount Encrypted Vault");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuItem<'a> {
    number: u16,
    label: &'a str,
    badge: &'a str,
    badge_color: Option<Color>,
    enabled: bool,
    hyperlink: Option<&'a str>,
}

impl<'a> MenuItem<'a> {
    /// Creates a new enabled [`MenuItem`] with shortcut `number` and `label`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::widgets::MenuItem;
    ///
    /// let item = MenuItem::new(2, "Start Background Sync");
    /// assert!(item.is_enabled());
    /// ```
    #[must_use]
    #[inline]
    pub const fn new(number: u16, label: &'a str) -> Self {
        Self {
            number,
            label,
            badge: "",
            badge_color: None,
            enabled: true,
            hyperlink: None,
        }
    }

    /// Attaches an optional OSC 8 clickable target URL to this menu item.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::widgets::MenuItem;
    ///
    /// let item = MenuItem::new(1, "Docs").with_hyperlink("https://github.com");
    /// assert_eq!(item.hyperlink(), Some("https://github.com"));
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_hyperlink(mut self, url: &'a str) -> Self {
        self.hyperlink = Some(url);
        self
    }

    /// Returns the attached hyperlink URL, if any.
    #[must_use]
    #[inline]
    pub const fn hyperlink(&self) -> Option<&'a str> {
        self.hyperlink
    }

    /// Sets the right-aligned status badge string.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::widgets::MenuItem;
    ///
    /// let item = MenuItem::new(1, "Theme").with_badge("Catppuccin");
    /// assert_eq!(item.badge(), "Catppuccin");
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
    /// use dashboard_tui::core::Color;
    /// use dashboard_tui::widgets::MenuItem;
    ///
    /// let item = MenuItem::new(1, "Status").with_badge("OK").with_badge_color(Color::Green);
    /// assert_eq!(item.badge_color(), Some(Color::Green));
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_badge_color(mut self, color: Color) -> Self {
        self.badge_color = Some(color);
        self
    }

    /// Configures whether the option is enabled (dimmed when `false`).
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::widgets::MenuItem;
    ///
    /// let item = MenuItem::new(1, "Locked").with_enabled(false);
    /// assert!(!item.is_enabled());
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Returns the numeric shortcut (`1..=99`).
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::widgets::MenuItem;
    ///
    /// assert_eq!(MenuItem::new(7, "Logs").number(), 7);
    /// ```
    #[must_use]
    #[inline]
    pub const fn number(&self) -> u16 {
        self.number
    }

    /// Returns the option label text.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::widgets::MenuItem;
    ///
    /// assert_eq!(MenuItem::new(7, "Logs").label(), "Logs");
    /// ```
    #[must_use]
    #[inline]
    pub const fn label(&self) -> &'a str {
        self.label
    }

    /// Returns the status badge text.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::widgets::MenuItem;
    ///
    /// assert_eq!(MenuItem::new(1, "A").with_badge("ON").badge(), "ON");
    /// ```
    #[must_use]
    #[inline]
    pub const fn badge(&self) -> &'a str {
        self.badge
    }

    /// Returns the optional badge color override.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::widgets::MenuItem;
    ///
    /// assert_eq!(MenuItem::new(1, "A").badge_color(), None);
    /// ```
    #[must_use]
    #[inline]
    pub const fn badge_color(&self) -> Option<Color> {
        self.badge_color
    }

    /// Returns `true` if the option is enabled.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::widgets::MenuItem;
    ///
    /// assert!(MenuItem::new(1, "A").is_enabled());
    /// ```
    #[must_use]
    #[inline]
    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// Numbered menu list widget supporting 4-directional `option_gap` inset and
/// automatic 2-column layout on wide panels (`inner_width >= 72`).
///
/// # Examples
///
/// ```
/// use dashboard_tui::core::{Buffer, ColorPalette, Rect, TerminalHeight, TerminalWidth};
/// use dashboard_tui::widgets::{MenuItem, MenuList};
///
/// let items = [
///     MenuItem::new(1, "Mount Encrypted Vault").with_badge("MOUNTED"),
///     MenuItem::new(2, "Start Background Sync").with_badge("IDLE"),
/// ];
/// let area = Rect::from_size(TerminalWidth::new(50), TerminalHeight::new(6));
/// let mut buf = Buffer::empty(area);
/// MenuList::new(&items)
///     .with_cursor(Some(0))
///     .with_option_gap(1)
///     .render(&mut buf, area, &ColorPalette::DEFAULT);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct MenuList<'a> {
    items: &'a [MenuItem<'a>],
    cursor: Option<usize>,
    option_gap: u16,
    tty_mode: bool,
}

impl<'a> MenuList<'a> {
    /// Creates a new [`MenuList`] over `items`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::widgets::MenuList;
    ///
    /// let list = MenuList::new(&[]);
    /// assert_eq!(list.option_gap(), 0);
    /// ```
    #[must_use]
    #[inline]
    pub const fn new(items: &'a [MenuItem<'a>]) -> Self {
        Self {
            items,
            cursor: None,
            option_gap: 0,
            tty_mode: false,
        }
    }

    /// Sets the active cursor index (`None` when the section is unfocused).
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::widgets::MenuList;
    ///
    /// let list = MenuList::new(&[]).with_cursor(Some(2));
    /// assert_eq!(list.cursor(), Some(2));
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_cursor(mut self, cursor: Option<usize>) -> Self {
        self.cursor = cursor;
        self
    }

    /// Sets the 4-directional `section_option_gap` inset (`0..=6`).
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::widgets::MenuList;
    ///
    /// let list = MenuList::new(&[]).with_option_gap(2);
    /// assert_eq!(list.option_gap(), 2);
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_option_gap(mut self, gap: u16) -> Self {
        self.option_gap = if gap > 6 { 6 } else { gap };
        self
    }

    /// Configures whether ASCII `'>'` is used instead of `'❯'` for the selection cursor.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::widgets::MenuList;
    ///
    /// let list = MenuList::new(&[]).with_tty_mode(true);
    /// assert!(list.is_tty_mode());
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_tty_mode(mut self, tty_mode: bool) -> Self {
        self.tty_mode = tty_mode;
        self
    }

    /// Returns the active cursor index, if any.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::widgets::MenuList;
    ///
    /// assert_eq!(MenuList::new(&[]).cursor(), None);
    /// ```
    #[must_use]
    #[inline]
    pub const fn cursor(&self) -> Option<usize> {
        self.cursor
    }

    /// Returns the configured 4-directional option gap (`0..=6`).
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::widgets::MenuList;
    ///
    /// assert_eq!(MenuList::new(&[]).option_gap(), 0);
    /// ```
    #[must_use]
    #[inline]
    pub const fn option_gap(&self) -> u16 {
        self.option_gap
    }

    /// Returns `true` if TTY mode is active.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::widgets::MenuList;
    ///
    /// assert!(!MenuList::new(&[]).is_tty_mode());
    /// ```
    #[must_use]
    #[inline]
    pub const fn is_tty_mode(&self) -> bool {
        self.tty_mode
    }

    /// Computes the inset [`Rect`] after applying the 4-directional `option_gap`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::{Rect, TerminalHeight, TerminalWidth};
    /// use dashboard_tui::widgets::MenuList;
    ///
    /// let area = Rect::from_size(TerminalWidth::new(50), TerminalHeight::new(10));
    /// let inset = MenuList::new(&[]).with_option_gap(1).inset_area(area);
    /// assert_eq!(inset.width().get(), 46);
    /// assert_eq!(inset.height().get(), 8);
    /// ```
    #[must_use]
    pub fn inset_area(&self, area: Rect) -> Rect {
        if self.option_gap == 0 || area.is_empty() {
            return area;
        }
        let h = area.height().get();
        let w = area.width().get();

        // Top & bottom padding (keep at least 2 rows for items)
        let mut v_pad = self.option_gap;
        if h.saturating_sub(v_pad.saturating_mul(2)) < 2 {
            v_pad = h.saturating_sub(2) / 2;
        }

        // Left & right padding (keep at least 16 cols for items)
        let mut h_pad = self.option_gap.saturating_mul(2);
        if w.saturating_sub(h_pad.saturating_mul(2)) < 16 {
            h_pad = w.saturating_sub(16) / 2;
        }

        area.inner(Spacing4D::new(v_pad, h_pad, v_pad, h_pad))
    }

    /// Renders the numbered menu list into `area` inside `buffer`.
    ///
    /// # Examples
    ///
    /// ```
    /// use dashboard_tui::core::{Buffer, ColorPalette, Rect, TerminalHeight, TerminalWidth};
    /// use dashboard_tui::widgets::{MenuItem, MenuList};
    ///
    /// let items = [MenuItem::new(1, "Configure Remote").with_badge("rclone")];
    /// let area = Rect::from_size(TerminalWidth::new(40), TerminalHeight::new(4));
    /// let mut buf = Buffer::empty(area);
    /// MenuList::new(&items).with_cursor(Some(0)).render(&mut buf, area, &ColorPalette::DEFAULT);
    /// assert!(buf.to_visual_lines()[0].contains("[1]"));
    /// ```
    #[expect(clippy::cast_possible_truncation, reason = "row/col counts fit in u16")]
    pub fn render(&self, buffer: &mut Buffer, area: Rect, palette: &ColorPalette) {
        if self.items.is_empty() || area.is_empty() {
            return;
        }
        let inner = self.inset_area(area);
        if inner.is_empty() {
            return;
        }

        let inner_w = inner.width().get();
        let inner_h = usize::from(inner.height().get());
        let n = self.items.len();

        let (num_w, max_label_w) = self.measure_alignment();

        // Automatic 2-column layout when items exceed height and inner_w >= 72
        if n > inner_h && inner_w >= 72 {
            let col_w = inner_w.saturating_sub(2) / 2;
            let rows_needed = n.div_ceil(2);
            let visible_rows = rows_needed.min(inner_h);

            let cursor_row = self.cursor.map_or(0, |c| c % rows_needed);
            let mut scroll = VerticalScrollState::new();
            let start_row = scroll.center_on(cursor_row, rows_needed, visible_rows);

            for r in 0..visible_rows {
                let row_idx = start_row + r;
                let ry = inner.y().saturating_add(r as u16);

                let left_idx = row_idx;
                let right_idx = row_idx + rows_needed;

                if left_idx < n {
                    self.render_item_line(
                        buffer,
                        inner.x(),
                        ry,
                        col_w,
                        &self.items[left_idx],
                        self.cursor == Some(left_idx),
                        num_w,
                        max_label_w,
                        palette,
                    );
                }
                if right_idx < n {
                    let right_x = inner.x().saturating_add(col_w).saturating_add(2);
                    self.render_item_line(
                        buffer,
                        right_x,
                        ry,
                        col_w,
                        &self.items[right_idx],
                        self.cursor == Some(right_idx),
                        num_w,
                        max_label_w,
                        palette,
                    );
                }
            }
            return;
        }

        // Single-column layout with centered scrolling window
        let visible_rows = n.min(inner_h);
        let mut scroll = VerticalScrollState::new();
        let start_idx = scroll.center_on(self.cursor.unwrap_or(0), n, visible_rows);

        for r in 0..visible_rows {
            let idx = start_idx + r;
            let ry = inner.y().saturating_add(r as u16);
            self.render_item_line(
                buffer,
                inner.x(),
                ry,
                inner_w,
                &self.items[idx],
                self.cursor == Some(idx),
                num_w,
                max_label_w,
                palette,
            );
        }
    }

    fn measure_alignment(&self) -> (usize, usize) {
        let mut max_num_w = 0_usize;
        let mut max_label_w = 0_usize;
        for item in self.items {
            let mut tag = StackStr::<12>::new();
            let _ = write!(tag, "[{}]", item.number);
            if tag.len() > max_num_w {
                max_num_w = tag.len();
            }
            let lw = display_width(item.label).as_usize();
            if lw > max_label_w {
                max_label_w = lw;
            }
        }
        (max_num_w, max_label_w)
    }

    #[expect(
        clippy::too_many_arguments,
        clippy::too_many_lines,
        clippy::cast_possible_truncation,
        reason = "internal row renderer with bounded u16 widths"
    )]
    fn render_item_line(
        &self,
        buffer: &mut Buffer,
        col: ColumnIndex,
        row: RowIndex,
        width: u16,
        item: &MenuItem<'a>,
        selected: bool,
        num_w: usize,
        max_label_w: usize,
        palette: &ColorPalette,
    ) {
        if width < 6 {
            return;
        }
        let width_usize = usize::from(width);
        let right_edge = col.get().saturating_add(width);

        // If selected, fill row background with highlight_bg
        if selected {
            let bg_style = Style::new().with_bg(palette.highlight_bg);
            for offset in 0..width {
                let _ = buffer.set_cell(col.saturating_add(offset), row, " ", bg_style);
            }
        }

        let mut num_tag = StackStr::<16>::new();
        let _ = write!(num_tag, "[{}]", item.number);
        while num_tag.len() < num_w {
            let _ = num_tag.push_char(' ');
        }

        // Prefix: "❯ [1] " -> 2 + num_w + 1
        let prefix_w = 2 + num_w + 1;
        let badge_w = display_width(item.badge).as_usize();

        // Align badges at a consistent column after the widest label (up to 55% of width)
        let mut target_label_w = max_label_w.saturating_add(2);
        let max_allowed_label = width_usize
            .saturating_sub(prefix_w)
            .saturating_sub(badge_w)
            .saturating_sub(2);
        if target_label_w > max_allowed_label {
            target_label_w = max_allowed_label;
        }
        let fifty_five_pct = (width_usize * 55) / 100;
        if badge_w > 0 && target_label_w > fifty_five_pct && fifty_five_pct >= 8 {
            target_label_w = fifty_five_pct;
        }
        if target_label_w < 4 {
            target_label_w = width_usize.saturating_sub(prefix_w).max(1);
        }

        let (label_prefix, label_has_ellipsis, label_tw) =
            truncate_with_ellipsis_parts(item.label, TerminalWidth::new(target_label_w as u16));
        let label_cols = label_tw.as_usize();
        let pad_after_label = target_label_w.saturating_sub(label_cols).max(1);

        let cursor_glyph = if selected {
            if self.tty_mode { "> " } else { "❯ " }
        } else {
            "  "
        };

        let base_bg = if selected {
            Some(palette.highlight_bg)
        } else {
            None
        };
        let apply_bg = |s: Style| match base_bg {
            Some(bg) => s.with_bg(bg),
            None => s,
        };

        let cursor_style = apply_bg(Style::new().with_fg(palette.accent2).bold());
        let num_style = if !item.enabled {
            apply_bg(Style::new().with_fg(palette.dim))
        } else if selected {
            apply_bg(Style::new().with_fg(palette.accent2).bold())
        } else {
            apply_bg(Style::new().with_fg(palette.accent).bold())
        };
        let label_style = if !item.enabled {
            apply_bg(Style::new().with_fg(palette.dim))
        } else if selected {
            apply_bg(Style::new().with_fg(palette.accent2).bold())
        } else {
            apply_bg(Style::new().with_fg(palette.text))
        };

        let mut cx = buffer.set_string(col, row, cursor_glyph, cursor_style, 2);
        cx = buffer.set_string(
            cx,
            row,
            num_tag.as_str(),
            num_style,
            right_edge.saturating_sub(cx.get()),
        );
        cx = buffer.set_string(
            cx,
            row,
            " ",
            label_style,
            right_edge.saturating_sub(cx.get()),
        );
        if let Some(url) = item.hyperlink {
            let label_col = cx;
            cx = buffer.set_hyperlink(
                label_col,
                row,
                url,
                label_prefix,
                label_style.underlined(),
                right_edge.saturating_sub(cx.get()),
            );
        } else {
            cx = buffer.set_string(
                cx,
                row,
                label_prefix,
                label_style,
                right_edge.saturating_sub(cx.get()),
            );
        }
        if label_has_ellipsis {
            cx = buffer.set_string(
                cx,
                row,
                "…",
                label_style,
                right_edge.saturating_sub(cx.get()),
            );
        }

        if badge_w > 0 {
            cx = cx.saturating_add(pad_after_label as u16);
            if cx.get() < right_edge {
                let avail_badge = right_edge.saturating_sub(cx.get());
                let (b_pre, b_ell, _) =
                    truncate_with_ellipsis_parts(item.badge, TerminalWidth::new(avail_badge));
                let badge_fg = item.badge_color.unwrap_or(palette.muted);
                let badge_style = if item.enabled {
                    apply_bg(Style::new().with_fg(badge_fg))
                } else {
                    apply_bg(Style::new().with_fg(palette.dim))
                };
                cx = buffer.set_string(
                    cx,
                    row,
                    b_pre,
                    badge_style,
                    right_edge.saturating_sub(cx.get()),
                );
                if b_ell {
                    let _ = buffer.set_string(
                        cx,
                        row,
                        "…",
                        badge_style,
                        right_edge.saturating_sub(cx.get()),
                    );
                }
            }
        }
    }
}
