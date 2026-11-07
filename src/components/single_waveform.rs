//! Single-direction sub-cell waveform visualizer (`SingleWaveform`).
//!
//! Renders an upward-growing (or downward-growing) waveform into a [`Rect`]
//! inside a [`Buffer`], using a [`GradientLut`] for vertical per-row coloring
//! (when `height > 1`) or cell-peak coloring (when `height == 1` sparkline mode).

use crate::components::graph_symbols::{
    graph_down_cell, graph_up_cell, row_wave_dots, total_wave_dots, GraphSymbolMode,
};
use crate::components::time_series::{resample_slice_into, resample_slice_scaled_into};
use crate::core::buffer::Buffer;
use crate::core::color::Color;
use crate::core::palette::GradientLut;
use crate::core::rect::Rect;
use crate::core::style::Style;

/// Maximum stack-allocated bucket count (`1024` buckets = `512` columns).
const MAX_STACK_BUCKETS: usize = 1024;

/// Single-direction sub-cell waveform renderer.
///
/// # Examples
///
/// ```
/// use tui_rs::components::{GraphSymbolMode, SingleWaveform};
/// use tui_rs::core::{Buffer, ColorPalette, Rect, TerminalHeight, TerminalWidth};
///
/// let area = Rect::from_size(TerminalWidth::new(20), TerminalHeight::new(4));
/// let mut buf = Buffer::empty(area);
/// let samples = [10.0, 40.0, 80.0, 100.0, 60.0];
/// let wave = SingleWaveform::new(&samples, 100.0)
///     .with_mode(GraphSymbolMode::Braille);
/// wave.render(&mut buf, area, &ColorPalette::DEFAULT.gradient_lut());
/// ```
#[derive(Debug, Clone, Copy)]
pub struct SingleWaveform<'a> {
    samples: &'a [f64],
    max_value: f64,
    mode: GraphSymbolMode,
    invert: bool,
    fill_width: bool,
    empty_color: Option<Color>,
    fixed_color: Option<Color>,
}

impl<'a> SingleWaveform<'a> {
    /// Creates a new [`SingleWaveform`] over `samples` with ceiling `max_value`.
    ///
    /// By default, `fill_width` is `true` so the waveform stretches or compresses
    /// to fill 100% of the target [`Rect`] width and adapts dynamically when the
    /// terminal window is resized.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::SingleWaveform;
    ///
    /// let wave = SingleWaveform::new(&[10.0, 20.0], 100.0);
    /// assert!(!wave.is_inverted());
    /// assert!(wave.is_fill_width());
    /// ```
    #[must_use]
    #[inline]
    pub const fn new(samples: &'a [f64], max_value: f64) -> Self {
        Self {
            samples,
            max_value: if max_value > 0.0 { max_value } else { 1.0 },
            mode: GraphSymbolMode::Braille,
            invert: false,
            fill_width: true,
            empty_color: None,
            fixed_color: None,
        }
    }

    /// Sets the glyph rendering mode (`Braille`, `Block`, or `Tty`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::{GraphSymbolMode, SingleWaveform};
    ///
    /// let wave = SingleWaveform::new(&[1.0], 10.0).with_mode(GraphSymbolMode::Block);
    /// assert_eq!(wave.mode(), GraphSymbolMode::Block);
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_mode(mut self, mode: GraphSymbolMode) -> Self {
        self.mode = mode;
        self
    }

    /// Configures whether the waveform hangs downward from the top row (`invert = true`)
    /// instead of growing upward from the bottom row (`invert = false`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::SingleWaveform;
    ///
    /// let wave = SingleWaveform::new(&[1.0], 10.0).with_inverted(true);
    /// assert!(wave.is_inverted());
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_inverted(mut self, invert: bool) -> Self {
        self.invert = invert;
        self
    }

    /// Overrides the gradient with a single fixed foreground color for active cells.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::SingleWaveform;
    /// use tui_rs::core::Color;
    ///
    /// let wave = SingleWaveform::new(&[1.0], 10.0).with_fixed_color(Color::Cyan);
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_fixed_color(mut self, color: Color) -> Self {
        self.fixed_color = Some(color);
        self
    }

    /// Sets an optional baseline dot color for empty baseline cells.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::SingleWaveform;
    /// use tui_rs::core::Color;
    ///
    /// let wave = SingleWaveform::new(&[1.0], 10.0).with_empty_color(Color::DarkGray);
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_empty_color(mut self, color: Color) -> Self {
        self.empty_color = Some(color);
        self
    }

    /// Returns the configured [`GraphSymbolMode`].
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::{GraphSymbolMode, SingleWaveform};
    ///
    /// assert_eq!(SingleWaveform::new(&[], 1.0).mode(), GraphSymbolMode::Braille);
    /// ```
    #[must_use]
    #[inline]
    pub const fn mode(&self) -> GraphSymbolMode {
        self.mode
    }

    /// Returns `true` if the waveform grows downward from the top row.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::SingleWaveform;
    ///
    /// assert!(!SingleWaveform::new(&[], 1.0).is_inverted());
    /// ```
    #[must_use]
    #[inline]
    pub const fn is_inverted(&self) -> bool {
        self.invert
    }

    /// Configures whether the sample slice should be stretched/compressed to fill
    /// 100% of the target [`Rect`] width (`true`, the default) or right-aligned
    /// without stretching when fewer samples than sub-columns exist (`false`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::SingleWaveform;
    ///
    /// let wave = SingleWaveform::new(&[10.0, 20.0], 100.0).with_fill_width(false);
    /// assert!(!wave.is_fill_width());
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_fill_width(mut self, fill_width: bool) -> Self {
        self.fill_width = fill_width;
        self
    }

    /// Returns `true` if the waveform stretches/compresses samples to fill 100%
    /// of the panel width.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::SingleWaveform;
    ///
    /// assert!(SingleWaveform::new(&[], 1.0).is_fill_width());
    /// ```
    #[must_use]
    #[inline]
    pub const fn is_fill_width(&self) -> bool {
        self.fill_width
    }

    /// Renders the waveform directly into `area` inside `buffer` with zero heap allocations
    /// for terminal widths up to `512` columns.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::SingleWaveform;
    /// use tui_rs::core::{Buffer, ColorPalette, Rect, TerminalHeight, TerminalWidth};
    ///
    /// let area = Rect::from_size(TerminalWidth::new(10), TerminalHeight::new(2));
    /// let mut buf = Buffer::empty(area);
    /// SingleWaveform::new(&[50.0, 100.0], 100.0)
    ///     .render(&mut buf, area, &ColorPalette::DEFAULT.gradient_lut());
    /// ```
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        reason = "row and col indices bounded by u16 Rect dimensions"
    )]
    pub fn render(&self, buffer: &mut Buffer, area: Rect, gradient: &GradientLut) {
        if area.is_empty() {
            return;
        }
        let cols = usize::from(area.width().get());
        let rows = usize::from(area.height().get());
        let num_buckets = (cols.saturating_mul(2)).min(MAX_STACK_BUCKETS);
        let effective_cols = num_buckets / 2;

        let mut stack_buckets = [0.0_f64; MAX_STACK_BUCKETS];
        let buckets = &mut stack_buckets[..num_buckets];
        if self.fill_width {
            resample_slice_scaled_into(self.samples, buckets);
        } else {
            resample_slice_into(self.samples, buckets);
        }

        for row_idx in 0..rows {
            // Distance from baseline: 0 is baseline row, rows-1 is peak row
            let row_from_base = if self.invert {
                row_idx
            } else {
                rows.saturating_sub(1).saturating_sub(row_idx)
            };

            let row_ratio = if rows <= 1 {
                0.5
            } else {
                (row_from_base as f32) / ((rows - 1) as f32)
            };
            let row_grad_color = self
                .fixed_color
                .unwrap_or_else(|| gradient.at_ratio(row_ratio));

            let screen_y = area.y().saturating_add(row_idx as u16);

            for col_idx in 0..effective_cols {
                let left_val = buckets[col_idx * 2];
                let right_val = buckets[col_idx * 2 + 1];

                let left_total = total_wave_dots(left_val, self.max_value, rows);
                let right_total = total_wave_dots(right_val, self.max_value, rows);

                let left_dots = row_wave_dots(left_total, row_from_base);
                let right_dots = row_wave_dots(right_total, row_from_base);

                let glyph = if self.invert {
                    graph_down_cell(self.mode, left_dots, right_dots)
                } else {
                    graph_up_cell(self.mode, left_dots, right_dots)
                };

                let screen_x = area.x().saturating_add(col_idx as u16);

                if left_dots == 0 && right_dots == 0 {
                    if row_from_base == 0 {
                        if let Some(empty_fg) = self.empty_color {
                            let baseline_glyph = match (self.mode, self.invert) {
                                (GraphSymbolMode::Braille, false) => "⣀",
                                (GraphSymbolMode::Braille, true) => "⠉",
                                _ => " ",
                            };
                            let _ = buffer.set_cell(
                                screen_x,
                                screen_y,
                                baseline_glyph,
                                Style::new().with_fg(empty_fg),
                            );
                        }
                    }
                    continue;
                }

                // In 1-row sparkline mode, color each cell by its own peak intensity
                let cell_color = if rows == 1 && self.fixed_color.is_none() {
                    let peak_dots = left_dots.max(right_dots);
                    gradient.at_ratio((peak_dots as f32) / 4.0)
                } else {
                    row_grad_color
                };

                let _ = buffer.set_cell(
                    screen_x,
                    screen_y,
                    glyph,
                    Style::new().with_fg(cell_color),
                );
            }
        }
    }
}
