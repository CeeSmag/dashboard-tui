//! Rounded tab-notched panel chrome (`TabPanel`) with 4-step width degradation.
//!
//! Synthesizes `PanelBox` from `ref/dashboard/de_panel.go` and `Draw::box_create`
//! from `ref/btop-main/src/btop_draw.cpp`:
//! - Top border: `╭─┐ ⁰Title ┌──────┐ Badge ┌─╮`
//! - 4-step narrow width degradation:
//!   1. Full title + right badge
//!   2. Drop right badge if `title + badge` exceeds inner border width
//!   3. Truncate title with `…` (`truncate_with_ellipsis_parts`) if title alone exceeds width
//!   4. Plain `╭────╮` if `inner_width < 6`
//! - Bottom border: `╰─┘ subtitle └──────────────╯` (or plain `╰────╯`)
//! - Linux TTY fallback: automatically substitutes `┌ ┐ └ ┘` when `tty_mode` is active.

use crate::core::buffer::{Buffer, StackStr};
use crate::core::color::Color;
use crate::core::coordinates::{ColumnIndex, RowIndex, TerminalWidth};
use crate::core::grapheme::{display_width, truncate_with_ellipsis_parts};
use crate::core::palette::ColorPalette;
use crate::core::rect::{Rect, Spacing4D};
use crate::core::style::Style;

/// Superscript digit glyphs (`⁰..⁹`) used by `btop`-style panel shortcut badges.
const SUPERSCRIPT_DIGITS: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];

/// Formats a panel shortcut index (`1..=99`) into superscript digits (`"¹"`, `"¹²"`).
///
/// # Examples
///
/// ```
/// use tui_rs::widgets::format_superscript_number;
///
/// assert_eq!(format_superscript_number(1).as_str(), "¹");
/// assert_eq!(format_superscript_number(20).as_str(), "²⁰");
/// ```
#[must_use]
pub fn format_superscript_number(num: u8) -> StackStr<16> {
    let mut out = StackStr::<16>::new();
    let clamped = num.min(99);
    if clamped >= 10 {
        let tens = usize::from(clamped / 10);
        let _ = out.push_char(SUPERSCRIPT_DIGITS[tens]);
    }
    let ones = usize::from(clamped % 10);
    let _ = out.push_char(SUPERSCRIPT_DIGITS[ones]);
    out
}

/// Tab-notched bordered container widget (`╭─┐ Title ┌───╮`).
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Buffer, ColorPalette, Rect, TerminalHeight, TerminalWidth};
/// use tui_rs::widgets::TabPanel;
///
/// let area = Rect::from_size(TerminalWidth::new(40), TerminalHeight::new(6));
/// let mut buf = Buffer::empty(area);
/// let panel = TabPanel::new("Drive Status")
///     .with_badge("MOUNTED")
///     .with_subtitle("↑↓ navigate")
///     .with_focused(true);
/// let inner = panel.render(&mut buf, area, &ColorPalette::DEFAULT);
/// assert_eq!(inner.width().get(), 36);
/// assert_eq!(inner.height().get(), 4);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct TabPanel<'a> {
    title: &'a str,
    subtitle: &'a str,
    badge: &'a str,
    badge_color: Option<Color>,
    border_color_override: Option<Color>,
    superscript_number: Option<u8>,
    focused: bool,
    tty_mode: bool,
    horizontal_padding: u16,
}

impl<'a> TabPanel<'a> {
    /// Creates a new [`TabPanel`] with the given title.
    ///
    /// Defaults to 1 column of horizontal inner padding on each side (`│ content │`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::TabPanel;
    ///
    /// let panel = TabPanel::new("Telemetry");
    /// assert!(!panel.is_focused());
    /// ```
    #[must_use]
    #[inline]
    pub const fn new(title: &'a str) -> Self {
        Self {
            title,
            subtitle: "",
            badge: "",
            badge_color: None,
            border_color_override: None,
            superscript_number: None,
            focused: false,
            tty_mode: false,
            horizontal_padding: 1,
        }
    }

    /// Sets the bottom-border notch subtitle (`╰─┘ subtitle └───╯`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::TabPanel;
    ///
    /// let panel = TabPanel::new("Settings").with_subtitle("Enter edit");
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_subtitle(mut self, subtitle: &'a str) -> Self {
        self.subtitle = subtitle;
        self
    }

    /// Sets the right-aligned top-border notch badge (`┐ Badge ┌─╮`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::TabPanel;
    ///
    /// let panel = TabPanel::new("Sync").with_badge("IDLE");
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_badge(mut self, badge: &'a str) -> Self {
        self.badge = badge;
        self
    }

    /// Overrides the foreground color of the right-aligned badge.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::Color;
    /// use tui_rs::widgets::TabPanel;
    ///
    /// let panel = TabPanel::new("Sync").with_badge("ACTIVE").with_badge_color(Color::Green);
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_badge_color(mut self, color: Color) -> Self {
        self.badge_color = Some(color);
        self
    }

    /// Overrides the border color of the panel regardless of focus state.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::Color;
    /// use tui_rs::widgets::TabPanel;
    ///
    /// let panel = TabPanel::new("Alert").with_border_color(Color::Red);
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_border_color(mut self, color: Color) -> Self {
        self.border_color_override = Some(color);
        self
    }

    /// Attaches a `btop`-style superscript number (`⁰..⁹`) to the title notch.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::TabPanel;
    ///
    /// let panel = TabPanel::new("CPU").with_superscript_number(1);
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_superscript_number(mut self, number: u8) -> Self {
        self.superscript_number = Some(number);
        self
    }

    /// Configures whether the panel is currently focused (`border_focus` vs `border`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::TabPanel;
    ///
    /// let panel = TabPanel::new("Menu").with_focused(true);
    /// assert!(panel.is_focused());
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Configures whether square corners (`┌ ┐ └ ┘`) are used for Linux TTY consoles.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::TabPanel;
    ///
    /// let panel = TabPanel::new("TTY").with_tty_mode(true);
    /// assert!(panel.is_tty_mode());
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_tty_mode(mut self, tty_mode: bool) -> Self {
        self.tty_mode = tty_mode;
        self
    }

    /// Configures the horizontal padding inside the left and right vertical borders (default `1`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::TabPanel;
    ///
    /// let panel = TabPanel::new("Compact").with_horizontal_padding(0);
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_horizontal_padding(mut self, padding: u16) -> Self {
        self.horizontal_padding = padding;
        self
    }

    /// Returns `true` if the panel is marked as focused.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::TabPanel;
    ///
    /// assert!(!TabPanel::new("A").is_focused());
    /// ```
    #[must_use]
    #[inline]
    pub const fn is_focused(&self) -> bool {
        self.focused
    }

    /// Returns `true` if TTY square-corner mode is enabled.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::widgets::TabPanel;
    ///
    /// assert!(!TabPanel::new("A").is_tty_mode());
    /// ```
    #[must_use]
    #[inline]
    pub const fn is_tty_mode(&self) -> bool {
        self.tty_mode
    }

    /// Computes the inner content [`Rect`] inside the panel borders and horizontal padding.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Rect, TerminalHeight, TerminalWidth};
    /// use tui_rs::widgets::TabPanel;
    ///
    /// let area = Rect::from_size(TerminalWidth::new(30), TerminalHeight::new(10));
    /// let inner = TabPanel::new("Test").inner_area(area);
    /// assert_eq!(inner.width().get(), 26);
    /// assert_eq!(inner.height().get(), 8);
    /// ```
    #[must_use]
    #[inline]
    pub const fn inner_area(&self, area: Rect) -> Rect {
        let side = 1_u16.saturating_add(self.horizontal_padding);
        area.inner(Spacing4D::new(1, side, 1, side))
    }

    /// Renders the panel frame into `area` inside `buffer` and returns the inner
    /// content [`Rect`] for child widgets to draw into.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::core::{Buffer, ColorPalette, Rect, TerminalHeight, TerminalWidth};
    /// use tui_rs::widgets::TabPanel;
    ///
    /// let area = Rect::from_size(TerminalWidth::new(30), TerminalHeight::new(5));
    /// let mut buf = Buffer::empty(area);
    /// let inner = TabPanel::new("VaultDrive").render(&mut buf, area, &ColorPalette::DEFAULT);
    /// assert_eq!(inner.height().get(), 3);
    /// ```
    pub fn render(&self, buffer: &mut Buffer, area: Rect, palette: &ColorPalette) -> Rect {
        if area.width().get() < 2 || area.height().get() < 2 {
            return Rect::default();
        }

        let border_fg = self.border_color_override.unwrap_or(if self.focused {
            palette.border_focus
        } else {
            palette.border
        });
        let border_style = Style::new().with_fg(border_fg);
        let title_style = if self.focused {
            Style::new().with_fg(palette.accent2).bold()
        } else {
            Style::new().with_fg(palette.text).bold()
        };
        let badge_fg = self.badge_color.unwrap_or(palette.accent);
        let badge_style = Style::new().with_fg(badge_fg).bold();
        let subtitle_style = Style::new().with_fg(palette.muted);

        let (tl, tr, bl, br) = if self.tty_mode {
            ("┌", "┐", "└", "┘")
        } else {
            ("╭", "╮", "╰", "╯")
        };

        let x0 = area.x();
        let y0 = area.y();
        let x_right = area.right().saturating_sub(1);
        let y_bottom = area.bottom().saturating_sub(1);
        let inner_w = usize::from(area.width().get().saturating_sub(2));

        // 1. Top border
        self.render_top_border(
            buffer,
            x0,
            y0,
            inner_w,
            tl,
            tr,
            border_style,
            title_style,
            badge_style,
            palette,
        );

        // 2. Side vertical borders for middle rows
        for row_offset in 1..area.height().get().saturating_sub(1) {
            let ry = y0.saturating_add(row_offset);
            let _ = buffer.set_cell(x0, ry, "│", border_style);
            let _ = buffer.set_cell(x_right, ry, "│", border_style);
        }

        // 3. Bottom border
        self.render_bottom_border(
            buffer,
            x0,
            y_bottom,
            inner_w,
            bl,
            br,
            border_style,
            subtitle_style,
        );

        self.inner_area(area)
    }

    #[expect(
        clippy::too_many_arguments,
        clippy::cast_possible_truncation,
        reason = "internal border helper with coordinates bounded by u16"
    )]
    fn render_top_border(
        &self,
        buffer: &mut Buffer,
        x0: ColumnIndex,
        y0: RowIndex,
        inner_w: usize,
        tl: &str,
        tr: &str,
        border_style: Style,
        title_style: Style,
        badge_style: Style,
        palette: &ColorPalette,
    ) {
        let _ = buffer.set_cell(x0, y0, tl, border_style);
        let x_end = x0.saturating_add((inner_w + 1) as u16);
        let _ = buffer.set_cell(x_end, y0, tr, border_style);

        // Fill base horizontal rule '─' across inner_w first
        for i in 1..=inner_w {
            let _ = buffer.set_cell(x0.saturating_add(i as u16), y0, "─", border_style);
        }

        if self.title.is_empty() || inner_w < 6 {
            return;
        }

        let sup_str = if let Some(num) = self.superscript_number {
            if self.tty_mode {
                let mut s = StackStr::<16>::new();
                let _ = core::fmt::Write::write_fmt(&mut s, format_args!("{num}:"));
                s
            } else {
                format_superscript_number(num)
            }
        } else {
            StackStr::<16>::new()
        };
        let sup_w = display_width(sup_str.as_str()).as_usize();
        let raw_title_w = sup_w.saturating_add(display_width(self.title).as_usize());
        // "─┐ " (3) + title + " ┌" (2) = raw_title_w + 5
        let title_tab_w = raw_title_w.saturating_add(5);

        let badge_tab_w = if self.badge.is_empty() {
            0
        } else {
            // "┐ " (2) + badge + " ┌─" (3) = badge_w + 5
            display_width(self.badge).as_usize().saturating_add(5)
        };

        let show_badge = badge_tab_w > 0 && title_tab_w.saturating_add(badge_tab_w) < inner_w;

        let max_title_chars = if show_badge {
            inner_w.saturating_sub(badge_tab_w).saturating_sub(6)
        } else {
            inner_w.saturating_sub(6)
        };

        // Write left notch: at x0 + 1: "─┐ "
        let mut cx = x0.saturating_add(1);
        cx = buffer.set_string(cx, y0, "─┐ ", border_style, inner_w as u16);

        if !sup_str.is_empty() && sup_w <= max_title_chars {
            let sup_style = Style::new().with_fg(palette.accent).bold();
            cx = buffer.set_string(cx, y0, sup_str.as_str(), sup_style, sup_w as u16);
        }
        let avail_for_title = max_title_chars.saturating_sub(sup_w);
        let (prefix, has_ellipsis, _) =
            truncate_with_ellipsis_parts(self.title, TerminalWidth::new(avail_for_title as u16));
        cx = buffer.set_string(cx, y0, prefix, title_style, avail_for_title as u16);
        if has_ellipsis {
            cx = buffer.set_string(cx, y0, "…", title_style, 1);
        }
        let _ = buffer.set_string(cx, y0, " ┌", border_style, 2);

        if show_badge {
            let badge_start = x0.saturating_add((1 + inner_w.saturating_sub(badge_tab_w)) as u16);
            let mut bx = buffer.set_string(badge_start, y0, "┐ ", border_style, 2);
            bx = buffer.set_string(
                bx,
                y0,
                self.badge,
                badge_style,
                display_width(self.badge).as_u16(),
            );
            let _ = buffer.set_string(bx, y0, " ┌─", border_style, 3);
        }
    }

    #[expect(
        clippy::too_many_arguments,
        clippy::cast_possible_truncation,
        reason = "internal border helper with coordinates bounded by u16"
    )]
    fn render_bottom_border(
        &self,
        buffer: &mut Buffer,
        x0: ColumnIndex,
        y_bottom: RowIndex,
        inner_w: usize,
        bl: &str,
        br: &str,
        border_style: Style,
        subtitle_style: Style,
    ) {
        let _ = buffer.set_cell(x0, y_bottom, bl, border_style);
        let x_end = x0.saturating_add((inner_w + 1) as u16);
        let _ = buffer.set_cell(x_end, y_bottom, br, border_style);

        for i in 1..=inner_w {
            let _ = buffer.set_cell(x0.saturating_add(i as u16), y_bottom, "─", border_style);
        }

        if self.subtitle.is_empty() || inner_w < 8 {
            return;
        }
        let max_sub = inner_w.saturating_sub(6);
        let (sub_prefix, sub_ellipsis, _) =
            truncate_with_ellipsis_parts(self.subtitle, TerminalWidth::new(max_sub as u16));
        let mut cx = x0.saturating_add(1);
        cx = buffer.set_string(cx, y_bottom, "─┘ ", border_style, 3);
        cx = buffer.set_string(cx, y_bottom, sub_prefix, subtitle_style, max_sub as u16);
        if sub_ellipsis {
            cx = buffer.set_string(cx, y_bottom, "…", subtitle_style, 1);
        }
        let _ = buffer.set_string(cx, y_bottom, " └", border_style, 2);
    }
}
