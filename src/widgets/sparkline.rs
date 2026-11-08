//! Compact multi-row bar graph of recent values (`Sparkline`).
//!
//! Draws the **newest** samples (the tail of the slice) right-aligned, which is what a live
//! dashboard wants. Each column is built from eighth-block glyphs, giving `8 × height`
//! vertical levels.

use crate::core::buffer::Buffer;
use crate::core::rect::Rect;
use crate::core::style::Style;
use crate::core::widget::Widget;

const BLOCKS: [&str; 9] = [" ", "▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
const TTY_BLOCKS: [&str; 9] = [" ", ".", ".", ":", ":", "i", "i", "I", "#"];

/// A sparkline over a slice of values.
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Buffer, Rect, Widget};
/// use tui_rs::widgets::Sparkline;
///
/// let area = Rect::from_xywh(0, 0, 8, 1);
/// let mut buffer = Buffer::empty(area);
/// Sparkline::new(&[0.0, 1.0, 2.0, 4.0, 8.0]).render(area, &mut buffer);
/// assert_eq!(buffer.to_visual_lines(), vec!["    ▁▂▄█"]);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Sparkline<'a> {
    data: &'a [f64],
    max: Option<f64>,
    style: Style,
    tty_mode: bool,
}

impl<'a> Sparkline<'a> {
    /// Creates a sparkline over `data` (oldest first). Non-finite and negative values count as 0.
    #[must_use]
    pub const fn new(data: &'a [f64]) -> Self {
        Self {
            data,
            max: None,
            style: Style::DEFAULT,
            tty_mode: false,
        }
    }

    /// Fixes the value that fills the full height (default: the largest visible sample).
    #[must_use]
    pub const fn max(mut self, max: f64) -> Self {
        self.max = Some(max);
        self
    }

    /// Sets the bar style.
    #[must_use]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Uses ASCII glyphs for a Linux console.
    #[must_use]
    pub const fn tty_mode(mut self, tty_mode: bool) -> Self {
        self.tty_mode = tty_mode;
        self
    }
}

fn sanitize(value: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        0.0
    }
}

impl Widget for Sparkline<'_> {
    fn render(self, area: Rect, buffer: &mut Buffer) {
        if area.is_empty() || self.data.is_empty() {
            return;
        }
        let width = usize::from(area.width().get());
        let visible = &self.data[self.data.len().saturating_sub(width)..];
        let ceiling = self.max.map_or_else(
            || visible.iter().copied().map(sanitize).fold(0.0, f64::max),
            sanitize,
        );
        if ceiling <= 0.0 {
            return;
        }
        let rows = usize::from(area.height().get());
        let total_levels = u32::try_from(rows.saturating_mul(8)).unwrap_or(u32::MAX);
        let glyphs = if self.tty_mode { &TTY_BLOCKS } else { &BLOCKS };
        let first_column = area
            .x()
            .saturating_add(u16::try_from(width - visible.len()).unwrap_or(0));

        for (offset, &value) in visible.iter().enumerate() {
            let levels = usize::from(crate::widgets::round_to_u16(
                (sanitize(value) / ceiling) * f64::from(total_levels),
            ))
            .min(rows.saturating_mul(8));
            let column = first_column.saturating_add(u16::try_from(offset).unwrap_or(u16::MAX));
            for row_from_bottom in 0..rows {
                let row_levels = levels.saturating_sub(row_from_bottom * 8).min(8);
                if row_levels == 0 {
                    continue;
                }
                let row_index = u16::try_from(rows - 1 - row_from_bottom).unwrap_or(0);
                let _ = buffer.set_string(
                    column,
                    area.y().saturating_add(row_index),
                    glyphs[row_levels],
                    self.style,
                    1,
                );
            }
        }
    }
}
