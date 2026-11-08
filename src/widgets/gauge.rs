//! Progress indicators: block [`Gauge`] and single-line [`LineGauge`].

use crate::core::buffer::{Buffer, StackStr};
use crate::core::coordinates::ColumnIndex;
use crate::core::grapheme::display_width;
use crate::core::rect::{Rect, TerminalHeight, TerminalWidth};
use crate::core::style::Style;
use crate::core::widget::Widget;
use core::fmt::Write;

const EIGHTHS: [&str; 8] = [" ", "▏", "▎", "▍", "▌", "▋", "▊", "▉"];

/// A filled progress bar covering the whole area with a centered label.
///
/// `gauge_style` sets the filled part: give it a **background** color (the bar is drawn as
/// colored cells so the label stays readable on top).
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Buffer, Color, Rect, Style, Widget};
/// use tui_rs::widgets::Gauge;
///
/// let area = Rect::from_xywh(0, 0, 10, 1);
/// let mut buffer = Buffer::empty(area);
/// Gauge::default()
///     .ratio(0.5)
///     .gauge_style(Style::new().bg(Color::Green))
///     .render(area, &mut buffer);
/// // Default label is the percentage, centered on the bar.
/// assert_eq!(buffer.to_visual_lines(), vec!["   50%    "]);
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Gauge<'a> {
    ratio: f64,
    label: Option<&'a str>,
    style: Style,
    filled_style: Style,
    label_style: Style,
    use_unicode: bool,
}

impl<'a> Gauge<'a> {
    /// Sets the progress as a fraction in `0.0..=1.0` (clamped; `NaN` counts as 0).
    #[must_use]
    pub const fn ratio(mut self, ratio: f64) -> Self {
        self.ratio = if ratio.is_nan() {
            0.0
        } else {
            ratio.clamp(0.0, 1.0)
        };
        self
    }

    /// Sets the progress as a percentage in `0..=100`.
    #[must_use]
    pub fn percent(self, percent: u16) -> Self {
        self.ratio(f64::from(percent.min(100)) / 100.0)
    }

    /// Overrides the label (default: the rounded percentage).
    #[must_use]
    pub const fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// Sets the style behind the whole gauge (the unfilled part).
    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Sets the style of the filled part (use a background color).
    #[must_use]
    pub const fn gauge_style(mut self, style: Style) -> Self {
        self.filled_style = style;
        self
    }

    /// Sets the label style.
    #[must_use]
    pub const fn label_style(mut self, style: Style) -> Self {
        self.label_style = style;
        self
    }

    /// Uses eighth-block glyphs for a smooth leading edge (not for Linux consoles).
    #[must_use]
    pub const fn use_unicode(mut self, enabled: bool) -> Self {
        self.use_unicode = enabled;
        self
    }
}

impl Widget for Gauge<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        buffer.set_style(area, self.style);
        let width = f64::from(area.width().get());
        let filled_exact = self.ratio * width;
        let whole = filled_exact.floor();
        let partial = ((filled_exact - whole) * 8.0).floor();
        let whole_cells = crate::widgets::floor_to_u16(whole);

        for row in area.rows() {
            let fill = Rect::new(
                row.x(),
                row.y(),
                TerminalWidth::new(whole_cells.min(area.width().get())),
                TerminalHeight::new(1),
            );
            buffer.fill_rect(fill, ' ', self.filled_style);
            if self.use_unicode && whole_cells < area.width().get() {
                let eighths = usize::from(crate::widgets::floor_to_u16(partial));
                if eighths > 0 {
                    let edge_style = Style::new().patch(Style::DEFAULT).patch(
                        match self.filled_style.background() {
                            Some(color) => Style::new().fg(color),
                            None => Style::new(),
                        },
                    );
                    let _ = buffer.set_string(
                        row.x().saturating_add(whole_cells),
                        row.y(),
                        EIGHTHS[eighths.min(7)],
                        edge_style,
                        1,
                    );
                }
            }
        }

        let mut default_label = StackStr::<8>::new();
        let label = if let Some(text) = self.label {
            text
        } else {
            let _ = write!(
                default_label,
                "{}%",
                crate::widgets::round_to_u16(self.ratio * 100.0)
            );
            default_label.as_str()
        };
        let label_width = display_width(label).as_u16().min(area.width().get());
        let label_x = area
            .x()
            .saturating_add(area.width().get().saturating_sub(label_width) / 2);
        let label_y = area
            .y()
            .saturating_add(area.height().get().saturating_sub(1) / 2);
        let _ = buffer.set_string(label_x, label_y, label, self.label_style, label_width);
    }
}

/// A one-row progress line: an optional label followed by a thin bar.
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Buffer, Rect, Widget};
/// use tui_rs::widgets::LineGauge;
///
/// let area = Rect::from_xywh(0, 0, 16, 1);
/// let mut buffer = Buffer::empty(area);
/// LineGauge::default().ratio(0.25).render(area, &mut buffer);
/// assert_eq!(buffer.to_visual_lines(), vec![" 25% ━━━────────"]);
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LineGauge<'a> {
    ratio: f64,
    label: Option<&'a str>,
    style: Style,
    filled_style: Style,
    unfilled_style: Style,
    tty_mode: bool,
}

impl<'a> LineGauge<'a> {
    /// Sets the progress as a fraction in `0.0..=1.0` (clamped; `NaN` counts as 0).
    #[must_use]
    pub const fn ratio(mut self, ratio: f64) -> Self {
        self.ratio = if ratio.is_nan() {
            0.0
        } else {
            ratio.clamp(0.0, 1.0)
        };
        self
    }

    /// Overrides the label (default: the rounded percentage).
    #[must_use]
    pub const fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    /// Sets the base style (label and gaps).
    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Sets the style of the filled part of the line.
    #[must_use]
    pub const fn filled_style(mut self, style: Style) -> Self {
        self.filled_style = style;
        self
    }

    /// Sets the style of the unfilled part of the line.
    #[must_use]
    pub const fn unfilled_style(mut self, style: Style) -> Self {
        self.unfilled_style = style;
        self
    }

    /// Uses ASCII glyphs (`#`/`-`) for a Linux console.
    #[must_use]
    pub const fn tty_mode(mut self, tty_mode: bool) -> Self {
        self.tty_mode = tty_mode;
        self
    }
}

impl Widget for LineGauge<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        buffer.set_style(area, self.style);
        let mut default_label = StackStr::<8>::new();
        let label = if let Some(text) = self.label {
            text
        } else {
            let _ = write!(
                default_label,
                "{}%",
                crate::widgets::round_to_u16(self.ratio * 100.0)
            );
            default_label.as_str()
        };
        let row = area.y();
        let right_edge = area.right();
        let mut column = buffer.set_str_clipped(area.x(), row, " ", right_edge, self.style);
        column = buffer.set_str_clipped(column, row, label, right_edge, self.style);
        column = buffer.set_str_clipped(column, row, " ", right_edge, self.style);
        let line_width = right_edge.get().saturating_sub(column.get());
        let filled = crate::widgets::round_to_u16(self.ratio * f64::from(line_width));
        let (filled_glyph, unfilled_glyph) = if self.tty_mode {
            ("#", "-")
        } else {
            ("━", "─")
        };
        for offset in 0..line_width {
            let (glyph, style) = if offset < filled {
                (filled_glyph, self.filled_style)
            } else {
                (unfilled_glyph, self.unfilled_style)
            };
            let _ = buffer.set_string(
                ColumnIndex::new(column.get().saturating_add(offset)),
                row,
                glyph,
                style,
                1,
            );
        }
    }
}
