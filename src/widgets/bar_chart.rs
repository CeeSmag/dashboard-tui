//! Vertical bar chart (`BarChart`) with labels and values.

use core::fmt::Write;

use crate::core::buffer::{Buffer, StackStr};
use crate::core::grapheme::display_width;
use crate::core::rect::Rect;
use crate::core::style::Style;
use crate::core::widget::Widget;

const BLOCKS: [&str; 9] = [" ", "▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
const TTY_BLOCKS: [&str; 9] = [" ", ".", ".", ":", ":", "i", "i", "I", "#"];

/// One bar of a [`BarChart`].
///
/// # Examples
///
/// ```
/// use tui_rs::widgets::Bar;
///
/// let bar = Bar::new("cpu", 70);
/// assert_eq!(bar.value(), 70);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Bar<'a> {
    label: &'a str,
    value: u64,
    style: Option<Style>,
}

impl<'a> Bar<'a> {
    /// Creates a bar with a `label` and a `value`.
    #[must_use]
    pub const fn new(label: &'a str, value: u64) -> Self {
        Self {
            label,
            value,
            style: None,
        }
    }

    /// Overrides the bar style for this bar only.
    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    /// Returns the bar's value.
    #[must_use]
    pub const fn value(&self) -> u64 {
        self.value
    }
}

/// A vertical bar chart: bars grow upward, labels sit on the bottom row.
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Buffer, Rect, Widget};
/// use tui_rs::widgets::{Bar, BarChart};
///
/// let area = Rect::from_xywh(0, 0, 9, 3);
/// let mut buffer = Buffer::empty(area);
/// BarChart::new([Bar::new("a", 4), Bar::new("b", 8)])
///     .bar_width(3)
///     .render(area, &mut buffer);
/// let lines = buffer.to_visual_lines();
/// assert_eq!(lines[2].trim_end(), " a   b");
/// assert!(lines[0].contains("███"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BarChart<'a> {
    bars: Vec<Bar<'a>>,
    bar_width: u16,
    bar_gap: u16,
    max: Option<u64>,
    style: Style,
    bar_style: Style,
    label_style: Style,
    value_style: Style,
    show_values: bool,
    tty_mode: bool,
}

impl<'a> BarChart<'a> {
    /// Creates a chart from bars.
    #[must_use]
    pub fn new(bars: impl IntoIterator<Item = Bar<'a>>) -> Self {
        Self {
            bars: bars.into_iter().collect(),
            bar_width: 3,
            bar_gap: 1,
            max: None,
            style: Style::DEFAULT,
            bar_style: Style::DEFAULT,
            label_style: Style::DEFAULT,
            value_style: Style::DEFAULT.reversed(),
            show_values: false,
            tty_mode: false,
        }
    }

    /// Sets the width of each bar in columns (minimum 1, default 3).
    #[must_use]
    pub const fn bar_width(mut self, width: u16) -> Self {
        self.bar_width = if width == 0 { 1 } else { width };
        self
    }

    /// Sets the gap between bars (default 1).
    #[must_use]
    pub const fn bar_gap(mut self, gap: u16) -> Self {
        self.bar_gap = gap;
        self
    }

    /// Fixes the value that fills the full height (default: the largest bar).
    #[must_use]
    pub const fn max(mut self, max: u64) -> Self {
        self.max = Some(max);
        self
    }

    /// Sets the style behind the whole chart.
    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Sets the default bar style.
    #[must_use]
    pub const fn bar_style(mut self, style: Style) -> Self {
        self.bar_style = style;
        self
    }

    /// Sets the label style.
    #[must_use]
    pub const fn label_style(mut self, style: Style) -> Self {
        self.label_style = style;
        self
    }

    /// Sets the style of the value text drawn on each bar (default: reversed).
    #[must_use]
    pub const fn value_style(mut self, style: Style) -> Self {
        self.value_style = style;
        self
    }

    /// Draws each bar's value at the bottom of the bar when it fits.
    #[must_use]
    pub const fn show_values(mut self, show: bool) -> Self {
        self.show_values = show;
        self
    }

    /// Uses ASCII glyphs for a Linux console.
    #[must_use]
    pub const fn tty_mode(mut self, tty_mode: bool) -> Self {
        self.tty_mode = tty_mode;
        self
    }
}

impl Widget for BarChart<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        if area.is_empty() || self.bars.is_empty() {
            return;
        }
        buffer.set_style(area, self.style);
        let has_labels = self.bars.iter().any(|bar| !bar.label.is_empty());
        let label_rows = u16::from(has_labels);
        let plot_height = area.height().get().saturating_sub(label_rows);
        let ceiling = self
            .max
            .unwrap_or_else(|| self.bars.iter().map(Bar::value).max().unwrap_or(0))
            .max(1);
        let glyphs = if self.tty_mode { &TTY_BLOCKS } else { &BLOCKS };
        let total_levels = u64::from(plot_height) * 8;
        let step = self.bar_width.saturating_add(self.bar_gap);
        let right_edge = area.right().get();

        for (index, bar) in self.bars.iter().enumerate() {
            let offset = u16::try_from(index)
                .unwrap_or(u16::MAX)
                .saturating_mul(step);
            let left = area.x().get().saturating_add(offset);
            if left >= right_edge {
                break;
            }
            let width = self.bar_width.min(right_edge - left);
            let style = bar.style.unwrap_or(self.bar_style);
            let levels = (bar.value.min(ceiling).saturating_mul(total_levels)) / ceiling;

            for row_from_bottom in 0..plot_height {
                let row_levels =
                    levels.saturating_sub(u64::from(row_from_bottom) * 8).min(8) as usize;
                if row_levels == 0 {
                    continue;
                }
                let row = area.y().saturating_add(plot_height - 1 - row_from_bottom);
                for column in 0..width {
                    let _ = buffer.set_string(
                        crate::core::coordinates::ColumnIndex::new(left + column),
                        row,
                        glyphs[row_levels],
                        style,
                        1,
                    );
                }
            }

            if self.show_values && plot_height > 0 {
                let mut text = StackStr::<24>::new();
                let _ = write!(text, "{}", bar.value);
                let text_width = display_width(text.as_str()).as_u16();
                if text_width <= width {
                    let value_x = left + (width - text_width) / 2;
                    let _ = buffer.set_string(
                        crate::core::coordinates::ColumnIndex::new(value_x),
                        area.y().saturating_add(plot_height - 1),
                        text.as_str(),
                        self.value_style,
                        text_width,
                    );
                }
            }

            if has_labels {
                let label_width = display_width(bar.label).as_u16().min(width);
                let label_x = left + (width - label_width) / 2;
                let _ = buffer.set_string(
                    crate::core::coordinates::ColumnIndex::new(label_x),
                    area.bottom().saturating_sub(1),
                    bar.label,
                    self.label_style,
                    label_width,
                );
            }
        }
    }
}
