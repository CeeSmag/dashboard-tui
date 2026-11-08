//! Sub-cell dot plotting shared by [`Canvas`](crate::widgets::Canvas) and
//! [`Chart`](crate::widgets::Chart).
//!
//! A [`DotPlotter`] maps a grid of "dots" onto the cells of a [`Rect`]. Braille markers give
//! 2×4 dots per cell, half blocks 1×2, blocks 1×1. Dots are OR-ed into the glyph already
//! stored in the buffer, so plotting needs no scratch memory. Lines are clipped to the plot
//! area before they are traced, so absurd coordinates cost nothing.

use crate::core::buffer::Buffer;
use crate::core::coordinates::{ColumnIndex, RowIndex};
use crate::core::rect::Rect;
use crate::core::style::Style;

/// Glyph family used to draw dots.
///
/// # Examples
///
/// ```
/// use tui_rs::widgets::Marker;
/// assert_eq!(Marker::default(), Marker::Braille);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Marker {
    /// Braille patterns: 2×4 dots per cell (highest resolution).
    #[default]
    Braille,
    /// Half blocks `▀ ▄ █`: 1×2 dots per cell.
    HalfBlock,
    /// Full blocks `█`: 1×1 dot per cell.
    Block,
    /// ASCII `#`: 1×1 dot per cell (Linux console safe).
    Ascii,
}

impl Marker {
    /// Dots per cell as `(columns, rows)`.
    #[must_use]
    pub const fn dots_per_cell(self) -> (u16, u16) {
        match self {
            Self::Braille => (2, 4),
            Self::HalfBlock => (1, 2),
            Self::Block | Self::Ascii => (1, 1),
        }
    }
}

const BRAILLE_BASE: u32 = 0x2800;
// Bit for the braille dot at (column, row) inside a cell.
const BRAILLE_BITS: [[u8; 4]; 2] = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];

fn braille_mask(symbol: &str) -> u8 {
    let mut chars = symbol.chars();
    match (chars.next(), chars.next()) {
        (Some(ch), None) if (BRAILLE_BASE..BRAILLE_BASE + 0x100).contains(&u32::from(ch)) => {
            u8::try_from(u32::from(ch) - BRAILLE_BASE).unwrap_or(0)
        }
        _ => 0,
    }
}

fn half_block_mask(symbol: &str) -> u8 {
    match symbol {
        "▀" => 0b01,
        "▄" => 0b10,
        "█" => 0b11,
        _ => 0,
    }
}

/// Maps dots to cells inside `area` and writes them into a [`Buffer`].
///
/// # Examples
///
/// ```
/// use tui_rs::core::{Buffer, Rect, Style};
/// use tui_rs::widgets::{DotPlotter, Marker};
///
/// let area = Rect::from_xywh(0, 0, 2, 1);
/// let mut buffer = Buffer::empty(area);
/// let plotter = DotPlotter::new(area, Marker::Braille);
/// assert_eq!((plotter.dots_wide(), plotter.dots_high()), (4, 4));
/// plotter.set_dot(&mut buffer, 0, 0, Style::DEFAULT);
/// plotter.set_dot(&mut buffer, 0, 3, Style::DEFAULT);
/// assert_eq!(buffer.to_visual_lines(), vec!["⡁ "]);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DotPlotter {
    area: Rect,
    marker: Marker,
}

impl DotPlotter {
    /// Creates a plotter drawing into `area` with `marker` glyphs.
    #[must_use]
    pub const fn new(area: Rect, marker: Marker) -> Self {
        Self { area, marker }
    }

    /// Total dot columns across the area.
    #[must_use]
    pub fn dots_wide(&self) -> u32 {
        u32::from(self.area.width().get()) * u32::from(self.marker.dots_per_cell().0)
    }

    /// Total dot rows down the area.
    #[must_use]
    pub fn dots_high(&self) -> u32 {
        u32::from(self.area.height().get()) * u32::from(self.marker.dots_per_cell().1)
    }

    /// Lights the dot at dot-grid position `(dot_x, dot_y)` (origin top-left). Out-of-range
    /// positions are ignored.
    pub fn set_dot(&self, buffer: &mut Buffer, dot_x: i64, dot_y: i64, style: Style) {
        let (per_cell_x, per_cell_y) = self.marker.dots_per_cell();
        let (Ok(dot_x), Ok(dot_y)) = (u32::try_from(dot_x), u32::try_from(dot_y)) else {
            return;
        };
        if dot_x >= self.dots_wide() || dot_y >= self.dots_high() {
            return;
        }
        let cell_x = dot_x / u32::from(per_cell_x);
        let cell_y = dot_y / u32::from(per_cell_y);
        let in_x = usize::try_from(dot_x % u32::from(per_cell_x)).unwrap_or(0);
        let in_y = usize::try_from(dot_y % u32::from(per_cell_y)).unwrap_or(0);
        let column = ColumnIndex::new(
            self.area
                .x()
                .get()
                .saturating_add(u16::try_from(cell_x).unwrap_or(u16::MAX)),
        );
        let row = RowIndex::new(
            self.area
                .y()
                .get()
                .saturating_add(u16::try_from(cell_y).unwrap_or(u16::MAX)),
        );
        let (existing_braille, existing_half) =
            buffer.cell_at(column, row).map_or((0, 0), |cell| {
                (braille_mask(cell.symbol()), half_block_mask(cell.symbol()))
            });

        let mut scratch = [0_u8; 4];
        let glyph: &str = match self.marker {
            Marker::Braille => {
                let mask = existing_braille | BRAILLE_BITS[in_x][in_y];
                char::from_u32(BRAILLE_BASE + u32::from(mask))
                    .unwrap_or('⣿')
                    .encode_utf8(&mut scratch)
            }
            Marker::HalfBlock => {
                let mask = existing_half | (1 << in_y);
                match mask {
                    0b01 => "▀",
                    0b10 => "▄",
                    _ => "█",
                }
            }
            Marker::Block => "█",
            Marker::Ascii => "#",
        };
        let _ = buffer.set_string(column, row, glyph, style, 1);
    }

    /// Draws a line between two dot-grid points (floating point), clipped to the area.
    pub fn line(&self, buffer: &mut Buffer, from: (f64, f64), to: (f64, f64), style: Style) {
        let width = f64::from(self.dots_wide());
        let height = f64::from(self.dots_high());
        if width <= 0.0 || height <= 0.0 {
            return;
        }
        let Some((start, end)) = clip_line(from, to, width - 1.0, height - 1.0) else {
            return;
        };
        let (mut x0, mut y0) = (round_dot(start.0), round_dot(start.1));
        let (x1, y1) = (round_dot(end.0), round_dot(end.1));
        let delta_x = (x1 - x0).abs();
        let delta_y = -(y1 - y0).abs();
        let step_x: i64 = if x0 < x1 { 1 } else { -1 };
        let step_y: i64 = if y0 < y1 { 1 } else { -1 };
        let mut error = delta_x + delta_y;
        loop {
            self.set_dot(buffer, x0, y0, style);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let doubled = 2 * error;
            if doubled >= delta_y {
                error += delta_y;
                x0 += step_x;
            }
            if doubled <= delta_x {
                error += delta_x;
                y0 += step_y;
            }
        }
    }
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "value is clamped to a range far inside i64 first; NaN maps to 0"
)]
fn round_dot(value: f64) -> i64 {
    if value.is_nan() {
        0
    } else {
        value.clamp(-1.0e12, 1.0e12).round() as i64
    }
}

/// Rounds a dot-grid coordinate to an integer (clamped, NaN → 0).
pub(crate) fn round_to_i64(value: f64) -> i64 {
    round_dot(value)
}

/// Liang–Barsky clip of the segment `from -> to` to the box `0..=max_x` × `0..=max_y`.
fn clip_line(
    from: (f64, f64),
    to: (f64, f64),
    max_x: f64,
    max_y: f64,
) -> Option<((f64, f64), (f64, f64))> {
    if !(from.0.is_finite() && from.1.is_finite() && to.0.is_finite() && to.1.is_finite()) {
        return None;
    }
    let (delta_x, delta_y) = (to.0 - from.0, to.1 - from.1);
    let mut entering = 0.0_f64;
    let mut leaving = 1.0_f64;
    for (direction, distance) in [
        (-delta_x, from.0),
        (delta_x, max_x - from.0),
        (-delta_y, from.1),
        (delta_y, max_y - from.1),
    ] {
        if direction == 0.0 {
            if distance < 0.0 {
                return None;
            }
        } else {
            let ratio = distance / direction;
            if direction < 0.0 {
                entering = entering.max(ratio);
            } else {
                leaving = leaving.min(ratio);
            }
            if entering > leaving {
                return None;
            }
        }
    }
    Some((
        (from.0 + entering * delta_x, from.1 + entering * delta_y),
        (from.0 + leaving * delta_x, from.1 + leaving * delta_y),
    ))
}

/// Maps `value` from `bounds` onto `0.0..=span` (inverted when `invert`), or `None` if the
/// value or bounds are not finite / the bounds are empty.
#[must_use]
pub(crate) fn scale_to_dots(value: f64, bounds: [f64; 2], span: f64, invert: bool) -> Option<f64> {
    let range = bounds[1] - bounds[0];
    if !value.is_finite() || !range.is_finite() || range <= 0.0 {
        return None;
    }
    let unit = (value - bounds[0]) / range;
    let along = unit * span;
    Some(if invert { span - along } else { along })
}
