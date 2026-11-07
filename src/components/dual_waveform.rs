//! Dual mirrored sub-cell waveform visualizer (`DualWaveform`).
//!
//! Ports and enhances `comp_dual_waveform.go` (`RenderDualWave`) and `btop`'s
//! mirrored network throughput graph:
//! - **Top half**: Grows **upward** from the center divider (e.g., Upload / Read).
//! - **Center axis row**: Responsive labeled divider
//!   `▲ TopLabel ─── [≤Ceiling] ─── ▼ BottomLabel` that degrades cleanly on
//!   narrow widths.
//! - **Bottom half**: Hangs **downward** from the center divider (e.g., Download / Write).
//! - **Extreme compact height fallback (`height == 1` or `height == 2`)**:
//!   Never panics; degrades to single/dual sparkline rows without the divider.

use crate::components::formatters::format_rate_short;
use crate::components::graph_symbols::GraphSymbolMode;
use crate::components::single_waveform::SingleWaveform;
use crate::core::buffer::Buffer;
use crate::core::color::Color;
use crate::core::grapheme::display_width;
use crate::core::palette::{ColorPalette, GradientLut};
use crate::core::rect::{Rect, TerminalHeight};
use crate::core::style::Style;

/// Dual mirrored sub-cell waveform component with a responsive center axis divider.
///
/// # Examples
///
/// ```
/// use tui_rs::components::DualWaveform;
/// use tui_rs::core::{Buffer, ColorPalette, Rect, TerminalHeight, TerminalWidth};
///
/// let area = Rect::from_size(TerminalWidth::new(50), TerminalHeight::new(7));
/// let mut buf = Buffer::empty(area);
/// let top = [100.0, 400.0, 900.0];
/// let bottom = [200.0, 600.0, 300.0];
/// let dual = DualWaveform::new(&top, &bottom, 1024.0)
///     .with_labels("Upload 900 B/s", "Download 300 B/s");
/// dual.render(&mut buf, area, &ColorPalette::DEFAULT);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct DualWaveform<'a> {
    top_samples: &'a [f64],
    bottom_samples: &'a [f64],
    scale_ceiling: f64,
    top_label: &'a str,
    bottom_label: &'a str,
    scale_label_override: Option<&'a str>,
    top_color: Option<Color>,
    bottom_color: Option<Color>,
    mode: GraphSymbolMode,
    fill_width: bool,
}

impl<'a> DualWaveform<'a> {
    /// Creates a new [`DualWaveform`] with top and bottom sample slices and a shared
    /// `scale_ceiling`.
    ///
    /// By default, `fill_width` is `true` so both top and bottom waveforms span 100%
    /// of the panel width and dynamically resize with the terminal window.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::DualWaveform;
    ///
    /// let wave = DualWaveform::new(&[10.0], &[20.0], 100.0);
    /// assert_eq!(wave.scale_ceiling(), 100.0);
    /// assert!(wave.is_fill_width());
    /// ```
    #[must_use]
    #[inline]
    pub const fn new(
        top_samples: &'a [f64],
        bottom_samples: &'a [f64],
        scale_ceiling: f64,
    ) -> Self {
        Self {
            top_samples,
            bottom_samples,
            scale_ceiling: if scale_ceiling > 0.0 {
                scale_ceiling
            } else {
                1.0
            },
            top_label: "",
            bottom_label: "",
            scale_label_override: None,
            top_color: None,
            bottom_color: None,
            mode: GraphSymbolMode::Braille,
            fill_width: true,
        }
    }

    /// Sets the left (`▲ top_label`) and right (`▼ bottom_label`) center divider labels.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::DualWaveform;
    ///
    /// let wave = DualWaveform::new(&[], &[], 100.0).with_labels("Up", "Down");
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_labels(mut self, top_label: &'a str, bottom_label: &'a str) -> Self {
        self.top_label = top_label;
        self.bottom_label = bottom_label;
        self
    }

    /// Sets an explicit center scale badge string (e.g., `"≤10M"`), overriding the
    /// automatic [`format_rate_short`] formatter.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::DualWaveform;
    ///
    /// let wave = DualWaveform::new(&[], &[], 100.0).with_scale_label("≤100 ops");
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_scale_label(mut self, label: &'a str) -> Self {
        self.scale_label_override = Some(label);
        self
    }

    /// Overrides the top and bottom waveform foreground colors.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::DualWaveform;
    /// use tui_rs::core::Color;
    ///
    /// let wave = DualWaveform::new(&[], &[], 100.0).with_colors(Color::Green, Color::Cyan);
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_colors(mut self, top_color: Color, bottom_color: Color) -> Self {
        self.top_color = Some(top_color);
        self.bottom_color = Some(bottom_color);
        self
    }

    /// Sets the [`GraphSymbolMode`] (`Braille`, `Block`, or `Tty`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::{DualWaveform, GraphSymbolMode};
    ///
    /// let wave = DualWaveform::new(&[], &[], 100.0).with_mode(GraphSymbolMode::Block);
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_mode(mut self, mode: GraphSymbolMode) -> Self {
        self.mode = mode;
        self
    }

    /// Configures whether the top and bottom sample slices stretch/compress to fill
    /// 100% of the panel width (`true`, the default) or right-align without stretching
    /// when fewer samples than sub-columns exist (`false`).
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::DualWaveform;
    ///
    /// let wave = DualWaveform::new(&[10.0], &[20.0], 100.0).with_fill_width(false);
    /// assert!(!wave.is_fill_width());
    /// ```
    #[must_use]
    #[inline]
    pub const fn with_fill_width(mut self, fill_width: bool) -> Self {
        self.fill_width = fill_width;
        self
    }

    /// Returns `true` if both waveforms stretch/compress to fill 100% of the panel width.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::DualWaveform;
    ///
    /// assert!(DualWaveform::new(&[], &[], 100.0).is_fill_width());
    /// ```
    #[must_use]
    #[inline]
    pub const fn is_fill_width(&self) -> bool {
        self.fill_width
    }

    /// Returns the shared scale ceiling.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::DualWaveform;
    ///
    /// assert_eq!(DualWaveform::new(&[], &[], 512.0).scale_ceiling(), 512.0);
    /// ```
    #[must_use]
    #[inline]
    pub const fn scale_ceiling(&self) -> f64 {
        self.scale_ceiling
    }

    /// Renders the dual mirrored waveform and its responsive center axis into `area`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tui_rs::components::DualWaveform;
    /// use tui_rs::core::{Buffer, ColorPalette, Rect, TerminalHeight, TerminalWidth};
    ///
    /// let area = Rect::from_size(TerminalWidth::new(40), TerminalHeight::new(5));
    /// let mut buf = Buffer::empty(area);
    /// DualWaveform::new(&[50.0], &[25.0], 100.0)
    ///     .with_labels("Up", "Down")
    ///     .render(&mut buf, area, &ColorPalette::DEFAULT);
    /// ```
    pub fn render(&self, buffer: &mut Buffer, area: Rect, palette: &ColorPalette) {
        if area.is_empty() {
            return;
        }
        let gradient = palette.gradient_lut();
        let h = area.height().get();

        // Compact height fallback: 1 row -> top wave only; 2 rows -> top & bottom without axis
        if h == 1 {
            self.render_half(buffer, area, gradient, palette, true);
            return;
        }
        if h == 2 {
            let top_area = Rect::new(area.x(), area.y(), area.width(), TerminalHeight::new(1));
            let bot_area = Rect::new(
                area.x(),
                area.y().saturating_add(1),
                area.width(),
                TerminalHeight::new(1),
            );
            self.render_half(buffer, top_area, gradient, palette, true);
            self.render_half(buffer, bot_area, gradient, palette, false);
            return;
        }

        let avail = h.saturating_sub(1);
        let top_rows = (avail / 2).max(1);
        let bot_rows = avail.saturating_sub(top_rows).max(1);

        let top_area = Rect::new(
            area.x(),
            area.y(),
            area.width(),
            TerminalHeight::new(top_rows),
        );
        let axis_y = area.y().saturating_add(top_rows);
        let bot_area = Rect::new(
            area.x(),
            axis_y.saturating_add(1),
            area.width(),
            TerminalHeight::new(bot_rows),
        );

        self.render_half(buffer, top_area, gradient, palette, true);
        self.render_axis_row(
            buffer,
            Rect::new(area.x(), axis_y, area.width(), TerminalHeight::new(1)),
            palette,
        );
        self.render_half(buffer, bot_area, gradient, palette, false);
    }

    fn render_half(
        &self,
        buffer: &mut Buffer,
        area: Rect,
        gradient: &GradientLut,
        palette: &ColorPalette,
        is_top: bool,
    ) {
        let samples = if is_top {
            self.top_samples
        } else {
            self.bottom_samples
        };
        let mut wave = SingleWaveform::new(samples, self.scale_ceiling)
            .with_mode(self.mode)
            .with_inverted(!is_top)
            .with_fill_width(self.fill_width)
            .with_empty_color(palette.dim);

        if is_top {
            if let Some(c) = self.top_color {
                wave = wave.with_fixed_color(c);
            }
        } else if let Some(c) = self.bottom_color {
            wave = wave.with_fixed_color(c);
        }
        wave.render(buffer, area, gradient);
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::too_many_lines,
        reason = "display widths are clamped to u16 area width; cohesive 3-zone axis renderer"
    )]
    fn render_axis_row(&self, buffer: &mut Buffer, row_area: Rect, palette: &ColorPalette) {
        let width = usize::from(row_area.width().get());
        if width == 0 {
            return;
        }
        let x0 = row_area.x();
        let y = row_area.y();
        let line_style = Style::new().with_fg(palette.dim);

        // Fill base horizontal rule '─' across the entire row first
        let rule_glyph = if self.mode == GraphSymbolMode::Tty {
            "-"
        } else {
            "─"
        };
        for offset in 0..row_area.width().get() {
            let _ = buffer.set_cell(x0.saturating_add(offset), y, rule_glyph, line_style);
        }

        let top_fg = self.top_color.unwrap_or(palette.accent);
        let bot_fg = self.bottom_color.unwrap_or(palette.info);
        let top_arrow = if self.mode == GraphSymbolMode::Tty {
            "^ "
        } else {
            "▲ "
        };
        let bot_arrow = if self.mode == GraphSymbolMode::Tty {
            "v "
        } else {
            "▼ "
        };

        let left_w = if self.top_label.is_empty() {
            0
        } else {
            display_width(top_arrow)
                .as_usize()
                .saturating_add(display_width(self.top_label).as_usize())
                .saturating_add(1)
        };
        let right_w = if self.bottom_label.is_empty() {
            0
        } else {
            display_width(bot_arrow)
                .as_usize()
                .saturating_add(display_width(self.bottom_label).as_usize())
                .saturating_add(1)
        };

        let formatted_scale = format_rate_short(self.scale_ceiling);
        let scale_text = self
            .scale_label_override
            .unwrap_or_else(|| formatted_scale.as_str());
        // "[≤256K]" width = scale_text width + 2
        let mid_w = display_width(scale_text).as_usize().saturating_add(2);

        // Full layout if left + mid + right + 2 dashes fit
        if left_w + right_w + mid_w + 2 <= width {
            if left_w > 0 {
                let mut cx = buffer.set_string(
                    x0,
                    y,
                    top_arrow,
                    Style::new().with_fg(top_fg).bold(),
                    row_area.width().get(),
                );
                cx = buffer.set_string(
                    cx,
                    y,
                    self.top_label,
                    Style::new().with_fg(top_fg).bold(),
                    row_area.right().get().saturating_sub(cx.get()),
                );
                let _ = buffer.set_cell(cx, y, " ", line_style);
            }

            if right_w > 0 {
                let right_start = x0.saturating_add((width.saturating_sub(right_w)) as u16);
                let _ = buffer.set_cell(right_start, y, " ", line_style);
                let cx = buffer.set_string(
                    right_start.saturating_add(1),
                    y,
                    bot_arrow,
                    Style::new().with_fg(bot_fg).bold(),
                    row_area.right().get().saturating_sub(right_start.get()),
                );
                let _ = buffer.set_string(
                    cx,
                    y,
                    self.bottom_label,
                    Style::new().with_fg(bot_fg).bold(),
                    row_area.right().get().saturating_sub(cx.get()),
                );
            }

            // Center the scale badge in the gap between left_w and (width - right_w)
            let gap = width.saturating_sub(left_w).saturating_sub(right_w);
            let left_dashes = gap.saturating_sub(mid_w) / 2;
            let mid_x = x0.saturating_add((left_w + left_dashes) as u16);
            let mut cx = buffer.set_string(mid_x, y, "[", line_style, 1);
            cx = buffer.set_string(
                cx,
                y,
                scale_text,
                Style::new().with_fg(palette.muted),
                row_area.right().get().saturating_sub(cx.get()),
            );
            let _ = buffer.set_string(cx, y, "]", line_style, 1);
        } else if left_w + right_w < width {
            // Omit center scale badge if narrow
            if left_w > 0 {
                let cx = buffer.set_string(
                    x0,
                    y,
                    top_arrow,
                    Style::new().with_fg(top_fg).bold(),
                    row_area.width().get(),
                );
                let _ = buffer.set_string(
                    cx,
                    y,
                    self.top_label,
                    Style::new().with_fg(top_fg).bold(),
                    row_area.right().get().saturating_sub(cx.get()),
                );
            }
            if right_w > 0 {
                let right_start = x0.saturating_add((width.saturating_sub(right_w)) as u16);
                let cx = buffer.set_string(
                    right_start.saturating_add(1),
                    y,
                    bot_arrow,
                    Style::new().with_fg(bot_fg).bold(),
                    row_area.right().get().saturating_sub(right_start.get()),
                );
                let _ = buffer.set_string(
                    cx,
                    y,
                    self.bottom_label,
                    Style::new().with_fg(bot_fg).bold(),
                    row_area.right().get().saturating_sub(cx.get()),
                );
            }
        }
    }
}
